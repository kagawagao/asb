use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::info;

/// Module skin package info
#[derive(Debug)]
pub struct ModuleSkinPackage {
    pub module_name: String,
    pub apk_path: PathBuf,
}

/// Utility for merging multiple module skin packages
pub struct SkinMerger;

impl SkinMerger {
    /// Merge multiple module APKs into a single file
    pub fn merge_packages(
        packages: &[ModuleSkinPackage],
        output_path: &Path,
    ) -> Result<()> {
        info!("Merging {} module packages...", packages.len());

        // Create a merged structure
        let mut merged_data = Vec::new();

        // Write header
        let header = format!("ASB_MERGED_V1\n{}\n", packages.len());
        merged_data.extend_from_slice(header.as_bytes());

        // For each module, write: module_name|size|data
        for package in packages {
            let mut apk_data = Vec::new();
            let mut file = File::open(&package.apk_path)
                .with_context(|| format!("Failed to open APK: {}", package.apk_path.display()))?;
            file.read_to_end(&mut apk_data)?;

            // Write module metadata
            let metadata = format!("{}|{}\n", package.module_name, apk_data.len());
            merged_data.extend_from_slice(metadata.as_bytes());

            // Write APK data
            merged_data.extend_from_slice(&apk_data);
        }

        // Write merged file
        let mut output_file = File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
        output_file.write_all(&merged_data)?;

        info!("Merged package created: {}", output_path.display());
        Ok(())
    }

    /// Extract individual modules from a merged package
    pub fn extract_modules(merged_path: &Path, output_dir: &Path) -> Result<Vec<ModuleSkinPackage>> {
        let mut file = File::open(merged_path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        let content_str = String::from_utf8_lossy(&content);
        let mut lines = content_str.lines();

        // Read header
        let header = lines.next().context("Missing header")?;
        if !header.starts_with("ASB_MERGED_V1") {
            anyhow::bail!("Invalid merged package format");
        }

        let count: usize = lines
            .next()
            .context("Missing package count")?
            .parse()
            .context("Invalid package count")?;

        std::fs::create_dir_all(output_dir)?;

        let mut packages = Vec::new();
        let mut offset = header.len() + 1 + count.to_string().len() + 1;

        for _ in 0..count {
            let metadata_line = lines.next().context("Missing module metadata")?;
            let parts: Vec<&str> = metadata_line.split('|').collect();

            if parts.len() != 2 {
                anyhow::bail!("Invalid module metadata format");
            }

            let module_name = parts[0];
            let size: usize = parts[1].parse().context("Invalid module size")?;

            offset += metadata_line.len() + 1;

            // Extract APK data
            let apk_data = &content[offset..offset + size];
            let apk_path = output_dir.join(format!("{}.apk", module_name));

            let mut apk_file = File::create(&apk_path)?;
            apk_file.write_all(apk_data)?;

            packages.push(ModuleSkinPackage {
                module_name: module_name.to_string(),
                apk_path,
            });

            offset += size;
        }

        info!("Extracted {} modules from merged package", packages.len());
        Ok(packages)
    }
}
