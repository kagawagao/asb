use asb::types::BuildConfig;
use std::path::PathBuf;

/// Integration tests for array mode configuration
/// These tests verify the complete workflow of loading and processing configurations

#[test]
fn test_load_single_config_from_json() {
    let json = r#"{
        "resourceDir": "./res",
        "manifestPath": "./AndroidManifest.xml",
        "outputDir": "./build",
        "packageName": "com.example.test",
        "androidJar": "/path/to/android.jar",
        "incremental": true,
        "versionCode": 1,
        "versionName": "1.0.0"
    }"#;

    // Parse as single config
    let config: BuildConfig = serde_json::from_str(json).unwrap();
    
    assert_eq!(config.package_name, "com.example.test");
    assert_eq!(config.resource_dir, PathBuf::from("./res"));
    assert_eq!(config.manifest_path, PathBuf::from("./AndroidManifest.xml"));
    assert_eq!(config.version_code, Some(1));
    assert_eq!(config.version_name, Some("1.0.0".to_string()));
}

#[test]
fn test_load_array_config_from_json() {
    let json = r#"[
        {
            "resourceDir": "./app1/res",
            "manifestPath": "./app1/AndroidManifest.xml",
            "outputDir": "./build",
            "packageName": "com.example.app1",
            "androidJar": "/path/to/android.jar",
            "incremental": true,
            "versionCode": 1,
            "versionName": "1.0.0"
        },
        {
            "resourceDir": "./app2/res",
            "manifestPath": "./app2/AndroidManifest.xml",
            "outputDir": "./build",
            "packageName": "com.example.app2",
            "androidJar": "/path/to/android.jar",
            "incremental": true,
            "versionCode": 2,
            "versionName": "2.0.0"
        }
    ]"#;

    // Parse as array config
    let configs: Vec<BuildConfig> = serde_json::from_str(json).unwrap();
    
    assert_eq!(configs.len(), 2);
    
    assert_eq!(configs[0].package_name, "com.example.app1");
    assert_eq!(configs[0].resource_dir, PathBuf::from("./app1/res"));
    assert_eq!(configs[0].version_code, Some(1));
    
    assert_eq!(configs[1].package_name, "com.example.app2");
    assert_eq!(configs[1].resource_dir, PathBuf::from("./app2/res"));
    assert_eq!(configs[1].version_code, Some(2));
}

#[test]
fn test_array_config_with_additional_resources() {
    let json = r#"[
        {
            "resourceDir": "./base/res",
            "manifestPath": "./base/AndroidManifest.xml",
            "outputDir": "./build",
            "packageName": "com.example.base",
            "androidJar": "/path/to/android.jar"
        },
        {
            "resourceDir": "./feature/res",
            "manifestPath": "./feature/AndroidManifest.xml",
            "outputDir": "./build",
            "packageName": "com.example.feature",
            "androidJar": "/path/to/android.jar",
            "additionalResourceDirs": ["./base/res", "./common/res"]
        }
    ]"#;

    let configs: Vec<BuildConfig> = serde_json::from_str(json).unwrap();
    
    assert_eq!(configs.len(), 2);
    
    // Base config has no additional resources
    assert!(configs[0].additional_resource_dirs.is_none());
    
    // Feature config has additional resources
    assert!(configs[1].additional_resource_dirs.is_some());
    let additional = configs[1].additional_resource_dirs.as_ref().unwrap();
    assert_eq!(additional.len(), 2);
    assert_eq!(additional[0], PathBuf::from("./base/res"));
    assert_eq!(additional[1], PathBuf::from("./common/res"));
}

#[test]
fn test_backward_compatibility_single_to_array() {
    let single_json = r#"{
        "resourceDir": "./res",
        "manifestPath": "./AndroidManifest.xml",
        "outputDir": "./build",
        "packageName": "com.example.single",
        "androidJar": "/path/to/android.jar"
    }"#;

    // Try parsing as array first (should fail), then as single
    let configs: Vec<BuildConfig> = serde_json::from_str::<Vec<BuildConfig>>(single_json)
        .or_else(|_| serde_json::from_str::<BuildConfig>(single_json).map(|c| vec![c]))
        .unwrap();

    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].package_name, "com.example.single");
}

#[test]
fn test_config_with_all_optional_fields() {
    let json = r#"{
        "resourceDir": "./res",
        "manifestPath": "./AndroidManifest.xml",
        "outputDir": "./build",
        "packageName": "com.example.full",
        "androidJar": "/path/to/android.jar",
        "aapt2Path": "/path/to/aapt2",
        "aarFiles": ["/path/to/lib.aar"],
        "incremental": true,
        "cacheDir": "./cache",
        "versionCode": 42,
        "versionName": "3.14.159",
        "additionalResourceDirs": ["./extra/res"],
        "compiledDir": "./compiled",
        "stableIdsFile": "./stable-ids.txt",
        "parallelWorkers": 8
    }"#;

    let config: BuildConfig = serde_json::from_str(json).unwrap();
    
    assert_eq!(config.package_name, "com.example.full");
    assert_eq!(config.aapt2_path, Some(PathBuf::from("/path/to/aapt2")));
    assert_eq!(config.aar_files, Some(vec![PathBuf::from("/path/to/lib.aar")]));
    assert_eq!(config.incremental, Some(true));
    assert_eq!(config.cache_dir, Some(PathBuf::from("./cache")));
    assert_eq!(config.version_code, Some(42));
    assert_eq!(config.version_name, Some("3.14.159".to_string()));
    assert_eq!(config.additional_resource_dirs, Some(vec![PathBuf::from("./extra/res")]));
    assert_eq!(config.compiled_dir, Some(PathBuf::from("./compiled")));
    assert_eq!(config.stable_ids_file, Some(PathBuf::from("./stable-ids.txt")));
    assert_eq!(config.parallel_workers, Some(8));
}
