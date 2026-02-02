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

        // Validate module names to prevent injection attacks
        for package in packages {
            if package.module_name.contains('\n')
                || package.module_name.contains('|')
                || package.module_name.contains('\r')
            {
                anyhow::bail!(
                    "Invalid module name '{}': cannot contain newline or pipe characters",
                    package.module_name
                );
            }
        }

        // Create a merged structure
        let mut merged_data = Vec::new();

        // Write header
        let header = format!("ASB_MERGED_V1\n{}\n", packages.len());
        merged_data.extend_from_slice(header.as_bytes());

        // For each module, write: module_name|size|data
        for package in packages {
            let mut apk_data = Vec::new();
            let mut file = File::open(&package.apk_path)
                .with_context(|| format!("Failed to open skin package: {}", package.apk_path.display()))?;
            file.read_to_end(&mut apk_data)?;

            // Write module metadata
            let metadata = format!("{}|{}\n", package.module_name, apk_data.len());
            merged_data.extend_from_slice(metadata.as_bytes());

            // Write skin package data
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

        // Read header line (text)
        let mut offset = 0;
        let header_end = content.iter()
            .position(|&b| b == b'\n')
            .context("Missing header line")?;
        let header = std::str::from_utf8(&content[..header_end])?;
        
        if !header.starts_with("ASB_MERGED_V1") {
            anyhow::bail!("Invalid merged package format");
        }
        offset = header_end + 1;

        // Read count line (text)
        let count_end = content[offset..]
            .iter()
            .position(|&b| b == b'\n')
            .context("Missing count line")?;
        let count_str = std::str::from_utf8(&content[offset..offset + count_end])?;
        let count: usize = count_str.parse().context("Invalid package count")?;
        offset += count_end + 1;

        std::fs::create_dir_all(output_dir)?;

        let mut packages = Vec::new();

        for _ in 0..count {
            // Read metadata line (text)
            let metadata_end = content[offset..]
                .iter()
                .position(|&b| b == b'\n')
                .context("Missing module metadata")?;
            let metadata = std::str::from_utf8(&content[offset..offset + metadata_end])?;
            let parts: Vec<&str> = metadata.split('|').collect();

            if parts.len() != 2 {
                anyhow::bail!("Invalid module metadata format");
            }

            let module_name = parts[0];
            let size: usize = parts[1].parse().context("Invalid module size")?;
            offset += metadata_end + 1;

            // Extract skin package data (binary)
            if offset + size > content.len() {
                anyhow::bail!("Invalid skin package data size");
            }
            let apk_data = &content[offset..offset + size];
            let apk_path = output_dir.join(format!("{}.skin", module_name));

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
