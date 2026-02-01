use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    fn expand_paths(&mut self) {
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
}

/// Multi-module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModuleConfig {
    /// List of module configurations
    pub modules: Vec<ModuleConfig>,

    /// Output file for merged skin package
    #[serde(rename = "mergedOutput")]
    pub merged_output: PathBuf,
}

/// Individual module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// Module name
    pub name: String,

    /// Build configuration for this module
    #[serde(flatten)]
    pub config: BuildConfig,
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
