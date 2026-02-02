use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// App-specific configuration in multi-app mode
/// Contains only app-specific fields, common fields are inherited from parent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Path to the resources directory (res/)
    #[serde(rename = "resourceDir")]
    pub resource_dir: PathBuf,

    /// Path to the Android manifest file
    #[serde(rename = "manifestPath")]
    pub manifest_path: PathBuf,

    /// Package name for the skin package
    #[serde(rename = "packageName")]
    pub package_name: String,

    /// Additional resource directories (optional, for dependencies)
    #[serde(
        rename = "additionalResourceDirs",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub additional_resource_dirs: Option<Vec<PathBuf>>,

    /// App-specific output directory override (optional)
    #[serde(rename = "outputDir", skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<PathBuf>,

    /// App-specific version code override (optional)
    #[serde(rename = "versionCode", skip_serializing_if = "Option::is_none")]
    pub version_code: Option<u32>,

    /// App-specific version name override (optional)
    #[serde(rename = "versionName", skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,
}

/// Multi-app configuration wrapper
/// Supports multiple apps with common configuration extracted to top level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAppConfig {
    /// Common output directory for all apps
    #[serde(rename = "outputDir")]
    pub output_dir: PathBuf,

    /// Common Android platform JAR path
    #[serde(rename = "androidJar")]
    pub android_jar: PathBuf,

    /// Common aapt2 path (optional)
    #[serde(rename = "aapt2Path", skip_serializing_if = "Option::is_none")]
    pub aapt2_path: Option<PathBuf>,

    /// Common AAR files (optional)
    #[serde(rename = "aarFiles", skip_serializing_if = "Option::is_none", default)]
    pub aar_files: Option<Vec<PathBuf>>,

    /// Common incremental build setting (optional)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub incremental: Option<bool>,

    /// Common cache directory (optional)
    #[serde(rename = "cacheDir", skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,

    /// Common version code (optional, can be overridden per app)
    #[serde(rename = "versionCode", skip_serializing_if = "Option::is_none")]
    pub version_code: Option<u32>,

    /// Common version name (optional, can be overridden per app)
    #[serde(rename = "versionName", skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,

    /// Common stable IDs file (optional)
    #[serde(rename = "stableIdsFile", skip_serializing_if = "Option::is_none")]
    pub stable_ids_file: Option<PathBuf>,

    /// Common parallel workers setting (optional)
    #[serde(rename = "parallelWorkers", skip_serializing_if = "Option::is_none")]
    pub parallel_workers: Option<usize>,

    /// Array of app-specific configurations
    pub apps: Vec<AppConfig>,
}

impl MultiAppConfig {
    /// Convert multi-app config to individual BuildConfig instances
    /// Merges common fields with app-specific fields
    pub fn into_build_configs(self) -> Vec<BuildConfig> {
        self.apps
            .into_iter()
            .map(|app| BuildConfig {
                resource_dir: app.resource_dir,
                manifest_path: app.manifest_path,
                output_dir: app.output_dir.unwrap_or_else(|| self.output_dir.clone()),
                package_name: app.package_name,
                aapt2_path: self.aapt2_path.clone(),
                android_jar: self.android_jar.clone(),
                aar_files: self.aar_files.clone(),
                incremental: self.incremental,
                cache_dir: self.cache_dir.clone(),
                version_code: app.version_code.or(self.version_code),
                version_name: app.version_name.or_else(|| self.version_name.clone()),
                additional_resource_dirs: app.additional_resource_dirs,
                compiled_dir: None, // Will be set later if needed
                stable_ids_file: self.stable_ids_file.clone(),
                parallel_workers: self.parallel_workers,
            })
            .collect()
    }
}

/// Configuration for building Android skin packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Path to the resources directory (res/)
    #[serde(rename = "resourceDir")]
    pub resource_dir: PathBuf,

    /// Path to the Android manifest file
    #[serde(rename = "manifestPath")]
    pub manifest_path: PathBuf,

    /// Output directory for the skin package
    #[serde(rename = "outputDir")]
    pub output_dir: PathBuf,

    /// Package name for the skin package
    #[serde(rename = "packageName")]
    pub package_name: String,

    /// Path to aapt2 binary (optional, will auto-detect if not provided)
    #[serde(rename = "aapt2Path", skip_serializing_if = "Option::is_none")]
    pub aapt2_path: Option<PathBuf>,

    /// Path to Android platform JAR (android.jar)
    #[serde(rename = "androidJar")]
    pub android_jar: PathBuf,

    /// Additional AAR files to include resources from
    #[serde(rename = "aarFiles", skip_serializing_if = "Option::is_none", default)]
    pub aar_files: Option<Vec<PathBuf>>,

    /// Enable incremental build
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub incremental: Option<bool>,

    /// Build cache directory
    #[serde(rename = "cacheDir", skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,

    /// Version code for the skin package
    #[serde(rename = "versionCode", skip_serializing_if = "Option::is_none")]
    pub version_code: Option<u32>,

    /// Version name for the skin package
    #[serde(rename = "versionName", skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,

    /// Additional resource directories
    #[serde(
        rename = "additionalResourceDirs",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub additional_resource_dirs: Option<Vec<PathBuf>>,

    /// Compiled resource directory (for intermediate .flat files)
    #[serde(rename = "compiledDir", skip_serializing_if = "Option::is_none")]
    pub compiled_dir: Option<PathBuf>,

    /// Path to stable IDs file for consistent resource IDs
    #[serde(rename = "stableIdsFile", skip_serializing_if = "Option::is_none")]
    pub stable_ids_file: Option<PathBuf>,

    /// Number of parallel workers (defaults to number of CPUs)
    #[serde(rename = "parallelWorkers", skip_serializing_if = "Option::is_none")]
    pub parallel_workers: Option<usize>,
}

impl BuildConfig {
    /// Create default configuration based on standard Android project structure
    pub fn default_config() -> Self {
        // Try to find ANDROID_HOME for android.jar
        let android_jar = if let Ok(android_home) = std::env::var("ANDROID_HOME") {
            PathBuf::from(android_home).join("platforms/android-34/android.jar")
        } else {
            PathBuf::from("${ANDROID_HOME}/platforms/android-34/android.jar")
        };

        Self {
            resource_dir: PathBuf::from("./src/main/res"),
            manifest_path: PathBuf::from("./src/main/AndroidManifest.xml"),
            output_dir: PathBuf::from("./build/outputs/skin"),
            package_name: "com.example.skin".to_string(),
            android_jar,
            aar_files: None,
            aapt2_path: None,
            incremental: Some(true),
            cache_dir: None,
            version_code: Some(1),
            version_name: Some("1.0.0".to_string()),
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: None,
            parallel_workers: None,
        }
    }

    /// Expand environment variables in path strings
    fn expand_env_vars(path: &str) -> String {
        let mut result = path.to_string();
        
        // Find all ${VAR} patterns and replace them
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let end = start + end;
                let var_name = &result[start + 2..end];
                
                if let Ok(value) = std::env::var(var_name) {
                    result.replace_range(start..=end, &value);
                } else {
                    // If variable is not set, leave it as is
                    break;
                }
            } else {
                break;
            }
        }
        
        result
    }

    /// Expand environment variables in all path fields
    pub fn expand_paths(&mut self) {
        // Expand environment variables in paths
        self.resource_dir = PathBuf::from(Self::expand_env_vars(&self.resource_dir.to_string_lossy()));
        self.manifest_path = PathBuf::from(Self::expand_env_vars(&self.manifest_path.to_string_lossy()));
        self.output_dir = PathBuf::from(Self::expand_env_vars(&self.output_dir.to_string_lossy()));
        self.android_jar = PathBuf::from(Self::expand_env_vars(&self.android_jar.to_string_lossy()));
        
        if let Some(aapt2) = &self.aapt2_path {
            self.aapt2_path = Some(PathBuf::from(Self::expand_env_vars(&aapt2.to_string_lossy())));
        }
        
        if let Some(cache) = &self.cache_dir {
            self.cache_dir = Some(PathBuf::from(Self::expand_env_vars(&cache.to_string_lossy())));
        }
        
        if let Some(compiled) = &self.compiled_dir {
            self.compiled_dir = Some(PathBuf::from(Self::expand_env_vars(&compiled.to_string_lossy())));
        }
        
        if let Some(stable) = &self.stable_ids_file {
            self.stable_ids_file = Some(PathBuf::from(Self::expand_env_vars(&stable.to_string_lossy())));
        }
        
        if let Some(aars) = &self.aar_files {
            self.aar_files = Some(
                aars.iter()
                    .map(|p| PathBuf::from(Self::expand_env_vars(&p.to_string_lossy())))
                    .collect()
            );
        }
        
        if let Some(additional) = &self.additional_resource_dirs {
            self.additional_resource_dirs = Some(
                additional.iter()
                    .map(|p| PathBuf::from(Self::expand_env_vars(&p.to_string_lossy())))
                    .collect()
            );
        }
    }

    /// Load configuration from file or use defaults
    /// Priority: explicit config file > asb.config.json in current dir > built-in defaults
    pub fn load_or_default(config_file: Option<PathBuf>) -> anyhow::Result<Self> {
        // If explicit config file is provided, use it
        if let Some(config_path) = config_file {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: Self = serde_json::from_str(&content)?;
            config.expand_paths();
            return Ok(config);
        }

        // Check for asb.config.json in current directory
        let default_config_path = PathBuf::from("./asb.config.json");
        if default_config_path.exists() {
            let content = std::fs::read_to_string(&default_config_path)?;
            let mut config: Self = serde_json::from_str(&content)?;
            config.expand_paths();
            return Ok(config);
        }

        // Use built-in defaults
        Ok(Self::default_config())
    }

    /// Load multiple configurations from file
    /// Supports three modes for backward compatibility:
    /// 1. Multi-app object format (new): { "outputDir": "...", "androidJar": "...", "apps": [...] }
    /// 2. Array format: [{ config1 }, { config2 }]
    /// 3. Single object format: { "resourceDir": "...", ... }
    pub fn load_configs(config_file: Option<PathBuf>) -> anyhow::Result<Vec<Self>> {
        // Determine which config file to use
        let config_path = if let Some(path) = config_file {
            path
        } else {
            let default_path = PathBuf::from("./asb.config.json");
            if default_path.exists() {
                default_path
            } else {
                // No config file, use default single config
                return Ok(vec![Self::default_config()]);
            }
        };

        let content = std::fs::read_to_string(&config_path)?;
        
        // Try to parse as multi-app config first (new format)
        if let Ok(multi_config) = serde_json::from_str::<MultiAppConfig>(&content) {
            let mut configs = multi_config.into_build_configs();
            for config in &mut configs {
                config.expand_paths();
            }
            return Ok(configs);
        }
        
        // Try to parse as array (previous format)
        if let Ok(mut configs) = serde_json::from_str::<Vec<Self>>(&content) {
            for config in &mut configs {
                config.expand_paths();
            }
            return Ok(configs);
        }
        
        // Fall back to single object (original format for backward compatibility)
        let mut config: Self = serde_json::from_str(&content)?;
        config.expand_paths();
        Ok(vec![config])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_single_config() {
        let json = r#"{
            "resourceDir": "./res",
            "manifestPath": "./AndroidManifest.xml",
            "outputDir": "./build",
            "packageName": "com.example.app",
            "androidJar": "/path/to/android.jar"
        }"#;

        let configs: Vec<BuildConfig> = serde_json::from_str(json)
            .or_else(|_| serde_json::from_str::<BuildConfig>(json).map(|c| vec![c]))
            .unwrap();

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].package_name, "com.example.app");
    }

    #[test]
    fn test_load_array_config() {
        let json = r#"[
            {
                "resourceDir": "./app1/res",
                "manifestPath": "./app1/AndroidManifest.xml",
                "outputDir": "./build",
                "packageName": "com.example.app1",
                "androidJar": "/path/to/android.jar"
            },
            {
                "resourceDir": "./app2/res",
                "manifestPath": "./app2/AndroidManifest.xml",
                "outputDir": "./build",
                "packageName": "com.example.app2",
                "androidJar": "/path/to/android.jar"
            }
        ]"#;

        let configs: Vec<BuildConfig> = serde_json::from_str(json).unwrap();

        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].package_name, "com.example.app1");
        assert_eq!(configs[1].package_name, "com.example.app2");
    }

    #[test]
    fn test_load_array_config_with_dependencies() {
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
                "additionalResourceDirs": ["./base/res"]
            }
        ]"#;

        let configs: Vec<BuildConfig> = serde_json::from_str(json).unwrap();

        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].package_name, "com.example.base");
        assert_eq!(configs[1].package_name, "com.example.feature");
        assert!(configs[1].additional_resource_dirs.is_some());
        assert_eq!(configs[1].additional_resource_dirs.as_ref().unwrap().len(), 1);
    }
}

/// Result of aapt2 compile operation
#[derive(Debug)]
pub struct CompileResult {
    pub success: bool,
    pub flat_files: Vec<PathBuf>,
    pub errors: Vec<String>,
}

/// Result of aapt2 link operation
#[derive(Debug)]
pub struct LinkResult {
    pub success: bool,
    pub apk_path: Option<PathBuf>,
    pub errors: Vec<String>,
}

/// AAR file information
#[derive(Debug, Clone)]
pub struct AarInfo {
    pub path: PathBuf,
    pub resource_dir: Option<PathBuf>,
    pub manifest_path: Option<PathBuf>,
    pub extracted_dir: PathBuf,
}

/// Build result
#[derive(Debug)]
pub struct BuildResult {
    pub success: bool,
    pub apk_path: Option<PathBuf>,
    pub errors: Vec<String>,
}
