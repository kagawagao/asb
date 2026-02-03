use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    hash: String,
    timestamp: u64,
    flat_file: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheData {
    version: String,
    entries: HashMap<PathBuf, CacheEntry>,
}

/// Utility for managing build cache for incremental builds
pub struct BuildCache {
    cache_dir: PathBuf,
    cache_file: PathBuf,
    cache: CacheData,
}

impl BuildCache {
    /// Create a new build cache
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let cache_file = cache_dir.join("build-cache.json");

        let cache = if cache_file.exists() {
            match std::fs::read_to_string(&cache_file) {
                Ok(content) => match serde_json::from_str::<CacheData>(&content) {
                    Ok(data) if data.version == "1.0" => data,
                    _ => Self::empty_cache(),
                },
                Err(_) => Self::empty_cache(),
            }
        } else {
            Self::empty_cache()
        };

        Ok(Self {
            cache_dir,
            cache_file,
            cache,
        })
    }

    fn empty_cache() -> CacheData {
        CacheData {
            version: "1.0".to_string(),
            entries: HashMap::new(),
        }
    }

    /// Initialize cache directory
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    /// Calculate file hash
    fn calculate_hash(file_path: &Path) -> Result<String> {
        let content = std::fs::read(file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Check if a file needs recompilation
    pub fn needs_recompile(&self, resource_file: &Path) -> Result<bool> {
        let entry = self.cache.entries.get(resource_file);

        if entry.is_none() {
            return Ok(true);
        }

        let entry = entry.unwrap();

        // Check if flat file still exists
        if !entry.flat_file.exists() {
            return Ok(true);
        }

        // Check if file has been modified
        let current_hash = Self::calculate_hash(resource_file)?;
        Ok(current_hash != entry.hash)
    }

    /// Get cached flat file for a resource
    pub fn get_cached_flat_file(&self, resource_file: &Path) -> Option<PathBuf> {
        self.cache
            .entries
            .get(resource_file)
            .map(|e| e.flat_file.clone())
    }

    /// Update cache entry
    pub fn update_entry(&mut self, resource_file: &Path, flat_file: &Path) -> Result<()> {
        let hash = Self::calculate_hash(resource_file)?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.cache.entries.insert(
            resource_file.to_path_buf(),
            CacheEntry {
                hash,
                timestamp,
                flat_file: flat_file.to_path_buf(),
            },
        );

        Ok(())
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.cache)?;
        std::fs::write(&self.cache_file, content)?;
        debug!("Cache saved to: {}", self.cache_file.display());
        Ok(())
    }

    /// Clear cache
    #[allow(dead_code)]
    pub fn clear(&mut self) -> Result<()> {
        self.cache.entries.clear();
        if self.cache_file.exists() {
            std::fs::remove_file(&self.cache_file)?;
        }
        Ok(())
    }

    /// Get all cached flat files
    #[allow(dead_code)]
    pub fn get_all_cached_flat_files(&self) -> Vec<PathBuf> {
        self.cache
            .entries
            .values()
            .map(|e| e.flat_file.clone())
            .collect()
    }
}
