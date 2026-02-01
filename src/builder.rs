use anyhow::{Context, Result};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use walkdir::WalkDir;

use crate::aapt2::Aapt2;
use crate::aar::AarExtractor;
use crate::cache::BuildCache;
use crate::types::{AarInfo, BuildConfig, BuildResult, CompileResult};

/// Main builder for Android skin packages
pub struct SkinBuilder {
    config: BuildConfig,
    aapt2: Aapt2,
    cache: Option<BuildCache>,
}

impl SkinBuilder {
    /// Create a new SkinBuilder
    pub fn new(config: BuildConfig) -> Result<Self> {
        let aapt2 = Aapt2::new(config.aapt2_path.clone())?;

        let cache = if config.incremental.unwrap_or(false) {
            let cache_dir = config
                .cache_dir
                .clone()
                .unwrap_or_else(|| config.output_dir.join(".build-cache"));
            let cache = BuildCache::new(cache_dir)?;
            cache.init()?;
            Some(cache)
        } else {
            None
        };

        Ok(Self {
            config,
            aapt2,
            cache,
        })
    }

    /// Build the skin package
    pub async fn build(&mut self) -> Result<BuildResult> {
        info!("Starting build for package: {}", self.config.package_name);

        // Ensure output directories exist
        let compiled_dir = self
            .config
            .compiled_dir
            .clone()
            .unwrap_or_else(|| self.config.output_dir.join("compiled"));
        std::fs::create_dir_all(&compiled_dir)?;
        std::fs::create_dir_all(&self.config.output_dir)?;

        // Extract AAR files if provided
        let mut aar_infos = Vec::new();
        let temp_dir = self.config.output_dir.join(".temp");

        if let Some(aar_files) = &self.config.aar_files {
            if !aar_files.is_empty() {
                info!("Extracting {} AAR files...", aar_files.len());
                aar_infos = AarExtractor::extract_aars(aar_files, &temp_dir)?;
            }
        }

        // Collect all resource directories
        let mut resource_dirs = vec![self.config.resource_dir.clone()];

        // Add AAR resource directories
        for aar_info in &aar_infos {
            if let Some(res_dir) = &aar_info.resource_dir {
                resource_dirs.push(res_dir.clone());
            }
        }

        // Add additional resource directories
        if let Some(additional_dirs) = &self.config.additional_resource_dirs {
            resource_dirs.extend(additional_dirs.clone());
        }

        // Compile resources
        info!("Compiling resources from {} directories...", resource_dirs.len());
        let mut flat_files = Vec::new();
        let mut missing_dirs = Vec::new();

        for res_dir in &resource_dirs {
            if res_dir.exists() {
                let result = self.compile_resource_dir(res_dir, &compiled_dir)?;
                flat_files.extend(result);
            } else {
                info!("Resource directory not found: {}", res_dir.display());
                missing_dirs.push(res_dir.display().to_string());
            }
        }

        if flat_files.is_empty() {
            AarExtractor::cleanup_aars(&aar_infos)?;
            
            // Provide helpful error message
            let mut error_msg = String::from("No resources found to compile.\n\n");
            
            if !missing_dirs.is_empty() {
                error_msg.push_str("The following resource directories do not exist:\n");
                for dir in &missing_dirs {
                    error_msg.push_str(&format!("  - {}\n", dir));
                }
                error_msg.push_str("\n");
            }
            
            error_msg.push_str("Possible solutions:\n");
            error_msg.push_str("  1. Make sure you're running 'asb build' from your Android project root directory\n");
            error_msg.push_str("  2. Create a config file with: asb init\n");
            error_msg.push_str("  3. Specify custom paths with: asb build --resource-dir <path> --manifest <path> --android-jar <path>\n");
            error_msg.push_str("  4. Check that your resource directory contains valid Android resources\n");
            
            return Ok(BuildResult {
                success: false,
                apk_path: None,
                errors: vec![error_msg],
            });
        }

        info!("Compiled {} resource files", flat_files.len());

        // Save cache
        if let Some(cache) = &self.cache {
            cache.save()?;
        }

        // Link resources into APK
        info!("Linking resources...");
        let output_apk = self.config.output_dir.join(format!(
            "skin-{}.apk",
            self.config.package_name.replace('.', "_")
        ));

        let link_result = self.aapt2.link(
            &flat_files,
            &self.config.manifest_path,
            &self.config.android_jar,
            &output_apk,
            Some(&self.config.package_name),
            self.config.version_code,
            self.config.version_name.as_deref(),
            self.config.stable_ids_file.as_deref(),
        )?;

        // Cleanup AAR extraction directories
        if !aar_infos.is_empty() {
            AarExtractor::cleanup_aars(&aar_infos)?;
            if temp_dir.exists() {
                std::fs::remove_dir_all(&temp_dir).ok();
            }
        }

        if !link_result.success {
            return Ok(BuildResult {
                success: false,
                apk_path: None,
                errors: link_result.errors,
            });
        }

        info!("Build completed successfully!");
        Ok(BuildResult {
            success: true,
            apk_path: link_result.apk_path,
            errors: vec![],
        })
    }

    /// Compile a resource directory
    fn compile_resource_dir(&mut self, res_dir: &Path, compiled_dir: &Path) -> Result<Vec<PathBuf>> {
        // If incremental build is disabled or no cache, compile the whole directory
        if self.cache.is_none() {
            let result = self.aapt2.compile_dir(res_dir, compiled_dir)?;
            if !result.success {
                anyhow::bail!("Compilation failed: {:?}", result.errors);
            }
            return Ok(result.flat_files);
        }

        // For incremental builds, check each file individually
        let resource_files = self.find_resource_files(res_dir)?;
        debug!("Found {} resource files", resource_files.len());

        // Set number of parallel workers (note: this only works if not already initialized)
        if let Some(workers) = self.config.parallel_workers {
            if rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build_global()
                .is_err()
            {
                debug!("Worker thread count already set, using existing pool");
            }
        }

        let cache = self.cache.as_mut().unwrap();
        let aapt2 = &self.aapt2;

        // First, determine serially which files need recompilation and which can use cache
        let mut to_compile: Vec<PathBuf> = Vec::new();
        let mut cached_results: Vec<(PathBuf, PathBuf)> = Vec::new();

        for resource_file in &resource_files {
            if cache.needs_recompile(resource_file).unwrap_or(true) {
                // Need to recompile
                to_compile.push(resource_file.clone());
            } else {
                // Use cached flat file if available
                debug!("Using cached: {}", resource_file.display());
                if let Some(flat) = cache.get_cached_flat_file(resource_file) {
                    cached_results.push((resource_file.clone(), flat));
                }
            }
        }

        // Process recompilations in parallel
        let flat_files_results = if !to_compile.is_empty() {
            debug!("Recompiling {} files...", to_compile.len());
            aapt2.compile_files_parallel(&to_compile, compiled_dir)?
        } else {
            CompileResult {
                success: true,
                flat_files: vec![],
                errors: vec![],
            }
        };

        if !flat_files_results.success {
            anyhow::bail!("Parallel compilation failed: {:?}", flat_files_results.errors);
        }

        let mut flat_files = Vec::new();

        // First, handle newly compiled results
        for (i, resource_file) in to_compile.iter().enumerate() {
            if i < flat_files_results.flat_files.len() {
                let flat_file = &flat_files_results.flat_files[i];
                cache.update_entry(resource_file, flat_file)?;
                if flat_file.exists() {
                    flat_files.push(flat_file.clone());
                }
            }
        }

        // Then, handle cached results
        for (resource_file, flat_file) in cached_results {
            cache.update_entry(&resource_file, &flat_file)?;
            if flat_file.exists() {
                flat_files.push(flat_file);
            }
        }

        Ok(flat_files)
    }

    /// Find all resource files in a directory
    fn find_resource_files(&self, res_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(res_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') && name != "Thumbs.db" {
                    files.push(path.to_path_buf());
                }
            }
        }

        Ok(files)
    }

    /// Clean build artifacts
    pub fn clean(&self) -> Result<()> {
        let compiled_dir = self
            .config
            .compiled_dir
            .clone()
            .unwrap_or_else(|| self.config.output_dir.join("compiled"));
        let temp_dir = self.config.output_dir.join(".temp");

        if compiled_dir.exists() {
            std::fs::remove_dir_all(&compiled_dir)?;
        }

        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)?;
        }

        if let Some(cache_dir) = &self.config.cache_dir {
            if cache_dir.exists() {
                std::fs::remove_dir_all(cache_dir)?;
            }
        }

        info!("Build artifacts cleaned");
        Ok(())
    }
}
