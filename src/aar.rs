use anyhow::{Context, Result};
use std::fs::File;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::types::AarInfo;

/// Utility for handling AAR files
pub struct AarExtractor;

impl AarExtractor {
    /// Extract AAR file to a directory
    pub fn extract_aar(aar_path: &Path, extract_dir: &Path) -> Result<AarInfo> {
        if !aar_path.exists() {
            anyhow::bail!("AAR file not found: {}", aar_path.display());
        }

        std::fs::create_dir_all(extract_dir)?;

        debug!("Extracting AAR: {} to {}", aar_path.display(), extract_dir.display());

        // Extract ZIP
        let file = File::open(aar_path)
            .with_context(|| format!("Failed to open AAR file: {}", aar_path.display()))?;
        let mut archive = zip::ZipArchive::new(file)
            .with_context(|| format!("Failed to read AAR as ZIP: {}", aar_path.display()))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = extract_dir.join(file.name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p)?;
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        // Find resource directory and manifest
        let res_dir = extract_dir.join("res");
        let manifest_path = extract_dir.join("AndroidManifest.xml");

        Ok(AarInfo {
            path: aar_path.to_path_buf(),
            resource_dir: if res_dir.exists() { Some(res_dir) } else { None },
            manifest_path: if manifest_path.exists() {
                Some(manifest_path)
            } else {
                None
            },
            extracted_dir: extract_dir.to_path_buf(),
        })
    }

    /// Extract multiple AAR files
    pub fn extract_aars(aar_paths: &[PathBuf], base_temp_dir: &Path) -> Result<Vec<AarInfo>> {
        let mut aar_infos = Vec::new();

        for (i, aar_path) in aar_paths.iter().enumerate() {
            let aar_name = aar_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let extract_dir = base_temp_dir.join(format!("aar_{}_{}", i, aar_name));

            let info = Self::extract_aar(aar_path, &extract_dir)?;
            aar_infos.push(info);
        }

        Ok(aar_infos)
    }

    /// Clean up extracted AAR directories
    pub fn cleanup_aars(aar_infos: &[AarInfo]) -> Result<()> {
        for info in aar_infos {
            if info.extracted_dir.exists() {
                std::fs::remove_dir_all(&info.extracted_dir).ok();
            }
        }
        Ok(())
    }
}
