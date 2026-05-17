use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AsbError {
    #[error("aapt2 not found. Set ANDROID_HOME or specify --aapt2 path")]
    Aapt2NotFound,

    #[error("aapt2 compilation failed: {0}")]
    Aapt2CompileError(String),

    #[error("aapt2 link failed: {0}")]
    Aapt2LinkError(String),

    #[error("AAR file not found: {0}")]
    AarNotFound(PathBuf),

    #[error("AAR extraction failed: {0}")]
    AarExtractError(String),

    #[error("android.jar not found. Set ANDROID_HOME or configure androidJar")]
    AndroidJarNotFound,

    #[error("Build failed for '{package}': {errors:?}")]
    BuildFailed { package: String, errors: Vec<String> },

    #[error("Circular dependency detected in configuration")]
    CircularDependency,

    #[error("No configurations match packages: {0}")]
    NoMatchingPackages(String),

    #[error("Invalid manifest merge: {0}")]
    ManifestMergeError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
