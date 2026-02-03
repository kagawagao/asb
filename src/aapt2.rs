use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

use crate::types::{CompileResult, LinkResult};

/// Default Android package ID for standard applications
/// This is used for dynamic resource loading via new Resources()
pub const DEFAULT_PACKAGE_ID: &str = "0x7f";

/// Utility for interacting with aapt2
pub struct Aapt2 {
    aapt2_path: PathBuf,
}

impl Aapt2 {
    /// Create a new Aapt2 instance
    pub fn new(aapt2_path: Option<PathBuf>) -> Result<Self> {
        let path = match aapt2_path {
            Some(p) => p,
            None => Self::find_aapt2()?,
        };

        Ok(Self { aapt2_path: path })
    }

    /// Find aapt2 binary in the system
    fn find_aapt2() -> Result<PathBuf> {
        // Try PATH first
        if let Ok(output) = Command::new(if cfg!(windows) { "where" } else { "which" })
            .arg("aapt2")
            .output()
        {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = path_str.lines().next() {
                    let path = PathBuf::from(line.trim());
                    if path.exists() {
                        info!("Found aapt2 at: {}", path.display());
                        return Ok(path);
                    }
                }
            }
        }

        // Try ANDROID_HOME
        if let Ok(android_home) = std::env::var("ANDROID_HOME") {
            let build_tools_dir = PathBuf::from(android_home).join("build-tools");
            if build_tools_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&build_tools_dir) {
                    let mut versions: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .collect();
                    versions.sort_by(|a, b| b.path().cmp(&a.path()));

                    for entry in versions {
                        let aapt2_name = if cfg!(windows) { "aapt2.exe" } else { "aapt2" };
                        let aapt2_path = entry.path().join(aapt2_name);
                        if aapt2_path.exists() {
                            info!("Found aapt2 at: {}", aapt2_path.display());
                            return Ok(aapt2_path);
                        }
                    }
                }
            }
        }

        anyhow::bail!(
            "aapt2 not found. Please install Android SDK and set ANDROID_HOME, or provide aapt2Path"
        )
    }

    /// Get aapt2 version
    pub fn version(&self) -> Result<String> {
        let output = Command::new(&self.aapt2_path)
            .arg("version")
            .output()
            .context("Failed to execute aapt2")?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Compile a directory of resources
    pub fn compile_dir(&self, resource_dir: &Path, output_dir: &Path) -> Result<CompileResult> {
        std::fs::create_dir_all(output_dir)?;

        debug!("Compiling resources from: {}", resource_dir.display());

        let output = Command::new(&self.aapt2_path)
            .arg("compile")
            .arg("--dir")
            .arg(resource_dir)
            .arg("-o")
            .arg(output_dir)
            .output()
            .context("Failed to execute aapt2 compile")?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Ok(CompileResult {
                success: false,
                flat_files: vec![],
                errors: vec![stderr.to_string()],
            });
        }

        // Collect all .flat files
        let flat_files = Self::collect_flat_files(output_dir)?;

        Ok(CompileResult {
            success: true,
            flat_files,
            errors: vec![],
        })
    }

    /// Compile individual resource files in parallel
    pub fn compile_files_parallel(
        &self,
        resource_files: &[PathBuf],
        output_dir: &Path,
    ) -> Result<CompileResult> {
        use rayon::prelude::*;

        std::fs::create_dir_all(output_dir)?;

        let results: Vec<_> = resource_files
            .par_iter()
            .map(|file| {
                // For parallel compilation, we can't use before/after file lists
                // because other threads are also writing files. Instead, we predict
                // the flat file name based on the resource file path.
                let output = Command::new(&self.aapt2_path)
                    .arg("compile")
                    .arg("-o")
                    .arg(output_dir)
                    .arg(file)
                    .output()
                    .context("Failed to execute aapt2 compile")?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("Failed to compile {}: {}", file.display(), stderr);
                }

                // Predict the flat file name based on the resource file path
                // aapt2 creates names like:
                //   - values_strings.arsc.flat for res/values/strings.xml
                //   - layout_activity_main.xml.flat for res/layout/activity_main.xml
                if let Some(parent) = file.parent() {
                    if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                        if let Some(file_name) = file.file_name().and_then(|n| n.to_str()) {
                            // Try different naming patterns based on resource type
                            let possible_names = if parent_name.starts_with("values") {
                                // For values resources: values_strings.arsc.flat
                                vec![format!("{}_{}.arsc.flat", parent_name,
                                    file.file_stem().and_then(|s| s.to_str()).unwrap_or(""))]
                            } else {
                                // For other resources (layout, drawable, etc.): layout_activity_main.xml.flat
                                vec![format!("{}_{}.flat", parent_name, file_name)]
                            };
                            
                            for flat_name in possible_names {
                                let flat_path = output_dir.join(&flat_name);
                                if flat_path.exists() {
                                    return Ok(flat_path);
                                }
                            }
                        }
                    }
                }
                
                anyhow::bail!("Could not find compiled flat file for {}", file.display())
            })
            .collect();

        let mut flat_files = Vec::new();
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(flat) => flat_files.push(flat),
                Err(e) => errors.push(e.to_string()),
            }
        }

        Ok(CompileResult {
            success: errors.is_empty(),
            flat_files,
            errors,
        })
    }

    /// Compile a single resource file
    fn compile_single_file(&self, resource_file: &Path, output_dir: &Path) -> Result<PathBuf> {
        // Get existing flat files before compilation
        let before_files = Self::collect_flat_files(output_dir)?;
        
        let output = Command::new(&self.aapt2_path)
            .arg("compile")
            .arg("-o")
            .arg(output_dir)
            .arg(resource_file)
            .output()
            .context("Failed to execute aapt2 compile")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to compile {}: {}", resource_file.display(), stderr);
        }

        // Get flat files after compilation - the new one is our result
        let after_files = Self::collect_flat_files(output_dir)?;
        
        // Find the newly created flat file
        for file in &after_files {
            if !before_files.contains(file) {
                return Ok(file.clone());
            }
        }
        
        // If we didn't find a new file, it might have been overwritten
        // In that case, try to guess the name based on the resource file path
        // aapt2 creates names like: 
        //   - values_strings.arsc.flat for res/values/strings.xml
        //   - layout_activity_main.xml.flat for res/layout/activity_main.xml
        if let Some(parent) = resource_file.parent() {
            if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                if let Some(file_name) = resource_file.file_name().and_then(|n| n.to_str()) {
                    // Try different naming patterns based on resource type
                    let possible_names = if parent_name.starts_with("values") {
                        // For values resources: values_strings.arsc.flat
                        vec![format!("{}_{}.arsc.flat", parent_name, 
                            resource_file.file_stem().and_then(|s| s.to_str()).unwrap_or(""))]
                    } else {
                        // For other resources (layout, drawable, etc.): layout_activity_main.xml.flat
                        vec![format!("{}_{}.flat", parent_name, file_name)]
                    };
                    
                    for flat_name in possible_names {
                        let flat_path = output_dir.join(&flat_name);
                        if flat_path.exists() {
                            return Ok(flat_path);
                        }
                    }
                }
            }
        }
        
        anyhow::bail!("Could not find compiled flat file for {}", resource_file.display())
    }

    /// Link compiled resources into an APK
    pub fn link(
        &self,
        flat_files: &[PathBuf],
        manifest_path: &Path,
        android_jar: &Path,
        output_apk: &Path,
        package_name: Option<&str>,
        version_code: Option<u32>,
        version_name: Option<&str>,
        stable_ids_file: Option<&Path>,
        package_id: Option<&str>,
        min_sdk_version: Option<u32>,
    ) -> Result<LinkResult> {
        debug!("Linking {} flat files", flat_files.len());

        let mut cmd = Command::new(&self.aapt2_path);
        cmd.arg("link")
            .arg("--manifest")
            .arg(manifest_path)
            .arg("-I")
            .arg(android_jar)
            .arg("-o")
            .arg(output_apk)
            .arg("--auto-add-overlay")
            .arg("--no-version-vectors")
            // Keep resource files in the APK (not just resources.arsc)
            .arg("--keep-raw-values")
            // Allow references to resources not defined in this package
            .arg("--allow-reserved-package-id")
            .arg("--no-resource-removal");

        if let Some(pkg) = package_name {
            cmd.arg("--rename-manifest-package").arg(pkg);
        }

        if let Some(code) = version_code {
            cmd.arg("--version-code").arg(code.to_string());
        }

        if let Some(name) = version_name {
            cmd.arg("--version-name").arg(name);
        }

        if let Some(min_sdk) = min_sdk_version {
            cmd.arg("--min-sdk-version").arg(min_sdk.to_string());
        }

        if let Some(stable_ids) = stable_ids_file {
            cmd.arg("--stable-ids").arg(stable_ids);
            cmd.arg("--emit-ids").arg(stable_ids);
        }

        // Set package ID for resource IDs
        // This is critical for dynamic resource loading via new Resources()
        // Default to standard app package ID if not specified
        let pkg_id = package_id.unwrap_or(DEFAULT_PACKAGE_ID);
        cmd.arg("--package-id").arg(pkg_id);

        // Add all flat files
        for flat_file in flat_files {
            cmd.arg(flat_file);
        }

        let output = cmd.output().context("Failed to execute aapt2 link")?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Ok(LinkResult {
                success: false,
                apk_path: None,
                errors: vec![stderr.to_string()],
            });
        }

        Ok(LinkResult {
            success: true,
            apk_path: Some(output_apk.to_path_buf()),
            errors: vec![],
        })
    }

    /// Collect all .flat files from a directory
    fn collect_flat_files(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut flat_files = Vec::new();

        if !dir.exists() {
            return Ok(flat_files);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("flat") {
                flat_files.push(path);
            }
        }

        Ok(flat_files)
    }
}
