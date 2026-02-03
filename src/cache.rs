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
    pub fn clear(&mut self) -> Result<()> {
        self.cache.entries.clear();
        if self.cache_file.exists() {
            std::fs::remove_file(&self.cache_file)?;
        }
        Ok(())
    }

    /// Get all cached flat files
    pub fn get_all_cached_flat_files(&self) -> Vec<PathBuf> {
        self.cache
            .entries
            .values()
            .map(|e| e.flat_file.clone())
            .collect()
    }
}

/// Cache entry for compiled common dependencies
#[derive(Debug, Serialize, Deserialize)]
struct CommonDepCacheEntry {
    /// Resource directory path
    resource_dir: PathBuf,
    /// Hash of all files in the resource directory
    directory_hash: String,
    /// Timestamp of when this was cached
    timestamp: u64,
    /// Paths to all compiled flat files for this dependency
    flat_files: Vec<PathBuf>,
}

/// Cache data for common dependencies
#[derive(Debug, Serialize, Deserialize)]
struct CommonDepCacheData {
    version: String,
    entries: HashMap<PathBuf, CommonDepCacheEntry>,
}

/// Cache for managing compiled common dependencies
pub struct CommonDependencyCache {
    cache_dir: PathBuf,
    cache_file: PathBuf,
    cache: CommonDepCacheData,
}

impl CommonDependencyCache {
    /// Create a new common dependency cache
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let cache_file = cache_dir.join("common-dep-cache.json");

        let cache = if cache_file.exists() {
            match std::fs::read_to_string(&cache_file) {
                Ok(content) => match serde_json::from_str::<CommonDepCacheData>(&content) {
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

    fn empty_cache() -> CommonDepCacheData {
        CommonDepCacheData {
            version: "1.0".to_string(),
            entries: HashMap::new(),
        }
    }

    /// Initialize cache directory
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    /// Calculate hash of all files in a directory
    fn calculate_directory_hash(dir_path: &Path) -> Result<String> {
        use walkdir::WalkDir;
        
        let mut hasher = Sha256::new();
        let mut files: Vec<PathBuf> = WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();
        
        // Sort to ensure consistent hashing
        files.sort();
        
        for file in files {
            if let Ok(content) = std::fs::read(&file) {
                // Hash file path relative to dir_path
                if let Ok(rel_path) = file.strip_prefix(dir_path) {
                    hasher.update(rel_path.to_string_lossy().as_bytes());
                }
                // Hash file content
                hasher.update(&content);
            }
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Check if a common dependency needs recompilation
    pub fn needs_recompile(&self, resource_dir: &Path) -> Result<bool> {
        let entry = self.cache.entries.get(resource_dir);

        if entry.is_none() {
            return Ok(true);
        }

        let entry = entry.unwrap();

        // Check if all flat files still exist
        for flat_file in &entry.flat_files {
            if !flat_file.exists() {
                return Ok(true);
            }
        }

        // Check if directory has been modified
        let current_hash = Self::calculate_directory_hash(resource_dir)?;
        Ok(current_hash != entry.directory_hash)
    }

    /// Get cached flat files for a common dependency
    pub fn get_cached_flat_files(&self, resource_dir: &Path) -> Option<Vec<PathBuf>> {
        self.cache
            .entries
            .get(resource_dir)
            .map(|e| e.flat_files.clone())
    }

    /// Update cache entry for a common dependency
    pub fn update_entry(&mut self, resource_dir: &Path, flat_files: Vec<PathBuf>) -> Result<()> {
        let directory_hash = Self::calculate_directory_hash(resource_dir)?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.cache.entries.insert(
            resource_dir.to_path_buf(),
            CommonDepCacheEntry {
                resource_dir: resource_dir.to_path_buf(),
                directory_hash,
                timestamp,
                flat_files,
            },
        );

        Ok(())
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.cache)?;
        std::fs::write(&self.cache_file, content)?;
        debug!("Common dependency cache saved to: {}", self.cache_file.display());
        Ok(())
    }

    /// Clear cache
    pub fn clear(&mut self) -> Result<()> {
        self.cache.entries.clear();
        if self.cache_file.exists() {
            std::fs::remove_file(&self.cache_file)?;
        }
        Ok(())
    }
}
