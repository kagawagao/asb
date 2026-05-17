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
    /// In-memory cache of recently computed hashes to avoid recomputing them
    /// when updating entries after a needs_recompile check.
    pending_hashes: HashMap<PathBuf, String>,
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
            pending_hashes: HashMap::new(),
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
    pub fn needs_recompile(&mut self, resource_file: &Path) -> Result<bool> {
        let entry = self.cache.entries.get(resource_file);

        if entry.is_none() {
            return Ok(true);
        }

        let entry = entry.unwrap();

        // Check if flat file still exists
        if !entry.flat_file.exists() {
            return Ok(true);
        }

        // Check if file has been modified; cache the hash to reuse in update_entry
        let current_hash = Self::calculate_hash(resource_file)?;
        if current_hash != entry.hash {
            self.pending_hashes
                .insert(resource_file.to_path_buf(), current_hash);
            Ok(true)
        } else {
            // Cache the hash even when unchanged so update_entry avoids recomputing it
            self.pending_hashes
                .insert(resource_file.to_path_buf(), current_hash);
            Ok(false)
        }
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
        // Reuse the hash computed during needs_recompile if available
        let hash = match self.pending_hashes.remove(resource_file) {
            Some(h) => h,
            None => Self::calculate_hash(resource_file)?,
        };
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
        self.pending_hashes.clear();
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
    #[allow(dead_code)]
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
        debug!(
            "Common dependency cache saved to: {}",
            self.cache_file.display()
        );
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Helper: create a temp file with given content
    fn create_temp_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    // ==================== BuildCache Tests ====================

    #[test]
    fn test_build_cache_new_empty() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let cache = BuildCache::new(cache_dir.clone()).unwrap();
        let flats = cache.get_all_cached_flat_files();
        assert!(flats.is_empty(), "New empty cache should have no entries");
    }

    #[test]
    fn test_build_cache_init_creates_dir() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        assert!(!cache_dir.exists(), "Cache dir should not exist yet");
        let cache = BuildCache::new(cache_dir.clone()).unwrap();
        cache.init().unwrap();
        assert!(cache_dir.exists(), "Cache dir should be created by init()");
    }

    #[test]
    fn test_build_cache_needs_recompile_no_entry() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = BuildCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let file = create_temp_file(tmp.path(), "test.xml", b"<resources/>");
        let result = cache.needs_recompile(&file).unwrap();
        assert!(result, "No entry should mean recompile is needed");
    }

    #[test]
    fn test_build_cache_get_cached_flat_file_none() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let cache = BuildCache::new(cache_dir).unwrap();
        let file = create_temp_file(tmp.path(), "fake.xml", b"fake");
        let result = cache.get_cached_flat_file(&file);
        assert!(result.is_none(), "No entry should return None");
    }

    #[test]
    fn test_build_cache_update_and_get_entry() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = BuildCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let source = create_temp_file(tmp.path(), "colors.xml", b"<resources><color name=\"x\">#fff</color></resources>");
        let flat = create_temp_file(tmp.path(), "colors.xml.flat", b"flat_data");

        cache.update_entry(&source, &flat).unwrap();
        let cached = cache.get_cached_flat_file(&source);
        assert_eq!(cached, Some(flat.clone()), "Should return the flat file path");
    }

    #[test]
    fn test_build_cache_needs_recompile_unchanged_file() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = BuildCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let source = create_temp_file(tmp.path(), "drawable.xml", b"<selector/>");
        let flat = create_temp_file(tmp.path(), "drawable.xml.flat", b"flat");

        cache.update_entry(&source, &flat).unwrap();

        // File hasn't changed; should not need recompile
        let result = cache.needs_recompile(&source).unwrap();
        assert!(!result, "Unchanged file should not need recompile");
    }

    #[test]
    fn test_build_cache_needs_recompile_changed_file() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = BuildCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let source = create_temp_file(tmp.path(), "values.xml", b"v1");
        let flat = create_temp_file(tmp.path(), "values.xml.flat", b"flat_v1");

        cache.update_entry(&source, &flat).unwrap();

        // Modify the source file
        fs::write(&source, b"v2").unwrap();

        // Should now need recompile
        let result = cache.needs_recompile(&source).unwrap();
        assert!(result, "Changed file should need recompile");
    }

    #[test]
    fn test_build_cache_needs_recompile_missing_flat() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = BuildCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let source = create_temp_file(tmp.path(), "temp.xml", b"<resources/>");
        let flat = tmp.path().join("missing.flat");

        cache.update_entry(&source, &flat).unwrap();

        // Flat file was never actually created, so it should need recompile
        let result = cache.needs_recompile(&source).unwrap();
        assert!(result, "Missing flat file should force recompile");
    }

    #[test]
    fn test_build_cache_save_and_reload() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let source = create_temp_file(tmp.path(), "a.xml", b"content A");
        let flat = create_temp_file(tmp.path(), "a.flat", b"flat A");

        // Create, populate, and save cache
        {
            let mut cache = BuildCache::new(cache_dir.clone()).unwrap();
            cache.init().unwrap();
            cache.update_entry(&source, &flat).unwrap();
            cache.save().unwrap();
        }

        // Reload cache from disk -> should have the entry
        let cache2 = BuildCache::new(cache_dir.clone()).unwrap();
        let cached = cache2.get_cached_flat_file(&source);
        assert_eq!(cached, Some(flat.clone()), "Reloaded cache should have entry");

        // Unchanged file -> no recompile
        let mut cache3 = BuildCache::new(cache_dir).unwrap();
        let needs = cache3.needs_recompile(&source).unwrap();
        assert!(!needs, "Reloaded cache + unchanged file -> no recompile");
    }

    #[test]
    fn test_build_cache_clear() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = BuildCache::new(cache_dir.clone()).unwrap();
        cache.init().unwrap();

        let source = create_temp_file(tmp.path(), "x.xml", b"x");
        let flat = create_temp_file(tmp.path(), "x.flat", b"fx");
        cache.update_entry(&source, &flat).unwrap();
        cache.save().unwrap();

        // Verify cache file exists
        let cache_file = cache_dir.join("build-cache.json");
        assert!(cache_file.exists(), "Cache file should exist before clear");

        cache.clear().unwrap();

        assert!(cache.get_all_cached_flat_files().is_empty(), "Entries should be cleared");
        assert!(!cache_file.exists(), "Cache file should be removed");
    }

    #[test]
    fn test_build_cache_get_all_cached_flat_files() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = BuildCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let s1 = create_temp_file(tmp.path(), "a.xml", b"a");
        let s2 = create_temp_file(tmp.path(), "b.xml", b"b");
        let f1 = create_temp_file(tmp.path(), "a.flat", b"fa");
        let f2 = create_temp_file(tmp.path(), "b.flat", b"fb");

        cache.update_entry(&s1, &f1).unwrap();
        cache.update_entry(&s2, &f2).unwrap();

        let all = cache.get_all_cached_flat_files();
        assert_eq!(all.len(), 2, "Should return 2 cached flat files");
        assert!(all.contains(&f1));
        assert!(all.contains(&f2));
    }

    #[test]
    fn test_build_cache_corrupted_file() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        // Write invalid JSON to cache file
        let cache_file = cache_dir.join("build-cache.json");
        fs::write(&cache_file, b"not valid json {{{").unwrap();

        // Should not panic, should start with empty cache
        let cache = BuildCache::new(cache_dir).unwrap();
        assert!(cache.get_all_cached_flat_files().is_empty(),
            "Corrupted cache file should result in empty cache");
    }

    #[test]
    fn test_build_cache_wrong_version() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        // Valid JSON but wrong version
        let bad_version = r#"{"version":"0.5","entries":{}}"#;
        let cache_file = cache_dir.join("build-cache.json");
        fs::write(&cache_file, bad_version).unwrap();

        let cache = BuildCache::new(cache_dir).unwrap();
        assert!(cache.get_all_cached_flat_files().is_empty(),
            "Wrong version should result in empty cache");
    }

    #[test]
    fn test_build_cache_hash_consistency() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = BuildCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let source = create_temp_file(tmp.path(), "same.xml", b"same content");

        // Update entry
        let flat = create_temp_file(tmp.path(), "same.flat", b"flat");
        cache.update_entry(&source, &flat).unwrap();

        // Same content -> no recompile
        let needs = cache.needs_recompile(&source).unwrap();
        assert!(!needs, "Hash should be consistent for same content");

        // Different content -> recompile
        fs::write(&source, b"different content").unwrap();
        let needs2 = cache.needs_recompile(&source).unwrap();
        assert!(needs2, "Different content should produce different hash");
    }

    // ==================== CommonDependencyCache Tests ====================

    #[test]
    fn test_common_dep_cache_new_empty() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let cache = CommonDependencyCache::new(cache_dir).unwrap();
        let result = cache.get_cached_flat_files(&PathBuf::from("/nonexistent"));
        assert!(result.is_none(), "Empty cache should return None");
    }

    #[test]
    fn test_common_dep_cache_init_creates_dir() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        assert!(!cache_dir.exists());
        let cache = CommonDependencyCache::new(cache_dir.clone()).unwrap();
        cache.init().unwrap();
        assert!(cache_dir.exists(), "init() should create directory");
    }

    #[test]
    fn test_common_dep_cache_needs_recompile_no_entry() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let cache = CommonDependencyCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let res_dir = tmp.path().join("res");
        fs::create_dir_all(&res_dir).unwrap();
        create_temp_file(&res_dir, "colors.xml", b"<resources/>");

        let result = cache.needs_recompile(&res_dir).unwrap();
        assert!(result, "No entry should mean recompile needed");
    }

    #[test]
    fn test_common_dep_cache_update_and_get() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let res_dir = tmp.path().join("res");
        fs::create_dir_all(&res_dir).unwrap();
        create_temp_file(&res_dir, "colors.xml", b"<resources><color name=\"x\">#fff</color></resources>");
        create_temp_file(&res_dir, "dimens.xml", b"<resources><dimen name=\"y\">10dp</dimen></resources>");

        let flat_files = vec![
            tmp.path().join("colors.flat"),
            tmp.path().join("dimens.flat"),
        ];
        // Create the flat files so they exist
        for f in &flat_files {
            fs::write(f, b"flat").unwrap();
        }

        cache.update_entry(&res_dir, flat_files.clone()).unwrap();

        let cached = cache.get_cached_flat_files(&res_dir);
        assert_eq!(cached, Some(flat_files.clone()), "Should return cached flat files");
    }

    #[test]
    fn test_common_dep_cache_needs_recompile_unchanged() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let res_dir = tmp.path().join("res");
        fs::create_dir_all(&res_dir).unwrap();
        create_temp_file(&res_dir, "values.xml", b"<resources/>");

        let flat = tmp.path().join("values.flat");
        fs::write(&flat, b"flat").unwrap();

        cache.update_entry(&res_dir, vec![flat.clone()]).unwrap();

        // Directory unchanged -> no recompile
        let result = cache.needs_recompile(&res_dir).unwrap();
        assert!(!result, "Unchanged directory should not need recompile");
    }

    #[test]
    fn test_common_dep_cache_needs_recompile_changed() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let res_dir = tmp.path().join("res");
        fs::create_dir_all(&res_dir).unwrap();
        create_temp_file(&res_dir, "values.xml", b"v1");

        let flat = tmp.path().join("values.flat");
        fs::write(&flat, b"flat_v1").unwrap();

        cache.update_entry(&res_dir, vec![flat.clone()]).unwrap();

        // Change a file in the directory
        fs::write(res_dir.join("values.xml"), b"v2").unwrap();

        let result = cache.needs_recompile(&res_dir).unwrap();
        assert!(result, "Changed directory should need recompile");
    }

    #[test]
    fn test_common_dep_cache_needs_recompile_missing_flat() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let res_dir = tmp.path().join("res");
        fs::create_dir_all(&res_dir).unwrap();
        create_temp_file(&res_dir, "dimens.xml", b"<resources/>");

        let missing_flat = tmp.path().join("missing.flat");
        // NOT creating missing_flat

        cache.update_entry(&res_dir, vec![missing_flat]).unwrap();

        let result = cache.needs_recompile(&res_dir).unwrap();
        assert!(result, "Missing flat file should force recompile");
    }

    #[test]
    fn test_common_dep_cache_save_and_reload() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");

        let res_dir = tmp.path().join("res");
        fs::create_dir_all(&res_dir).unwrap();
        create_temp_file(&res_dir, "a.xml", b"<resources/>");

        let flat = tmp.path().join("a.flat");
        fs::write(&flat, b"flat_a").unwrap();

        // Create, populate, save
        {
            let mut cache = CommonDependencyCache::new(cache_dir.clone()).unwrap();
            cache.init().unwrap();
            cache.update_entry(&res_dir, vec![flat.clone()]).unwrap();
            cache.save().unwrap();
        }

        // Reload
        let cache2 = CommonDependencyCache::new(cache_dir.clone()).unwrap();
        let cached = cache2.get_cached_flat_files(&res_dir);
        assert_eq!(cached, Some(vec![flat.clone()]), "Reloaded cache should have entry");

        // Unchanged -> no recompile
        let needs = cache2.needs_recompile(&res_dir).unwrap();
        assert!(!needs, "Reloaded + unchanged -> no recompile");
    }

    #[test]
    fn test_common_dep_cache_clear() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = CommonDependencyCache::new(cache_dir.clone()).unwrap();
        cache.init().unwrap();

        let res_dir = tmp.path().join("res");
        fs::create_dir_all(&res_dir).unwrap();
        create_temp_file(&res_dir, "x.xml", b"x");

        let flat = tmp.path().join("x.flat");
        fs::write(&flat, b"fx").unwrap();

        cache.update_entry(&res_dir, vec![flat]).unwrap();
        cache.save().unwrap();

        let cache_file = cache_dir.join("common-dep-cache.json");
        assert!(cache_file.exists(), "Cache file should exist before clear");

        cache.clear().unwrap();

        assert!(
            cache.get_cached_flat_files(&res_dir).is_none(),
            "Entries should be cleared"
        );
        assert!(!cache_file.exists(), "Cache file should be removed after clear");
    }

    #[test]
    fn test_common_dep_cache_corrupted_file() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let cache_file = cache_dir.join("common-dep-cache.json");
        fs::write(&cache_file, b"this is not json {{{{{{").unwrap();

        let cache = CommonDependencyCache::new(cache_dir).unwrap();
        let result = cache.get_cached_flat_files(&PathBuf::from("/nonexistent"));
        assert!(result.is_none(), "Corrupted file should result in empty cache");
    }

    #[test]
    fn test_common_dep_cache_wrong_version() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let bad_version = r#"{"version":"0.1","entries":{}}"#;
        let cache_file = cache_dir.join("common-dep-cache.json");
        fs::write(&cache_file, bad_version).unwrap();

        let cache = CommonDependencyCache::new(cache_dir).unwrap();
        let result = cache.get_cached_flat_files(&PathBuf::from("/nonexistent"));
        assert!(result.is_none(), "Wrong version should result in empty cache");
    }

    #[test]
    fn test_common_dep_cache_directory_hash_consistency() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");

        let res_dir_a = tmp.path().join("res_a");
        let res_dir_b = tmp.path().join("res_b");

        fs::create_dir_all(&res_dir_a).unwrap();
        fs::create_dir_all(&res_dir_b).unwrap();

        // Same content, same file names
        create_temp_file(&res_dir_a, "colors.xml", b"<resources><color name=\"x\">#fff</color></resources>");
        create_temp_file(&res_dir_b, "colors.xml", b"<resources><color name=\"x\">#fff</color></resources>");

        let flat_a = tmp.path().join("a.flat");
        let flat_b = tmp.path().join("b.flat");
        fs::write(&flat_a, b"fa").unwrap();
        fs::write(&flat_b, b"fb").unwrap();

        let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
        cache.init().unwrap();
        cache.update_entry(&res_dir_a, vec![flat_a]).unwrap();
        cache.update_entry(&res_dir_b, vec![flat_b]).unwrap();

        // Both dirs with same content should have same hash -> neither needs recompile
        let needs_a = cache.needs_recompile(&res_dir_a).unwrap();
        let needs_b = cache.needs_recompile(&res_dir_b).unwrap();
        assert!(!needs_a, "Dir A unchanged -> no recompile");
        assert!(!needs_b, "Dir B unchanged -> no recompile");

        // Change a file in res_dir_a
        fs::write(res_dir_a.join("colors.xml"), b"<resources><color name=\"x\">#000</color></resources>").unwrap();

        let needs_a2 = cache.needs_recompile(&res_dir_a).unwrap();
        assert!(needs_a2, "Changed dir should need recompile");

        // res_dir_b still unchanged
        let needs_b2 = cache.needs_recompile(&res_dir_b).unwrap();
        assert!(!needs_b2, "Unchanged dir should still not need recompile");
    }

    #[test]
    fn test_common_dep_cache_empty_directory() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
        cache.init().unwrap();

        let empty_dir = tmp.path().join("empty");
        fs::create_dir_all(&empty_dir).unwrap();

        let flat = tmp.path().join("e.flat");
        fs::write(&flat, b"fe").unwrap();

        cache.update_entry(&empty_dir, vec![flat.clone()]).unwrap();

        // Empty directory should have a consistent hash
        // Unchanged empty dir -> no recompile
        let needs = cache.needs_recompile(&empty_dir).unwrap();
        assert!(!needs, "Unchanged empty dir should not need recompile");

        // Add a file -> should need recompile
        create_temp_file(&empty_dir, "new.xml", b"<resources/>");
        let needs2 = cache.needs_recompile(&empty_dir).unwrap();
        assert!(needs2, "Dir with new file should need recompile");
    }
}
