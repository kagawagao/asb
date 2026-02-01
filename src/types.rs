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
            PathBuf::from(android_home).join("platforms/android-30/android.jar")
        } else {
            PathBuf::from("${ANDROID_HOME}/platforms/android-30/android.jar")
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

    /// Load configuration from file or use defaults
    /// Priority: explicit config file > asb.config.json in current dir > built-in defaults
    pub fn load_or_default(config_file: Option<PathBuf>) -> anyhow::Result<Self> {
        // If explicit config file is provided, use it
        if let Some(config_path) = config_file {
            let content = std::fs::read_to_string(&config_path)?;
            return Ok(serde_json::from_str(&content)?);
        }

        // Check for asb.config.json in current directory
        let default_config_path = PathBuf::from("./asb.config.json");
        if default_config_path.exists() {
            let content = std::fs::read_to_string(&default_config_path)?;
            return Ok(serde_json::from_str(&content)?);
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
