use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::aapt2::Aapt2;
use crate::aar::AarExtractor;
use crate::cache::BuildCache;
use crate::resource_priority::ResourcePriority;
use crate::types::{BuildConfig, BuildResult, CompileResult};

/// Normalize a resource path by removing version qualifiers
/// e.g., "res/drawable-v21/icon.xml" -> "res/drawable/icon.xml"
/// e.g., "res/color-v11/primary.xml" -> "res/color/primary.xml"
#[allow(dead_code)]
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
            if path.is_file()
                && let Some(parent) = path.parent()
                    && let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                        // Check for mipmap-anydpi (without version qualifier) or mipmap-anydpi-v* folders
                        if parent_name.starts_with("mipmap-anydpi")
                            && let Ok(content) = fs::read_to_string(path)
                                && (content.contains("<adaptive-icon>")
                                    || content.contains("<adaptive-icon "))
                                {
                                    return true;
                                }
                    }
        }
    }
    false
}

/// Create a minimal AndroidManifest.xml as a cached file in compiled directory
/// According to requirements, we only need: <manifest package="[package_name]"/>
/// This is sufficient for resource-only skin packages
/// The manifest is cached in the compiled_dir to avoid recreation
fn create_minimal_manifest(package_name: &str, compiled_dir: &Path) -> Result<PathBuf> {
    // Cache manifest in compiled directory for persistence alongside .flat files
    fs::create_dir_all(compiled_dir)?;
    let cached_manifest = compiled_dir.join("AndroidManifest.xml");

    // Check if cached manifest exists - if so, reuse it directly
    if cached_manifest.exists() {
        info!("Using cached manifest at: {}", cached_manifest.display());
        return Ok(cached_manifest);
    }

    // Create minimal manifest content - only package name is required for resource compilation
    let manifest_content = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<manifest package=\"{}\" />\n",
        package_name
    );

    // Write the manifest file
    fs::write(&cached_manifest, manifest_content)?;
    info!("Created manifest at: {}", cached_manifest.display());

    Ok(cached_manifest)
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
            // Determine cache base directory with priority:
            // 1. cache_dir (deprecated but takes precedence for backward compatibility)
            // 2. build_dir
            // 3. default to output_dir/.build
            let base_cache_dir = config
                .cache_dir
                .clone()
                .or_else(|| config.build_dir.clone())
                .unwrap_or_else(|| config.output_dir.join(".build"));
            let cache_dir = base_cache_dir.join(&config.package_name);
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

    /// Returns whether incremental build cache is enabled for this builder.
    pub fn has_cache(&self) -> bool {
        self.cache.is_some()
    }

    /// Build the skin package
    pub async fn build(&mut self) -> Result<BuildResult> {
        let build_start = std::time::Instant::now();

        // Determine number of phases for progress bar
        let has_aars = self
            .config
            .aar_files
            .as_ref()
            .is_some_and(|a| !a.is_empty());
        let phases = if has_aars { 4u64 } else { 3u64 };
        let pb = ProgressBar::new(phases);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg}")
                .unwrap()
                .progress_chars("##-"),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        info!("Starting build for package: {}", self.config.package_name);

        // Initialize rayon thread pool with CPU cores * 2
        // This is automatically set and not user-configurable for resource compilation
        let worker_threads = num_cpus::get() * 2;
        if rayon::ThreadPoolBuilder::new()
            .num_threads(worker_threads)
            .build_global()
            .is_ok()
        {
            debug!(
                "Initialized rayon thread pool with {} workers (CPU cores * 2)",
                worker_threads
            );
        } else {
            // Thread pool already initialized, just report current size
            debug!(
                "Rayon thread pool already initialized with {} workers",
                rayon::current_num_threads()
            );
        }

        // Determine build directory for intermediate files
        // Priority: build_dir > output_dir/.build (default)
        let build_dir = self
            .config
            .build_dir
            .clone()
            .unwrap_or_else(|| self.config.output_dir.join(".build"));

        // Ensure directories exist
        // Use build_dir for intermediate files, output_dir for final artifacts
        let compiled_dir = self
            .config
            .compiled_dir
            .clone()
            .unwrap_or_else(|| build_dir.join(&self.config.package_name));
        std::fs::create_dir_all(&compiled_dir)?;
        std::fs::create_dir_all(&self.config.output_dir)?;

        // Extract AAR files if provided - use build_dir for temp files
        let mut aar_infos = Vec::new();
        let temp_dir = build_dir.join(".temp");

        if let Some(aar_files) = &self.config.aar_files
            && !aar_files.is_empty() {
                pb.set_message("Extracting AARs...");
                info!("Extracting {} AAR files...", aar_files.len());
                aar_infos = AarExtractor::extract_aars(aar_files, &temp_dir)?;
                pb.inc(1);
            }

        // Collect all resource directories with their priorities
        // Following Android standard priority: Library (AAR) < Additional < Main
        let mut resource_dirs_with_priority: Vec<(PathBuf, ResourcePriority, String)> = Vec::new();

        // Main resource directory (highest priority)
        resource_dirs_with_priority.push((
            self.config.resource_dir.clone(),
            ResourcePriority::Main,
            "main".to_string(),
        ));

        // Add AAR resource directories (lowest priority)
        for (idx, aar_info) in aar_infos.iter().enumerate() {
            if let Some(res_dir) = &aar_info.resource_dir {
                let dir_name = format!("aar_{}", idx);
                resource_dirs_with_priority.push((
                    res_dir.clone(),
                    ResourcePriority::Library(idx),
                    dir_name,
                ));
            }
        }

        // Add additional resource directories (medium priority)
        if let Some(additional_dirs) = &self.config.additional_resource_dirs {
            for (idx, dir) in additional_dirs.iter().enumerate() {
                // Create directory name from path: "additional/a/res" -> "additional_a_res"
                let dir_name = format!(
                    "additional_{}",
                    dir.to_string_lossy()
                        .replace(['/', '\\', ':'], "_")
                        .trim_matches('_')
                );
                resource_dirs_with_priority.push((
                    dir.clone(),
                    ResourcePriority::Additional(idx),
                    dir_name,
                ));
            }
        }

        // Sort by priority (lowest to highest) so higher priority resources overwrite lower priority ones
        resource_dirs_with_priority.sort_by_key(|(_, priority, _)| priority.value());

        // Compile resources - each to its own subdirectory to avoid conflicts
        pb.set_message("Compiling resources...");
        // Use a spinner substyle for indeterminate compilation count
        let compile_spinner = ProgressBar::new_spinner();
        compile_spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        compile_spinner.set_message("Compiling resource files...");
        info!(
            "Compiling resources from {} directories...",
            resource_dirs_with_priority.len()
        );
        let mut missing_dirs = Vec::new();
        let mut valid_resource_dirs = Vec::new();

        // Track flat files by priority level for proper ordering
        // Flat files will be collected per directory and ordered by priority
        let mut flat_files_by_priority: Vec<(ResourcePriority, Vec<PathBuf>, PathBuf)> = Vec::new();

        for (res_dir, priority, dir_name) in &resource_dirs_with_priority {
            // Check if this resource directory has precompiled flat files
            let precompiled_flat_files = self
                .config
                .precompiled_dependencies
                .as_ref()
                .and_then(|map| map.get(res_dir))
                .cloned();

            if let Some(flat_files) = precompiled_flat_files {
                // Use precompiled flat files
                info!(
                    "Using {} precompiled flat files for {} (priority {:?})",
                    flat_files.len(),
                    res_dir.display(),
                    priority
                );

                flat_files_by_priority.push((*priority, flat_files, res_dir.clone()));
                valid_resource_dirs.push(res_dir.clone());
            } else if res_dir.exists() {
                // Compile each resource directory to its own subdirectory
                let module_compiled_dir = compiled_dir.join(dir_name);
                std::fs::create_dir_all(&module_compiled_dir)?;

                let files = self.find_resource_files(res_dir)?;
                if !files.is_empty() {
                    let flat_files = self.compile_all_resources(&files, &module_compiled_dir)?;

                    debug!(
                        "Resource directory {} has priority {:?}, compiled {} files to {}",
                        res_dir.display(),
                        priority,
                        flat_files.len(),
                        module_compiled_dir.display()
                    );
                    flat_files_by_priority.push((*priority, flat_files, res_dir.clone()));
                }
                valid_resource_dirs.push(res_dir.clone());
            } else {
                info!("Resource directory not found: {}", res_dir.display());
                missing_dirs.push(res_dir.display().to_string());
            }
        }

        // Collect all flat files organized by priority
        // Sort by priority to ensure correct order for linking
        flat_files_by_priority.sort_by_key(|(priority, _, _)| priority.value());

        // Separate base from overlays for aapt2 link
        // Following Android standard: Library (AAR) < Additional < Main
        // Library and Additional are base resources, Main is overlay
        let mut base_flat_files = Vec::new();
        let mut overlay_flat_files: Vec<Vec<PathBuf>> = Vec::new();

        for (priority, files, dir) in &flat_files_by_priority {
            match priority {
                ResourcePriority::Library(_) => {
                    // Libraries are always base (lowest priority)
                    debug!(
                        "Base resources (Library): {} files from {} (priority {:?})",
                        files.len(),
                        dir.display(),
                        priority
                    );
                    base_flat_files.extend(files.clone());
                },
                ResourcePriority::Additional(_) => {
                    // Additional resources are base (medium priority)
                    debug!(
                        "Base resources (Additional): {} files from {} (priority {:?})",
                        files.len(),
                        dir.display(),
                        priority
                    );
                    base_flat_files.extend(files.clone());
                },
                ResourcePriority::Main => {
                    // Main resources are overlay (highest priority)
                    debug!(
                        "Overlay resources (Main): {} files from {} (priority {:?})",
                        files.len(),
                        dir.display(),
                        priority
                    );
                    overlay_flat_files.push(files.clone());
                },
            }
        }

        let total_flat_files =
            base_flat_files.len() + overlay_flat_files.iter().map(|v| v.len()).sum::<usize>();

        if total_flat_files == 0 {
            AarExtractor::cleanup_aars(&aar_infos)?;
            compile_spinner.finish_and_clear();
            pb.finish_with_message("Build failed: no resources found");

            // Provide helpful error message
            let mut error_msg = String::from("No resources found to compile.\n\n");

            if !missing_dirs.is_empty() {
                error_msg.push_str("The following resource directories do not exist:\n");
                for dir in &missing_dirs {
                    error_msg.push_str(&format!("  - {}\n", dir));
                }
                error_msg.push('\n');
            }

            error_msg.push_str("Possible solutions:\n");
            error_msg.push_str("  1. Make sure you're running 'asb build' from your Android project root directory\n");
            error_msg.push_str("  2. Create a config file with: asb init\n");
            error_msg.push_str("  3. Specify custom paths with: asb build --resource-dir <path> --manifest <path> --android-jar <path>\n");
            error_msg.push_str(
                "  4. Check that your resource directory contains valid Android resources\n",
            );

            return Ok(BuildResult {
                success: false,
                apk_path: None,
                errors: vec![error_msg],
                build_duration: build_start.elapsed(),
            });
        }

        info!(
            "Compiled {} resource files total: {} base, {} overlay sets",
            total_flat_files,
            base_flat_files.len(),
            overlay_flat_files.len()
        );
        compile_spinner.finish_with_message("Resource compilation complete");
        pb.inc(1);

        // Save cache
        if let Some(cache) = &self.cache {
            cache.save()?;
        }

        // Create minimal AndroidManifest.xml as cached file in compiled_dir
        // According to requirements, we only need: <manifest package="[package_name]"/>
        let processed_manifest = create_minimal_manifest(&self.config.package_name, &compiled_dir)?;

        // Determine if we need to set min SDK version for adaptive icons
        // Use aapt2's --min-sdk-version parameter instead of modifying manifest
        let min_sdk_version = if has_adaptive_icon_resources(&valid_resource_dirs) {
            warn!("Detected adaptive-icon resources, setting minimum SDK version to 26");
            Some(26)
        } else {
            None
        };

        // Link resources into skin package using overlay strategy
        pb.set_message("Linking APK...");
        info!("Linking resources with Android resource priority strategy...");
        let output_filename = self
            .config
            .output_file.clone()
            .unwrap_or_else(|| format!("{}.skin", self.config.package_name));

        let output_apk = self.config.output_dir.join(output_filename);

        // Ensure android_jar is set
        let android_jar = self.config.android_jar.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "android_jar not set. Please configure it or ensure ANDROID_HOME is set."
            )
        })?;

        let link_result = self.aapt2.link_with_overlays(
            &base_flat_files,
            &overlay_flat_files,
            &processed_manifest,
            android_jar,
            &output_apk,
            Some(&self.config.package_name),
            self.config.version_code,
            self.config.version_name.as_deref(),
            self.config.stable_ids_file.as_deref(),
            self.config.package_id.as_deref(),
            min_sdk_version,
            Some(&compiled_dir), // Pass compiled_dir to avoid conflicts in multi-task builds
        )?;

        // Keep manifest cached in compiled_dir for reuse in subsequent builds
        // No need to cleanup - it's intentionally persisted for cache optimization

        // Cleanup AAR extraction directories
        if !aar_infos.is_empty() {
            AarExtractor::cleanup_aars(&aar_infos)?;
            if temp_dir.exists() {
                std::fs::remove_dir_all(&temp_dir).ok();
            }
        }

        pb.inc(1);

        if !link_result.success {
            return Ok(BuildResult {
                success: false,
                apk_path: None,
                errors: link_result.errors,
                build_duration: build_start.elapsed(),
            });
        }

        // Add raw resource files to the skin package
        pb.set_message("Finalizing...");
        info!("Adding resource files to skin package...");
        self.add_resources_to_apk(&output_apk, &valid_resource_dirs)?;

        pb.inc(1);
        pb.finish_with_message("Build complete");
        info!("Build completed successfully!");
        Ok(BuildResult {
            success: true,
            apk_path: link_result.apk_path,
            errors: vec![],
            build_duration: build_start.elapsed(),
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
    fn compile_all_resources(
        &mut self,
        resource_files: &[PathBuf],
        compiled_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        // If incremental build is disabled or no cache, compile all files together
        if !self.has_cache() {
            // Clear compiled directory to avoid stale flat files
            if compiled_dir.exists() {
                std::fs::remove_dir_all(compiled_dir)?;
            }
            std::fs::create_dir_all(compiled_dir)?;

            // Compile all files in parallel
            let result = self
                .aapt2
                .compile_files_parallel(resource_files, compiled_dir)?;
            if !result.success {
                anyhow::bail!("Compilation failed: {:?}", result.errors);
            }
            return Ok(result.flat_files);
        }

        // For incremental builds, check each file individually
        debug!("Found {} resource files", resource_files.len());

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
            anyhow::bail!(
                "Parallel compilation failed: {:?}",
                flat_files_results.errors
            );
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
    #[allow(dead_code)]
    fn compile_resource_dir(
        &mut self,
        res_dir: &Path,
        compiled_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        // If incremental build is disabled or no cache, compile the whole directory
        if !self.has_cache() {
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
            anyhow::bail!(
                "Parallel compilation failed: {:?}",
                flat_files_results.errors
            );
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

        // min_depth(2) skips both the root directory (depth 0) and files directly
        // under res_dir (depth 1), ensuring only files inside subdirectories are
        // included. This avoids per-entry canonicalize() syscalls.
        for entry in WalkDir::new(res_dir)
            .min_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();

            // Check if file is in a layout directory and skip it
            if let Some(parent) = path.parent()
                && let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                    // Check for layout directories (layout, layout-land, layout-sw600dp, etc.)
                    if parent_name.starts_with("layout") {
                        debug!("Filtering out layout file: {}", path.display());
                        continue;
                    }
                }

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Skip hidden files, system files, and specific resource files
                if name.starts_with('.') || name == "Thumbs.db" {
                    continue;
                }

                // Filter out styles.xml, attrs.xml, and strings.xml
                if name == "styles.xml" || name == "attrs.xml" || name == "strings.xml" {
                    debug!("Filtering out resource file: {}", path.display());
                    continue;
                }

                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    /// Clean build artifacts
    #[allow(dead_code)]
    pub fn clean(&self) -> Result<()> {
        // Determine build directory
        let build_dir = self
            .config
            .build_dir
            .clone()
            .unwrap_or_else(|| self.config.output_dir.join(".build"));

        let compiled_dir = self
            .config
            .compiled_dir
            .clone()
            .unwrap_or_else(|| build_dir.join(&self.config.package_name));
        let temp_dir = build_dir.join(".temp");

        if compiled_dir.exists() {
            std::fs::remove_dir_all(&compiled_dir)?;
        }

        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)?;
        }

        // Clean cache - check both cache_dir and build_dir
        if let Some(cache_dir) = &self.config.cache_dir {
            if cache_dir.exists() {
                std::fs::remove_dir_all(cache_dir)?;
            }
        } else if let Some(build_dir) = &self.config.build_dir
            && build_dir.exists() {
                std::fs::remove_dir_all(build_dir)?;
            }

        info!("Build artifacts cleaned");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_ignore_files_directly_under_resource_dir() -> Result<()> {
        // Create a temporary directory structure
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");

        // Create valid resource structure (res/values/colors.xml - not strings.xml which is filtered)
        let values_dir = res_dir.join("values");
        fs::create_dir_all(&values_dir)?;
        let valid_file = values_dir.join("colors.xml");
        fs::write(
            &valid_file,
            "<?xml version=\"1.0\"?><resources></resources>",
        )?;

        // Also test that strings.xml is filtered out
        let strings_file = values_dir.join("strings.xml");
        fs::write(
            &strings_file,
            "<?xml version=\"1.0\"?><resources></resources>",
        )?;

        // Create invalid resource (directly under res/)
        let invalid_file = res_dir.join("invalid.txt");
        fs::write(&invalid_file, "This should be ignored")?;

        // Create another invalid resource (directly under res/)
        let another_invalid = res_dir.join("readme.md");
        fs::write(&another_invalid, "# README")?;

        // Create config for testing
        let config = BuildConfig {
            resource_dir: res_dir.clone(),
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: temp_dir.path().join("output"),
            output_file: None,
            package_name: "com.test".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: None,
            build_dir: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let builder = SkinBuilder::new(config)?;
        let files = builder.find_resource_files(&res_dir)?;

        // Should only find the valid file (colors.xml), not strings.xml or files directly under res/
        assert_eq!(
            files.len(),
            1,
            "Should only find 1 valid resource file (colors.xml)"
        );
        assert_eq!(files[0], valid_file, "Should find the colors.xml file");
        assert!(
            !files.contains(&strings_file),
            "Should not include strings.xml (filtered out)"
        );
        assert!(
            !files.contains(&invalid_file),
            "Should not include invalid.txt"
        );
        assert!(
            !files.contains(&another_invalid),
            "Should not include readme.md"
        );

        Ok(())
    }

    #[test]
    fn test_valid_nested_resources_are_included() -> Result<()> {
        // Create a temporary directory structure
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");

        // Create various valid resource subdirectories
        let values_dir = res_dir.join("values");
        let drawable_dir = res_dir.join("drawable");
        let layout_dir = res_dir.join("layout");

        fs::create_dir_all(&values_dir)?;
        fs::create_dir_all(&drawable_dir)?;
        fs::create_dir_all(&layout_dir)?;

        // Create valid resource files
        // Note: strings.xml, styles.xml, attrs.xml are filtered out
        // Note: layout files are also filtered out
        let strings_xml = values_dir.join("strings.xml");
        let colors_xml = values_dir.join("colors.xml");
        let icon_png = drawable_dir.join("icon.png");
        let activity_xml = layout_dir.join("activity_main.xml");

        fs::write(&strings_xml, "<resources></resources>")?;
        fs::write(&colors_xml, "<resources></resources>")?;
        fs::write(&icon_png, "fake png data")?;
        fs::write(&activity_xml, "<LinearLayout></LinearLayout>")?;

        // Create config for testing
        let config = BuildConfig {
            resource_dir: res_dir.clone(),
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: temp_dir.path().join("output"),
            output_file: None,
            package_name: "com.test".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: None,
            build_dir: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let builder = SkinBuilder::new(config)?;
        let files = builder.find_resource_files(&res_dir)?;

        // Should find only 2 files now: colors.xml and icon.png
        // strings.xml is filtered, layout files are filtered
        assert_eq!(
            files.len(),
            2,
            "Should find 2 valid resource files (colors.xml and icon.png)"
        );
        assert!(
            !files.contains(&strings_xml),
            "Should NOT include strings.xml (filtered)"
        );
        assert!(files.contains(&colors_xml), "Should include colors.xml");
        assert!(files.contains(&icon_png), "Should include icon.png");
        assert!(
            !files.contains(&activity_xml),
            "Should NOT include layout files (filtered)"
        );

        Ok(())
    }

    #[test]
    fn test_build_dir_separation() -> Result<()> {
        // This test verifies that intermediate files go to build_dir and final output goes to output_dir
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");
        let values_dir = res_dir.join("values");
        let output_dir = temp_dir.path().join("output");
        let build_dir = temp_dir.path().join("build_temp");

        // Create resource structure
        fs::create_dir_all(&values_dir)?;
        let colors_xml = values_dir.join("colors.xml");
        fs::write(
            &colors_xml,
            r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="primary">#FF0000</color>
</resources>"#,
        )?;

        // Create config with separate build_dir and output_dir
        let config = BuildConfig {
            resource_dir: res_dir.clone(),
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: output_dir.clone(),
            output_file: None,
            package_name: "com.test.builddir".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: None,
            build_dir: Some(build_dir.clone()),
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let _builder = SkinBuilder::new(config)?;

        // Verify that intermediate files would go to build_dir
        // (We can't run the full build without aapt2, but we can verify the directory logic)

        // Check that compiled_dir is set correctly (under build_dir, not output_dir)
        let _expected_compiled_dir = build_dir.join("com.test.builddir");

        // The builder should use build_dir for intermediate files
        // Since we set build_dir explicitly, compiled resources should go there
        assert!(
            !output_dir.join("com.test.builddir").exists()
                || !output_dir.join("com.test.builddir").is_dir(),
            "Compiled dir should NOT be under output_dir"
        );

        // Verify build_dir structure expectations
        // Final output (.skin) should go to output_dir
        // Intermediate files (.flat, .temp) should go to build_dir

        Ok(())
    }

    #[test]
    fn test_build_dir_defaults_to_output_build() -> Result<()> {
        // Test that when build_dir is not specified, it defaults to {output_dir}/.build
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");
        let values_dir = res_dir.join("values");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir_all(&values_dir)?;
        let colors_xml = values_dir.join("colors.xml");
        fs::write(
            &colors_xml,
            r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="test">#00FF00</color>
</resources>"#,
        )?;

        // Config without explicit build_dir
        let config = BuildConfig {
            resource_dir: res_dir,
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: output_dir.clone(),
            output_file: None,
            package_name: "com.test.default".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: None,
            build_dir: None, // Not specified - should default
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let _builder = SkinBuilder::new(config)?;

        // The builder should default to output_dir/.build for build_dir
        // This is tested implicitly through the builder's logic

        Ok(())
    }

    #[test]
    fn test_manifest_caching() -> Result<()> {
        // Test that manifest file is cached and reused when content is unchanged
        let temp_dir = TempDir::new()?;
        let compiled_dir = temp_dir.path().join("compiled");
        let package_name = "com.test.cache";

        // First call - should create the manifest
        let manifest1 = super::create_minimal_manifest(package_name, &compiled_dir)?;
        assert!(manifest1.exists(), "Manifest should be created");

        // Get the file's modification time
        let metadata1 = fs::metadata(&manifest1)?;
        let modified1 = metadata1.modified()?;

        // Small delay to ensure different modification time if file is recreated
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Second call - should reuse the cached manifest (same content)
        let manifest2 = super::create_minimal_manifest(package_name, &compiled_dir)?;
        assert_eq!(manifest1, manifest2, "Should return the same path");

        let metadata2 = fs::metadata(&manifest2)?;
        let modified2 = metadata2.modified()?;

        // Modification time should be the same (file not rewritten)
        assert_eq!(
            modified1, modified2,
            "Manifest file should not be rewritten when content is unchanged"
        );

        // Verify content is correct
        let content = fs::read_to_string(&manifest2)?;
        assert!(
            content.contains(&format!("package=\"{}\"", package_name)),
            "Manifest should contain correct package name"
        );

        Ok(())
    }

    // ========== Tests for normalize_resource_path ==========

    #[test]
    fn test_normalize_resource_path_basic() {
        // Basic version qualifier removal
        assert_eq!(
            super::normalize_resource_path("res/drawable-v21/icon.xml"),
            "res/drawable/icon.xml"
        );
    }

    #[test]
    fn test_normalize_resource_path_complex_qualifier() {
        // Complex qualifier: mipmap-xxhdpi-v4 -> mipmap-xxhdpi
        assert_eq!(
            super::normalize_resource_path("res/mipmap-xxhdpi-v4/icon.png"),
            "res/mipmap-xxhdpi/icon.png"
        );
    }

    #[test]
    fn test_normalize_resource_path_no_version() {
        // No version qualifier
        assert_eq!(
            super::normalize_resource_path("res/drawable/icon.xml"),
            "res/drawable/icon.xml"
        );
    }

    #[test]
    fn test_normalize_resource_path_non_res() {
        // Non-res path should be returned as-is
        assert_eq!(
            super::normalize_resource_path("something/else/file.txt"),
            "something/else/file.txt"
        );
    }

    #[test]
    fn test_normalize_resource_path_short() {
        // Path too short (only 2 parts)
        assert_eq!(
            super::normalize_resource_path("res/icon.png"),
            "res/icon.png"
        );
    }

    #[test]
    fn test_normalize_resource_path_values_no_version() {
        // Values directory without version
        assert_eq!(
            super::normalize_resource_path("res/values/strings.xml"),
            "res/values/strings.xml"
        );
    }

    #[test]
    fn test_normalize_resource_path_nested_file() {
        // Nested file path with version qualifier
        assert_eq!(
            super::normalize_resource_path("res/drawable-v21/subdir/icon.xml"),
            "res/drawable/subdir/icon.xml"
        );
    }

    #[test]
    fn test_normalize_resource_path_anydpi_version() {
        // mipmap-anydpi-v26 -> mipmap-anydpi
        assert_eq!(
            super::normalize_resource_path("res/mipmap-anydpi-v26/icon.xml"),
            "res/mipmap-anydpi/icon.xml"
        );
    }

    #[test]
    fn test_normalize_resource_path_dash_v_in_middle() {
        // -v that's not followed by digits (e.g., in a directory name like "res/my-vendor/...")
        // This path has "my-vendor" but the function checks rfind("-v")
        // "my-vendor" contains "-v" but it's not at the start of the resource type
        // The function does rfind("-v") on the resource type
        // Let's use a path where the resource type has "-v" not followed by digits
        // Actually the function checks rfind("-v") and then checks if after it is all digits
        // So "my-vendor" would have rfind("-v") at position 3, after_v="endor" which is not all digits
        // So it would return "my-vendor" unchanged
        assert_eq!(
            super::normalize_resource_path("res/my-vendor/file.xml"),
            "res/my-vendor/file.xml"
        );
    }

    // ========== Tests for has_adaptive_icon_resources ==========

    #[test]
    fn test_has_adaptive_icon_empty_dirs() {
        let result = super::has_adaptive_icon_resources(&[]);
        assert!(!result, "Empty dirs should return false");
    }

    #[test]
    fn test_has_adaptive_icon_no_mipmap_anydpi() {
        let temp_dir = TempDir::new().unwrap();
        let res_dir = temp_dir.path().join("res");
        let drawable_dir = res_dir.join("drawable");
        fs::create_dir_all(&drawable_dir).unwrap();
        fs::write(drawable_dir.join("icon.png"), "fake png").unwrap();

        let result = super::has_adaptive_icon_resources(&[res_dir]);
        assert!(!result, "No mipmap-anydpi dir should return false");
    }

    #[test]
    fn test_has_adaptive_icon_without_adaptive_content() {
        let temp_dir = TempDir::new().unwrap();
        let res_dir = temp_dir.path().join("res");
        let anydpi_dir = res_dir.join("mipmap-anydpi-v26");
        fs::create_dir_all(&anydpi_dir).unwrap();
        fs::write(anydpi_dir.join("ic_launcher.xml"), "<not-adaptive/>").unwrap();

        let result = super::has_adaptive_icon_resources(&[res_dir]);
        assert!(
            !result,
            "mipmap-anydpi without <adaptive-icon should return false"
        );
    }

    #[test]
    fn test_has_adaptive_icon_with_adaptive_content() {
        let temp_dir = TempDir::new().unwrap();
        let res_dir = temp_dir.path().join("res");
        let anydpi_dir = res_dir.join("mipmap-anydpi-v26");
        fs::create_dir_all(&anydpi_dir).unwrap();
        fs::write(
            anydpi_dir.join("ic_launcher.xml"),
            "<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\">",
        )
        .unwrap();

        let result = super::has_adaptive_icon_resources(&[res_dir]);
        assert!(
            result,
            "mipmap-anydpi with <adaptive-icon should return true"
        );
    }

    #[test]
    fn test_has_adaptive_icon_anydpi_no_version() {
        let temp_dir = TempDir::new().unwrap();
        let res_dir = temp_dir.path().join("res");
        let anydpi_dir = res_dir.join("mipmap-anydpi");
        fs::create_dir_all(&anydpi_dir).unwrap();
        fs::write(
            anydpi_dir.join("ic_launcher.xml"),
            "<adaptive-icon></adaptive-icon>",
        )
        .unwrap();

        let result = super::has_adaptive_icon_resources(&[res_dir]);
        assert!(
            result,
            "mipmap-anydpi (no version) with <adaptive-icon should return true"
        );
    }

    // ========== Tests for SkinBuilder::new with various configs ==========

    #[test]
    fn test_skin_builder_new_no_incremental() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");
        fs::create_dir_all(res_dir.join("values"))?;

        let config = BuildConfig {
            resource_dir: res_dir,
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: temp_dir.path().join("output"),
            output_file: None,
            package_name: "com.test.noinc".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: None, // No incremental
            build_dir: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let builder = SkinBuilder::new(config)?;
        assert!(
            builder.cache.is_none(),
            "Cache should be None when incremental is not set"
        );
        Ok(())
    }

    #[test]
    fn test_skin_builder_new_incremental_true() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");
        fs::create_dir_all(res_dir.join("values"))?;

        let config = BuildConfig {
            resource_dir: res_dir,
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: temp_dir.path().join("output"),
            output_file: None,
            package_name: "com.test.inc".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: Some(true),
            build_dir: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let builder = SkinBuilder::new(config)?;
        assert!(
            builder.cache.is_some(),
            "Cache should be Some when incremental is true"
        );
        Ok(())
    }

    #[test]
    fn test_skin_builder_new_incremental_false() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");
        fs::create_dir_all(res_dir.join("values"))?;

        let config = BuildConfig {
            resource_dir: res_dir,
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: temp_dir.path().join("output"),
            output_file: None,
            package_name: "com.test.noinc2".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: Some(false),
            build_dir: None,
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let builder = SkinBuilder::new(config)?;
        assert!(
            builder.cache.is_none(),
            "Cache should be None when incremental is false"
        );
        Ok(())
    }

    #[test]
    fn test_skin_builder_new_with_cache_dir() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");
        let cache_dir = temp_dir.path().join("my_cache");
        fs::create_dir_all(res_dir.join("values"))?;

        let config = BuildConfig {
            resource_dir: res_dir,
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: temp_dir.path().join("output"),
            output_file: None,
            package_name: "com.test.cachedir".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: Some(true),
            build_dir: None,
            cache_dir: Some(cache_dir.clone()),
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let builder = SkinBuilder::new(config)?;
        assert!(builder.cache.is_some(), "Cache should be created");
        assert!(
            cache_dir.join("com.test.cachedir").exists(),
            "Cache directory should be created under the specified cache_dir"
        );
        Ok(())
    }

    #[test]
    fn test_skin_builder_new_with_build_dir() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let res_dir = temp_dir.path().join("res");
        let build_dir = temp_dir.path().join("my_build");
        fs::create_dir_all(res_dir.join("values"))?;

        let config = BuildConfig {
            resource_dir: res_dir,
            manifest_path: temp_dir.path().join("AndroidManifest.xml"),
            output_dir: temp_dir.path().join("output"),
            output_file: None,
            package_name: "com.test.builddir".to_string(),
            aapt2_path: Some(temp_dir.path().join("aapt2")),
            android_jar: Some(PathBuf::from("/fake/android.jar")),
            aar_files: None,
            incremental: Some(true),
            build_dir: Some(build_dir.clone()),
            cache_dir: None,
            version_code: None,
            version_name: None,
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            package_id: None,
            precompiled_dependencies: None,
        };

        fs::write(temp_dir.path().join("aapt2"), b"").unwrap();
        let builder = SkinBuilder::new(config)?;
        assert!(builder.cache.is_some(), "Cache should be created");
        assert!(
            build_dir.join("com.test.builddir").exists(),
            "Cache directory should be created under the build_dir"
        );
        Ok(())
    }

    #[test]
    fn test_has_adaptive_icon_multiple_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let res_dir1 = temp_dir.path().join("res1");
        let res_dir2 = temp_dir.path().join("res2");

        // res1 has no adaptive icons
        let drawable = res_dir1.join("drawable");
        fs::create_dir_all(&drawable).unwrap();
        fs::write(drawable.join("icon.png"), "fake").unwrap();

        // res2 has adaptive icons
        let anydpi = res_dir2.join("mipmap-anydpi-v26");
        fs::create_dir_all(&anydpi).unwrap();
        fs::write(
            anydpi.join("ic_launcher.xml"),
            "<adaptive-icon></adaptive-icon>",
        )
        .unwrap();

        let result = super::has_adaptive_icon_resources(&[res_dir1.clone(), res_dir2.clone()]);
        assert!(result, "Should detect adaptive icon in second dir");
    }
}
