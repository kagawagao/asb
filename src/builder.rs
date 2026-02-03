use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::aapt2::Aapt2;
use crate::aar::AarExtractor;
use crate::cache::BuildCache;
use crate::types::{BuildConfig, BuildResult, CompileResult};

/// Normalize a resource path by removing version qualifiers
/// e.g., "res/drawable-v21/icon.xml" -> "res/drawable/icon.xml"
/// e.g., "res/color-v11/primary.xml" -> "res/color/primary.xml"
fn normalize_resource_path(path: &str) -> String {
    if !path.starts_with("res/") {
        return path.to_string();
    }
    
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 3 {
        return path.to_string();
    }
    
    // parts[0] = "res"
    // parts[1] = resource type (e.g., "drawable-v21", "mipmap-xxhdpi-v4")
    // parts[2..] = file path
    
    let res_type = parts[1];
    
    // Remove version qualifiers like -v21, -v11, -v4, etc.
    // Also handle complex qualifiers like "mipmap-xxhdpi-v4" -> "mipmap-xxhdpi"
    let normalized_type = if let Some(v_pos) = res_type.rfind("-v") {
        // Check if what follows "-v" is a number
        let after_v = &res_type[v_pos + 2..];
        if after_v.chars().all(|c| c.is_ascii_digit()) {
            res_type[..v_pos].to_string()
        } else {
            res_type.to_string()
        }
    } else {
        res_type.to_string()
    };
    
    // Reconstruct the path
    format!("res/{}/{}", normalized_type, parts[2..].join("/"))
}

/// Check if the resource directories contain adaptive-icon resources
fn has_adaptive_icon_resources(resource_dirs: &[PathBuf]) -> bool {
    for res_dir in resource_dirs {
        for entry in WalkDir::new(res_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(parent) = path.parent() {
                    if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                        // Check for mipmap-anydpi (without version qualifier) or mipmap-anydpi-v* folders
                        if parent_name.starts_with("mipmap-anydpi") {
                            if let Ok(content) = fs::read_to_string(path) {
                                if content.contains("<adaptive-icon") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Inject package attribute into manifest if missing
/// Returns the path to the processed manifest (either original or temp file)
fn inject_package_if_needed(
    manifest_path: &Path,
    package_name: &str,
    output_dir: &Path,
) -> Result<PathBuf> {
    let manifest_content = fs::read_to_string(manifest_path)?;
    
    if manifest_content.contains("package=") {
        // Manifest already has package attribute, use as-is
        return Ok(manifest_path.to_path_buf());
    }
    
    warn!("AndroidManifest.xml is missing 'package' attribute, injecting package=\"{}\"", package_name);
    
    // Need to inject package attribute
    let mut modified_content = manifest_content;
    
    // Find the <manifest tag and inject the package attribute
    if let Some(pos) = modified_content.find("<manifest") {
        // Find the end of the opening tag
        if let Some(end_pos) = modified_content[pos..].find('>') {
            let end_index = pos + end_pos;
            
            // Insert package attribute before the closing >
            let insert_pos = end_index;
            let package_attr = format!("\n    package=\"{}\"", package_name);
            modified_content.insert_str(insert_pos, &package_attr);
        }
    }
    
    // Write to temporary file
    fs::create_dir_all(output_dir)?;
    let temp_manifest = output_dir.join(".temp_AndroidManifest.xml");
    fs::write(&temp_manifest, modified_content)?;
    
    Ok(temp_manifest)
}

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

        // Compile resources - compile each directory separately to avoid file name conflicts
        info!("Compiling resources from {} directories...", resource_dirs.len());
        let mut all_flat_files = Vec::new();
        let mut missing_dirs = Vec::new();
        let mut valid_resource_dirs = Vec::new();

        for (idx, res_dir) in resource_dirs.iter().enumerate() {
            if res_dir.exists() {
                // Use a separate compiled subdirectory for each resource directory to avoid flat file conflicts
                let module_compiled_dir = compiled_dir.join(format!("module_{}", idx));
                std::fs::create_dir_all(&module_compiled_dir)?;
                
                let files = self.find_resource_files(res_dir)?;
                if !files.is_empty() {
                    let flat_files = self.compile_all_resources(&files, &module_compiled_dir)?;
                    all_flat_files.extend(flat_files);
                }
                valid_resource_dirs.push(res_dir.clone());
            } else {
                info!("Resource directory not found: {}", res_dir.display());
                missing_dirs.push(res_dir.display().to_string());
            }
        }

        let flat_files = all_flat_files;

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

        // Inject package attribute if missing (aapt2's --rename-manifest-package only renames, doesn't add)
        let processed_manifest = inject_package_if_needed(
            &self.config.manifest_path,
            &self.config.package_name,
            &self.config.output_dir,
        )?;

        // Determine if we need to set min SDK version for adaptive icons
        // Use aapt2's --min-sdk-version parameter instead of modifying manifest
        let min_sdk_version = if has_adaptive_icon_resources(&resource_dirs) {
            warn!("Detected adaptive-icon resources, setting minimum SDK version to 26");
            Some(26)
        } else {
            None
        };

        // Link resources into skin package
        info!("Linking resources...");
        let output_filename = self.config.output_file.as_ref()
            .map(|f| f.clone())
            .unwrap_or_else(|| format!("{}.skin", self.config.package_name));
        
        let output_apk = self.config.output_dir.join(output_filename);

        let link_result = self.aapt2.link(
            &flat_files,
            &processed_manifest,
            &self.config.android_jar,
            &output_apk,
            Some(&self.config.package_name),
            self.config.version_code,
            self.config.version_name.as_deref(),
            self.config.stable_ids_file.as_deref(),
            self.config.package_id.as_deref(),
            min_sdk_version,
        )?;

        // Cleanup temporary manifest if created
        if processed_manifest != self.config.manifest_path {
            fs::remove_file(&processed_manifest).ok();
        }

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

        // Add raw resource files to the skin package
        info!("Adding resource files to skin package...");
        self.add_resources_to_apk(&output_apk, &valid_resource_dirs)?;

        info!("Build completed successfully!");
        Ok(BuildResult {
            success: true,
            apk_path: link_result.apk_path,
            errors: vec![],
        })
    }

    /// Add additional resource files to the APK if needed
    /// Note: aapt2 already compiles and includes all resources in binary format.
    /// This function is kept for future extensibility but currently just validates the APK.
    fn add_resources_to_apk(&self, _apk_path: &Path, _resource_dirs: &[PathBuf]) -> Result<()> {
        // aapt2 link already includes all compiled resources in the APK
        // including layouts, drawables, and other resource files in binary XML format.
        // Resources.arsc contains the resource table with IDs and references.
        // The compiled binary XML files are what Android expects at runtime.
        
        // No additional processing needed - aapt2 has done everything correctly
        Ok(())
    }

    /// Compile all resource files from multiple directories
    fn compile_all_resources(&mut self, resource_files: &[PathBuf], compiled_dir: &Path) -> Result<Vec<PathBuf>> {
        // If incremental build is disabled or no cache, compile all files together
        if self.cache.is_none() {
            // Clear compiled directory to avoid stale flat files
            if compiled_dir.exists() {
                std::fs::remove_dir_all(compiled_dir)?;
            }
            std::fs::create_dir_all(compiled_dir)?;
            
            // Compile all files in parallel
            let result = self.aapt2.compile_files_parallel(resource_files, compiled_dir)?;
            if !result.success {
                anyhow::bail!("Compilation failed: {:?}", result.errors);
            }
            return Ok(result.flat_files);
        }

        // For incremental builds, check each file individually
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

        for resource_file in resource_files {
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

        // Deduplicate flat files to avoid passing the same file multiple times to link
        flat_files.sort();
        flat_files.dedup();

        Ok(flat_files)
    }

    /// Compile a resource directory
    fn compile_resource_dir(&mut self, res_dir: &Path, compiled_dir: &Path) -> Result<Vec<PathBuf>> {
        // If incremental build is disabled or no cache, compile the whole directory
        if self.cache.is_none() {
            // Clear compiled directory to avoid stale flat files
            if compiled_dir.exists() {
                std::fs::remove_dir_all(compiled_dir)?;
            }
            std::fs::create_dir_all(compiled_dir)?;
            
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

        // Deduplicate flat files to avoid passing the same file multiple times to link
        flat_files.sort();
        flat_files.dedup();

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
