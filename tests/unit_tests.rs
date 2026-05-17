//! Integration-level unit tests for ASB builder and cache modules.
//!
//! These tests verify the public API behavior of SkinBuilder, BuildCache,
//! and CommonDependencyCache in various scenarios including edge cases.
//!
//! Note: private functions (normalize_resource_path, has_adaptive_icon_resources,
//! create_minimal_manifest) are tested in their respective source files'
//! #[cfg(test)] modules since they are not accessible from integration tests.

use asb::builder::SkinBuilder;
use asb::cache::{BuildCache, CommonDependencyCache};
use asb::types::BuildConfig;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ==============================
// Helper utilities
// ==============================

/// Create a temporary file and return its path
fn create_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

/// Create a minimal resource directory structure with a values subdirectory
fn create_minimal_res_dir(base: &Path) -> PathBuf {
    let res_dir = base.join("res");
    let values_dir = res_dir.join("values");
    fs::create_dir_all(&values_dir).unwrap();
    create_file(
        &values_dir,
        "colors.xml",
        b"<resources><color name=\"primary\">#FF0000</color></resources>",
    );
    res_dir
}

/// Build a standard BuildConfig for testing
fn test_config(
    temp_dir: &TempDir,
    package_name: &str,
    incremental: Option<bool>,
    cache_dir: Option<PathBuf>,
    build_dir: Option<PathBuf>,
) -> BuildConfig {
    let res_dir = create_minimal_res_dir(temp_dir.path());
    BuildConfig {
        resource_dir: res_dir,
        manifest_path: temp_dir.path().join("AndroidManifest.xml"),
        output_dir: temp_dir.path().join("output"),
        output_file: None,
        package_name: package_name.to_string(),
        aapt2_path: None,
        android_jar: Some(PathBuf::from("/fake/android.jar")),
        aar_files: None,
        incremental,
        build_dir,
        cache_dir,
        version_code: None,
        version_name: None,
        additional_resource_dirs: None,
        compiled_dir: None,
        stable_ids_file: None,
        package_id: None,
        precompiled_dependencies: None,
    }
}

// ==============================
// SkinBuilder::new tests
// ==============================

#[test]
fn test_skin_builder_new_no_incremental() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp, "com.test.noinc", None, None, None);
    let builder = SkinBuilder::new(config).unwrap();
    assert!(
        !builder.has_cache(),
        "Cache should be None when incremental is None"
    );
}

#[test]
fn test_skin_builder_new_incremental_true() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp, "com.test.inc", Some(true), None, None);
    let builder = SkinBuilder::new(config).unwrap();
    assert!(
        builder.has_cache(),
        "Cache should be Some when incremental is true"
    );
}

#[test]
fn test_skin_builder_new_incremental_false() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp, "com.test.falseinc", Some(false), None, None);
    let builder = SkinBuilder::new(config).unwrap();
    assert!(
        !builder.has_cache(),
        "Cache should be None when incremental is false"
    );
}

#[test]
fn test_skin_builder_new_with_cache_dir() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("custom_cache");
    let config = test_config(
        &tmp,
        "com.test.cached",
        Some(true),
        Some(cache_dir.clone()),
        None,
    );
    let builder = SkinBuilder::new(config).unwrap();
    assert!(builder.has_cache(), "Cache should be created");
    assert!(
        cache_dir.join("com.test.cached").exists(),
        "Cache directory should be under the specified cache_dir"
    );
}

#[test]
fn test_skin_builder_new_with_build_dir() {
    let tmp = TempDir::new().unwrap();
    let build_dir = tmp.path().join("custom_build");
    let config = test_config(
        &tmp,
        "com.test.bd",
        Some(true),
        None,
        Some(build_dir.clone()),
    );
    let builder = SkinBuilder::new(config).unwrap();
    assert!(builder.has_cache(), "Cache should be created");
    assert!(
        build_dir.join("com.test.bd").exists(),
        "Cache directory should be under build_dir"
    );
}

#[test]
fn test_skin_builder_new_cache_dir_takes_priority() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("priority_cache");
    let build_dir = tmp.path().join("priority_build");

    // When both cache_dir and build_dir are set, cache_dir takes priority
    let config = test_config(
        &tmp,
        "com.test.priority",
        Some(true),
        Some(cache_dir.clone()),
        Some(build_dir.clone()),
    );
    let builder = SkinBuilder::new(config).unwrap();
    assert!(builder.has_cache(), "Cache should be created");
    assert!(
        cache_dir.join("com.test.priority").exists(),
        "Cache should be under cache_dir (takes priority over build_dir)"
    );
}

#[test]
fn test_skin_builder_new_default_cache_location() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp, "com.test.default", Some(true), None, None);
    let builder = SkinBuilder::new(config).unwrap();
    assert!(builder.has_cache(), "Cache should be created");

    // Default location: output_dir/.build/<package_name>
    let default_cache = tmp
        .path()
        .join("output")
        .join(".build")
        .join("com.test.default");
    assert!(
        default_cache.exists(),
        "Cache should default to output_dir/.build/<package_name>"
    );
}

// ==============================
// BuildCache integration tests
// ==============================

#[test]
fn test_build_cache_lifecycle_new_init_save_clear() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let mut cache = BuildCache::new(cache_dir.clone()).unwrap();

    // init creates the directory
    cache.init().unwrap();
    assert!(cache_dir.exists(), "init should create cache directory");

    // Empty cache has no entries
    assert!(cache.get_all_cached_flat_files().is_empty());

    // Add entry
    let source = create_file(tmp.path(), "test.xml", b"data");
    let flat = create_file(tmp.path(), "test.flat", b"flat_data");
    cache.update_entry(&source, &flat).unwrap();

    // Verify entry
    assert_eq!(cache.get_cached_flat_file(&source), Some(flat.clone()));

    // Save persists
    cache.save().unwrap();
    assert!(
        cache_dir.join("build-cache.json").exists(),
        "Cache file should be saved"
    );

    // Clear removes everything
    cache.clear().unwrap();
    assert!(cache.get_all_cached_flat_files().is_empty());
    assert!(
        !cache_dir.join("build-cache.json").exists(),
        "Cache file should be removed by clear"
    );
}

#[test]
fn test_build_cache_persistence_across_instances() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");

    let source = create_file(tmp.path(), "persist.xml", b"<resources/>");
    let flat = create_file(tmp.path(), "persist.flat", b"compiled");

    // Instance 1: populate and save
    {
        let mut cache = BuildCache::new(cache_dir.clone()).unwrap();
        cache.init().unwrap();
        cache.update_entry(&source, &flat).unwrap();
        cache.save().unwrap();
    }

    // Instance 2: load and verify
    {
        let mut cache = BuildCache::new(cache_dir.clone()).unwrap();
        // Unchanged file should not need recompile
        assert!(
            !cache.needs_recompile(&source).unwrap(),
            "Persisted cache should recognize unchanged file"
        );
        assert_eq!(
            cache.get_cached_flat_file(&source),
            Some(flat),
            "Should retrieve cached flat file"
        );
    }

    // Modify source file
    fs::write(&source, b"modified content").unwrap();

    // Instance 3: should detect change
    {
        let mut cache = BuildCache::new(cache_dir.clone()).unwrap();
        assert!(
            cache.needs_recompile(&source).unwrap(),
            "Modified file should trigger recompile"
        );
    }
}

#[test]
fn test_build_cache_multiple_entries() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let mut cache = BuildCache::new(cache_dir).unwrap();
    cache.init().unwrap();

    let sources: Vec<_> = (0..10)
        .map(|i| {
            create_file(
                tmp.path(),
                &format!("file_{}.xml", i),
                format!("v{}", i).as_bytes(),
            )
        })
        .collect();
    let flats: Vec<_> = (0..10)
        .map(|i| {
            create_file(
                tmp.path(),
                &format!("file_{}.flat", i),
                format!("fv{}", i).as_bytes(),
            )
        })
        .collect();

    // Add all entries
    for (s, f) in sources.iter().zip(flats.iter()) {
        cache.update_entry(s, f).unwrap();
    }

    // Verify all entries
    let all_flat = cache.get_all_cached_flat_files();
    assert_eq!(all_flat.len(), 10, "Should have 10 entries");

    for (s, f) in sources.iter().zip(flats.iter()) {
        assert_eq!(
            cache.get_cached_flat_file(s),
            Some(f.clone()),
            "Each source should map to its flat file"
        );
    }

    // Modify one file
    fs::write(&sources[5], b"modified_v5").unwrap();
    assert!(
        cache.needs_recompile(&sources[5]).unwrap(),
        "Modified file needs recompile"
    );

    // Other files still OK
    for (i, s) in sources.iter().enumerate() {
        if i != 5 {
            assert!(
                !cache.needs_recompile(s).unwrap(),
                "File {} should not need recompile",
                i
            );
        }
    }
}

#[test]
fn test_build_cache_edge_cases() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let mut cache = BuildCache::new(cache_dir).unwrap();
    cache.init().unwrap();

    // Empty file
    let empty = create_file(tmp.path(), "empty.xml", b"");
    let flat_empty = create_file(tmp.path(), "empty.flat", b"");
    cache.update_entry(&empty, &flat_empty).unwrap();
    assert!(
        !cache.needs_recompile(&empty).unwrap(),
        "Empty file should hash consistently"
    );

    // Binary file
    let binary = create_file(tmp.path(), "data.bin", &[0x00, 0x01, 0x02, 0xFF]);
    let flat_bin = create_file(tmp.path(), "data.flat", b"bin_flat");
    cache.update_entry(&binary, &flat_bin).unwrap();
    assert!(
        !cache.needs_recompile(&binary).unwrap(),
        "Binary file should hash consistently"
    );

    // Large file
    let large_content = vec![b'A'; 100_000];
    let large = create_file(tmp.path(), "large.xml", &large_content);
    let flat_large = create_file(tmp.path(), "large.flat", b"large_flat");
    cache.update_entry(&large, &flat_large).unwrap();
    assert!(
        !cache.needs_recompile(&large).unwrap(),
        "Large file should hash consistently"
    );
}

#[test]
fn test_build_cache_corrupted_recovery() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    // Test 1: Non-JSON content
    let cache_file = cache_dir.join("build-cache.json");
    fs::write(&cache_file, b"binary garbage \x00\x01\x02").unwrap();
    let cache = BuildCache::new(cache_dir.clone()).unwrap();
    assert!(
        cache.get_all_cached_flat_files().is_empty(),
        "Binary garbage should result in empty cache"
    );

    // Test 2: Valid JSON but missing entries field
    fs::write(&cache_file, b"{\"version\":\"1.0\"}").unwrap();
    let cache2 = BuildCache::new(cache_dir.clone()).unwrap();
    assert!(
        cache2.get_all_cached_flat_files().is_empty(),
        "Missing entries field should be handled gracefully"
    );

    // Test 3: Empty file
    fs::write(&cache_file, b"").unwrap();
    let cache3 = BuildCache::new(cache_dir).unwrap();
    assert!(
        cache3.get_all_cached_flat_files().is_empty(),
        "Empty file should result in empty cache"
    );
}

// ==============================
// CommonDependencyCache integration tests
// ==============================

#[test]
fn test_common_dep_cache_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let mut cache = CommonDependencyCache::new(cache_dir.clone()).unwrap();

    // init
    cache.init().unwrap();
    assert!(cache_dir.exists());

    // Create resource directory with files
    let res_dir = tmp.path().join("res");
    fs::create_dir_all(&res_dir).unwrap();
    create_file(
        &res_dir,
        "colors.xml",
        b"<resources><color name=\"x\">#fff</color></resources>",
    );
    create_file(
        &res_dir,
        "dimens.xml",
        b"<resources><dimen name=\"y\">10dp</dimen></resources>",
    );

    let flats: Vec<_> = ["colors.flat", "dimens.flat"]
        .iter()
        .map(|name| create_file(tmp.path(), name, b"compiled"))
        .collect();

    // Add entry
    cache.update_entry(&res_dir, flats.clone()).unwrap();

    // Retrieve
    assert_eq!(
        cache.get_cached_flat_files(&res_dir),
        Some(flats.clone()),
        "Should return cached flat files"
    );

    // Unchanged -> no recompile
    assert!(!cache.needs_recompile(&res_dir).unwrap());

    // Save & reload
    cache.save().unwrap();
    let cache2 = CommonDependencyCache::new(cache_dir).unwrap();
    assert_eq!(cache2.get_cached_flat_files(&res_dir), Some(flats));
}

#[test]
fn test_common_dep_cache_directory_hash_detects_changes() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
    cache.init().unwrap();

    let res_dir = tmp.path().join("res");
    fs::create_dir_all(&res_dir).unwrap();
    create_file(&res_dir, "a.xml", b"original");
    create_file(&res_dir, "b.xml", b"also original");

    let flat = create_file(tmp.path(), "combined.flat", b"all_compiled");
    cache.update_entry(&res_dir, vec![flat.clone()]).unwrap();

    // Change existing file
    fs::write(res_dir.join("a.xml"), b"modified").unwrap();
    assert!(
        cache.needs_recompile(&res_dir).unwrap(),
        "Modified file should trigger recompile"
    );

    // Restore and verify unchanged
    fs::write(res_dir.join("a.xml"), b"original").unwrap();
    assert!(
        !cache.needs_recompile(&res_dir).unwrap(),
        "Restored file should not trigger recompile"
    );

    // Add a new file
    create_file(&res_dir, "c.xml", b"new file");
    assert!(
        cache.needs_recompile(&res_dir).unwrap(),
        "New file should trigger recompile"
    );
}

#[test]
fn test_common_dep_cache_multiple_directories() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
    cache.init().unwrap();

    // Two distinct resource directories
    let dir1 = tmp.path().join("res1");
    let dir2 = tmp.path().join("res2");
    fs::create_dir_all(&dir1).unwrap();
    fs::create_dir_all(&dir2).unwrap();

    create_file(
        &dir1,
        "colors.xml",
        b"<resources><color name=\"a\">#aaa</color></resources>",
    );
    create_file(&dir2, "shapes.xml", b"<resources/>");

    let flat1 = create_file(tmp.path(), "res1.flat", b"f1");
    let flat2 = create_file(tmp.path(), "res2.flat", b"f2");

    cache.update_entry(&dir1, vec![flat1.clone()]).unwrap();
    cache.update_entry(&dir2, vec![flat2.clone()]).unwrap();

    assert_eq!(cache.get_cached_flat_files(&dir1), Some(vec![flat1]));
    assert_eq!(cache.get_cached_flat_files(&dir2), Some(vec![flat2]));
    assert!(!cache.needs_recompile(&dir1).unwrap());
    assert!(!cache.needs_recompile(&dir2).unwrap());

    // Modify dir1 only
    fs::write(
        dir1.join("colors.xml"),
        b"<resources><color name=\"a\">#bbb</color></resources>",
    )
    .unwrap();
    assert!(cache.needs_recompile(&dir1).unwrap());
    assert!(
        !cache.needs_recompile(&dir2).unwrap(),
        "dir2 should remain unchanged"
    );
}

#[test]
fn test_common_dep_cache_edge_cases() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    let mut cache = CommonDependencyCache::new(cache_dir).unwrap();
    cache.init().unwrap();

    // Empty directory
    let empty_dir = tmp.path().join("empty");
    fs::create_dir_all(&empty_dir).unwrap();
    let empty_flat = create_file(tmp.path(), "empty.flat", b"ef");
    cache
        .update_entry(&empty_dir, vec![empty_flat.clone()])
        .unwrap();
    assert!(
        !cache.needs_recompile(&empty_dir).unwrap(),
        "Empty dir should be consistent"
    );

    // Directory with a nested subdirectory containing files
    let nested_dir = tmp.path().join("nested");
    fs::create_dir_all(nested_dir.join("values")).unwrap();
    create_file(&nested_dir.join("values"), "strings.xml", b"<resources/>");
    let nested_flat = create_file(tmp.path(), "nested.flat", b"nf");
    cache
        .update_entry(&nested_dir, vec![nested_flat.clone()])
        .unwrap();
    assert!(!cache.needs_recompile(&nested_dir).unwrap());

    // Modify nested file
    fs::write(
        nested_dir.join("values").join("strings.xml"),
        b"<resources><string name=\"x\">y</string></resources>",
    )
    .unwrap();
    assert!(
        cache.needs_recompile(&nested_dir).unwrap(),
        "Nested file change should be detected"
    );
}

#[test]
fn test_common_dep_cache_corrupted_recovery() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let cache_file = cache_dir.join("common-dep-cache.json");

    // Test: garbage content
    fs::write(&cache_file, b"\x00\x01\x02 not json").unwrap();
    let cache = CommonDependencyCache::new(cache_dir.clone()).unwrap();
    assert!(
        cache
            .get_cached_flat_files(&PathBuf::from("/fake"))
            .is_none(),
        "Corrupted file should yield empty cache"
    );

    // Test: wrong version
    fs::write(&cache_file, b"{\"version\":\"2.0-invalid\",\"entries\":{}}").unwrap();
    let cache2 = CommonDependencyCache::new(cache_dir).unwrap();
    assert!(
        cache2
            .get_cached_flat_files(&PathBuf::from("/fake"))
            .is_none(),
        "Wrong version should yield empty cache"
    );
}

#[test]
fn test_skin_builder_new_missing_resource_dir() {
    let tmp = TempDir::new().unwrap();
    let config = BuildConfig {
        resource_dir: tmp.path().join("nonexistent_res"),
        manifest_path: tmp.path().join("AndroidManifest.xml"),
        output_dir: tmp.path().join("output"),
        output_file: None,
        package_name: "com.test.nodir".to_string(),
        aapt2_path: None,
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

    // Should still succeed to create builder even without existing res dir
    let builder = SkinBuilder::new(config).unwrap();
    assert!(!builder.has_cache());
}
