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

    /// Get the aapt2 executable path
    pub fn into_path(self) -> PathBuf {
        self.aapt2_path
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
            .with_context(|| {
                format!(
                    "Failed to execute aapt2 compile\n\
                     aapt2 path: {}\n\
                     Resource dir: {}\n\
                     Output dir: {}\n\
                     \nPossible causes:\n\
                     - aapt2 binary not found or not executable\n\
                     - Resource directory does not exist or is not readable\n\
                     - Insufficient permissions to write to output directory",
                    self.aapt2_path.display(),
                    resource_dir.display(),
                    output_dir.display()
                )
            })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
            let mut error_msg = String::new();

            if !stderr.is_empty() {
                error_msg.push_str("aapt2 compile stderr:\n");
                error_msg.push_str(&stderr);
            }

            if !stdout.is_empty() {
                if !error_msg.is_empty() {
                    error_msg.push('\n');
                }
                error_msg.push_str("aapt2 compile stdout:\n");
                error_msg.push_str(&stdout);
            }

            if error_msg.is_empty() {
                error_msg = format!(
                    "aapt2 compile failed with exit code {:?}",
                    output.status.code()
                );
            }

            return Ok(CompileResult {
                success: false,
                flat_files: vec![],
                errors: vec![error_msg],
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
                    .with_context(|| {
                        format!(
                            "Failed to execute aapt2 compile for {}\n\
                             aapt2: {}\n\
                             Output: {}",
                            file.display(),
                            self.aapt2_path.display(),
                            output_dir.display()
                        )
                    })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!(
                        "Failed to compile {}\n\
                         Stderr: {}\n\
                         aapt2: {}",
                        file.display(),
                        stderr,
                        self.aapt2_path.display()
                    );
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
                                vec![format!(
                                    "{}_{}.arsc.flat",
                                    parent_name,
                                    file.file_stem().and_then(|s| s.to_str()).unwrap_or("")
                                )]
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
    #[allow(dead_code)]
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
                        vec![format!(
                            "{}_{}.arsc.flat",
                            parent_name,
                            resource_file
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                        )]
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

        anyhow::bail!(
            "Could not find compiled flat file for {}",
            resource_file.display()
        )
    }

    /// Link compiled resources into an APK with overlay support
    /// Base flat files are linked first, then overlay flat files are applied with -R flag
    /// This implements Android's resource priority strategy where later resources override earlier ones
    pub fn link_with_overlays(
        &self,
        base_flat_files: &[PathBuf],
        overlay_flat_files: &[Vec<PathBuf>], // Vec of overlay sets, ordered by priority
        manifest_path: &Path,
        android_jar: &Path,
        output_apk: &Path,
        package_name: Option<&str>,
        version_code: Option<u32>,
        version_name: Option<&str>,
        stable_ids_file: Option<&Path>,
        package_id: Option<&str>,
        min_sdk_version: Option<u32>,
        compiled_dir: Option<&Path>,  // Optional compiled directory for temp files
    ) -> Result<LinkResult> {
        debug!(
            "Linking {} base flat files with {} overlay sets",
            base_flat_files.len(),
            overlay_flat_files.len()
        );

        self.link_with_command_line(
            base_flat_files,
            overlay_flat_files,
            manifest_path,
            android_jar,
            output_apk,
            package_name,
            version_code,
            version_name,
            stable_ids_file,
            package_id,
            min_sdk_version,
            compiled_dir,
        )
    }

    /// Link using command line arguments
    /// Uses ZIP file for flat files when count exceeds threshold to avoid command line length limits
    fn link_with_command_line(
        &self,
        base_flat_files: &[PathBuf],
        overlay_flat_files: &[Vec<PathBuf>],
        manifest_path: &Path,
        android_jar: &Path,
        output_apk: &Path,
        package_name: Option<&str>,
        version_code: Option<u32>,
        version_name: Option<&str>,
        stable_ids_file: Option<&Path>,
        package_id: Option<&str>,
        min_sdk_version: Option<u32>,
        compiled_dir: Option<&Path>,
    ) -> Result<LinkResult> {
        // Calculate total flat file count
        let total_flat_files = base_flat_files.len() 
            + overlay_flat_files.iter().map(|v| v.len()).sum::<usize>();
        
        // Threshold for using ZIP (to avoid command line length issues)
        // Windows has ~8191 char limit, Unix has ~131072, use conservative threshold
        const USE_ZIP_THRESHOLD: usize = 100;
        
        let use_zip = total_flat_files > USE_ZIP_THRESHOLD;
        
        if use_zip {
            debug!("Using ZIP file for {} flat files (exceeds threshold of {})", 
                   total_flat_files, USE_ZIP_THRESHOLD);
            self.link_with_zip(
                base_flat_files,
                overlay_flat_files,
                manifest_path,
                android_jar,
                output_apk,
                package_name,
                version_code,
                version_name,
                stable_ids_file,
                package_id,
                min_sdk_version,
                compiled_dir,
            )
        } else {
            self.link_with_direct_args(
                base_flat_files,
                overlay_flat_files,
                manifest_path,
                android_jar,
                output_apk,
                package_name,
                version_code,
                version_name,
                stable_ids_file,
                package_id,
                min_sdk_version,
            )
        }
    }
    
    /// Link using ZIP file for flat files
    fn link_with_zip(
        &self,
        base_flat_files: &[PathBuf],
        overlay_flat_files: &[Vec<PathBuf>],
        manifest_path: &Path,
        android_jar: &Path,
        output_apk: &Path,
        package_name: Option<&str>,
        version_code: Option<u32>,
        version_name: Option<&str>,
        stable_ids_file: Option<&Path>,
        package_id: Option<&str>,
        min_sdk_version: Option<u32>,
        compiled_dir: Option<&Path>,
    ) -> Result<LinkResult> {
        use std::fs::File;
        use zip::write::{FileOptions, ZipWriter};
        
        // Create temporary directory for ZIP files
        // Use compiled directory if provided to avoid conflicts in multi-task builds
        let temp_dir = if let Some(compiled) = compiled_dir {
            compiled.join(".temp_zip")
        } else {
            output_apk.parent().unwrap().join(".temp_zip")
        };
        std::fs::create_dir_all(&temp_dir)?;
        
        // Create ZIP file for base flat files
        let base_zip = temp_dir.join("base_flats.zip");
        let base_file = File::create(&base_zip)?;
        let mut base_zip_writer = ZipWriter::new(base_file);
        
        for flat_file in base_flat_files {
            let file_name = flat_file.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid flat file name"))?;
            base_zip_writer.start_file::<_, ()>(file_name, FileOptions::default())?;
            let content = std::fs::read(flat_file)?;
            std::io::Write::write_all(&mut base_zip_writer, &content)?;
        }
        base_zip_writer.finish()?;
        
        // Create ZIP files for overlay flat files
        let mut overlay_zips = Vec::new();
        for (idx, overlay_set) in overlay_flat_files.iter().enumerate() {
            if overlay_set.is_empty() {
                continue;
            }
            
            let overlay_zip = temp_dir.join(format!("overlay_{}.zip", idx));
            let overlay_file = File::create(&overlay_zip)?;
            let mut overlay_zip_writer = ZipWriter::new(overlay_file);
            
            for flat_file in overlay_set {
                let file_name = flat_file.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| anyhow::anyhow!("Invalid flat file name"))?;
                overlay_zip_writer.start_file::<_, ()>(file_name, FileOptions::default())?;
                let content = std::fs::read(flat_file)?;
                std::io::Write::write_all(&mut overlay_zip_writer, &content)?;
            }
            overlay_zip_writer.finish()?;
            overlay_zips.push(overlay_zip);
        }
        
        // Build command with ZIP files
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
            .arg("--keep-raw-values")
            .arg("--allow-reserved-package-id")
            .arg("--no-resource-removal");

        if let Some(pkg) = package_name {
            cmd.arg("--rename-manifest-package").arg(pkg);
            cmd.arg("--rename-resources-package").arg(pkg);
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

        let pkg_id = package_id.unwrap_or(DEFAULT_PACKAGE_ID);
        cmd.arg("--package-id").arg(pkg_id);

        // Add base ZIP file
        cmd.arg(&base_zip);

        // Add overlay ZIP files with -R flag
        for overlay_zip in &overlay_zips {
            cmd.arg("-R").arg(overlay_zip);
        }

        debug!("Executing aapt2 link with ZIP files: {:?}", cmd);

        let output = cmd.output().with_context(|| {
            format!(
                "Failed to execute aapt2 link with ZIP files\n\
                 aapt2 path: {}\n\
                 Manifest: {}\n\
                 Android JAR: {}\n\
                 Output: {}",
                self.aapt2_path.display(),
                manifest_path.display(),
                android_jar.display(),
                output_apk.display()
            )
        })?;

        // Cleanup temporary ZIP files
        std::fs::remove_dir_all(&temp_dir).ok();

        self.process_link_output(
            output,
            manifest_path,
            android_jar,
            output_apk,
            package_name,
            version_code,
            version_name,
            stable_ids_file,
            package_id,
            base_flat_files,
            overlay_flat_files,
            min_sdk_version,
        )
    }
    
    /// Link using direct command line arguments (original method)
    fn link_with_direct_args(
        &self,
        base_flat_files: &[PathBuf],
        overlay_flat_files: &[Vec<PathBuf>],
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
            cmd.arg("--rename-resources-package").arg(pkg);
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

        // Add base flat files (normal arguments)
        for flat_file in base_flat_files {
            cmd.arg(flat_file);
        }

        // Add overlay flat files with -R flag for each set
        // The order matters: later overlays override earlier ones
        for overlay_set in overlay_flat_files {
            for flat_file in overlay_set {
                cmd.arg("-R").arg(flat_file);
            }
        }

        // Debug: print the full command for troubleshooting
        debug!("Executing aapt2 link command: {:?}", cmd);

        let output = cmd.output().with_context(|| {
            format!(
                "Failed to execute aapt2 link\n\
                 aapt2 path: {}\n\
                 Manifest: {}\n\
                 Android JAR: {}\n\
                 Output: {}\n\
                 Base files: {}\n\
                 Overlay sets: {}\n\
                 \nPossible causes:\n\
                 - aapt2 binary not found or not executable\n\
                 - Manifest file is invalid or corrupted\n\
                 - Android JAR path is incorrect\n\
                 - Insufficient permissions to write output file",
                self.aapt2_path.display(),
                manifest_path.display(),
                android_jar.display(),
                output_apk.display(),
                base_flat_files.len(),
                overlay_flat_files.len()
            )
        })?;

        self.process_link_output(
            output,
            manifest_path,
            android_jar,
            output_apk,
            package_name,
            version_code,
            version_name,
            stable_ids_file,
            package_id,
            base_flat_files,
            overlay_flat_files,
            min_sdk_version,
        )
    }

    /// Process the output from aapt2 link command
    fn process_link_output(
        &self,
        output: std::process::Output,
        manifest_path: &Path,
        android_jar: &Path,
        output_apk: &Path,
        package_name: Option<&str>,
        version_code: Option<u32>,
        version_name: Option<&str>,
        stable_ids_file: Option<&Path>,
        package_id: Option<&str>,
        base_flat_files: &[PathBuf],
        overlay_flat_files: &[Vec<PathBuf>],
        min_sdk_version: Option<u32>,
    ) -> Result<LinkResult> {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            // Construct detailed error message - prioritize actual aapt2 output
            let mut error_msg = String::new();

            // Show the actual command that was executed for manual debugging
            error_msg.push_str("Failed aapt2 command:\n");
            error_msg.push_str(&format!("  {}\n", self.aapt2_path.display()));
            error_msg.push_str(&format!("    link --manifest {} -I {} -o {} --auto-add-overlay --no-version-vectors --keep-raw-values --allow-reserved-package-id --no-resource-removal",
                manifest_path.display(),
                android_jar.display(),
                output_apk.display()));
            if let Some(pkg) = package_name {
                error_msg.push_str(&format!(
                    " --rename-manifest-package {} --rename-resources-package {}",
                    pkg, pkg
                ));
            }
            if let Some(code) = version_code {
                error_msg.push_str(&format!(" --version-code {}", code));
            }
            if let Some(name) = version_name {
                error_msg.push_str(&format!(" --version-name {}", name));
            }
            if let Some(min_sdk) = min_sdk_version {
                error_msg.push_str(&format!(" --min-sdk-version {}", min_sdk));
            }
            if let Some(stable_ids) = stable_ids_file {
                error_msg.push_str(&format!(
                    " --stable-ids {} --emit-ids {}",
                    stable_ids.display(),
                    stable_ids.display()
                ));
            }
            error_msg.push_str(&format!(
                " --package-id {}",
                package_id.unwrap_or(DEFAULT_PACKAGE_ID)
            ));

            // Add file counts instead of listing all files
            error_msg.push_str(&format!(" [{}  base flat files]", base_flat_files.len()));
            for (i, overlay_set) in overlay_flat_files.iter().enumerate() {
                error_msg.push_str(&format!(
                    " [-R {} overlay files (set {})]",
                    overlay_set.len(),
                    i + 1
                ));
            }
            error_msg.push_str("\n\n");

            // Show actual error output
            if !stderr.is_empty() {
                error_msg.push_str("aapt2 stderr:\n");
                error_msg.push_str(&stderr);
                error_msg.push('\n');
            }

            if !stdout.is_empty() {
                error_msg.push_str("\naapt2 stdout:\n");
                error_msg.push_str(&stdout);
                error_msg.push('\n');
            }

            // Then show context
            error_msg.push_str(&format!("\nExit code: {:?}\n", output.status.code()));
            error_msg.push_str("\nCommand context:\n");
            error_msg.push_str(&format!("  aapt2: {}\n", self.aapt2_path.display()));
            error_msg.push_str(&format!("  Manifest: {}\n", manifest_path.display()));
            error_msg.push_str(&format!("  Android JAR: {}\n", android_jar.display()));
            error_msg.push_str(&format!("  Output APK: {}\n", output_apk.display()));
            error_msg.push_str(&format!("  Base flat files: {}\n", base_flat_files.len()));
            error_msg.push_str(&format!("  Overlay sets: {}\n", overlay_flat_files.len()));
            if let Some(pkg) = package_name {
                error_msg.push_str(&format!("  Package: {}\n", pkg));
            }

            return Ok(LinkResult {
                success: false,
                apk_path: None,
                errors: vec![error_msg],
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
