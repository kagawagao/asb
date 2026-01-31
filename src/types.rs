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
