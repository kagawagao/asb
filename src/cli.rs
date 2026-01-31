use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use tracing::{error, info};

use crate::aapt2::Aapt2;
use crate::builder::SkinBuilder;
use crate::merge::{ModuleSkinPackage, SkinMerger};
use crate::types::{BuildConfig, ModuleConfig, MultiModuleConfig};

#[derive(Parser)]
#[command(name = "asb")]
#[command(about = "Android Skin Builder - Build resource-only skin packages using aapt2", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build a skin package from resources
    Build {
        /// Path to configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Path to resources directory
        #[arg(short, long)]
        resource_dir: Option<PathBuf>,

        /// Path to AndroidManifest.xml
        #[arg(short, long)]
        manifest: Option<PathBuf>,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Package name for the skin
        #[arg(short, long)]
        package: Option<String>,

        /// Path to android.jar
        #[arg(short, long)]
        android_jar: Option<PathBuf>,

        /// Paths to AAR files to include
        #[arg(long)]
        aar: Vec<PathBuf>,

        /// Path to aapt2 binary
        #[arg(long)]
        aapt2: Option<PathBuf>,

        /// Enable incremental build
        #[arg(long)]
        incremental: bool,

        /// Version code
        #[arg(long)]
        version_code: Option<u32>,

        /// Version name
        #[arg(long)]
        version_name: Option<String>,

        /// Path to stable IDs file
        #[arg(long)]
        stable_ids: Option<PathBuf>,

        /// Number of parallel workers
        #[arg(long)]
        workers: Option<usize>,
    },

    /// Build multiple modules and merge them
    BuildMulti {
        /// Path to multi-module configuration file
        #[arg(short, long)]
        config: PathBuf,
    },

    /// Clean build artifacts
    Clean {
        /// Path to configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show aapt2 version
    Version {
        /// Path to aapt2 binary
        #[arg(long)]
        aapt2: Option<PathBuf>,
    },

    /// Initialize a new skin project with sample configuration
    Init {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Build {
                config,
                resource_dir,
                manifest,
                output,
                package,
                android_jar,
                aar,
                aapt2,
                incremental,
                version_code,
                version_name,
                stable_ids,
                workers,
            } => {
                Self::run_build(
                    config,
                    resource_dir,
                    manifest,
                    output,
                    package,
                    android_jar,
                    aar,
                    aapt2,
                    incremental,
                    version_code,
                    version_name,
                    stable_ids,
                    workers,
                )
                .await
            }
            Commands::BuildMulti { config } => Self::run_build_multi(config).await,
            Commands::Clean { config, output } => Self::run_clean(config, output),
            Commands::Version { aapt2 } => Self::run_version(aapt2),
            Commands::Init { dir } => Self::run_init(dir),
        }
    }

    async fn run_build(
        config_file: Option<PathBuf>,
        resource_dir: Option<PathBuf>,
        manifest: Option<PathBuf>,
        output: Option<PathBuf>,
        package: Option<String>,
        android_jar: Option<PathBuf>,
        aar: Vec<PathBuf>,
        aapt2: Option<PathBuf>,
        incremental: bool,
        version_code: Option<u32>,
        version_name: Option<String>,
        stable_ids: Option<PathBuf>,
        workers: Option<usize>,
    ) -> Result<()> {
        let build_config = if let Some(config_path) = config_file {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: BuildConfig = serde_json::from_str(&content)?;

            // Override with CLI arguments
            if let Some(rd) = resource_dir {
                config.resource_dir = rd;
            }
            if let Some(m) = manifest {
                config.manifest_path = m;
            }
            if let Some(o) = output {
                config.output_dir = o;
            }
            if let Some(p) = package {
                config.package_name = p;
            }
            if let Some(aj) = android_jar {
                config.android_jar = aj;
            }
            if !aar.is_empty() {
                config.aar_files = Some(aar);
            }
            if let Some(a) = aapt2 {
                config.aapt2_path = Some(a);
            }
            if incremental {
                config.incremental = Some(true);
            }
            if let Some(vc) = version_code {
                config.version_code = Some(vc);
            }
            if let Some(vn) = version_name {
                config.version_name = Some(vn);
            }
            if let Some(si) = stable_ids {
                config.stable_ids_file = Some(si);
            }
            if let Some(w) = workers {
                config.parallel_workers = Some(w);
            }

            config
        } else {
            // Build from CLI arguments
            if resource_dir.is_none()
                || manifest.is_none()
                || output.is_none()
                || android_jar.is_none()
            {
                error!("Missing required options. Provide either --config or all of --resource-dir, --manifest, --output, and --android-jar");
                std::process::exit(1);
            }

            BuildConfig {
                resource_dir: resource_dir.unwrap(),
                manifest_path: manifest.unwrap(),
                output_dir: output.unwrap(),
                package_name: package.unwrap_or_else(|| "com.example.skin".to_string()),
                android_jar: android_jar.unwrap(),
                aar_files: if aar.is_empty() { None } else { Some(aar) },
                aapt2_path: aapt2,
                incremental: Some(incremental),
                cache_dir: None,
                version_code,
                version_name,
                additional_resource_dirs: None,
                compiled_dir: None,
                stable_ids_file: stable_ids,
                parallel_workers: workers,
            }
        };

        println!("{}", "\nBuilding skin package...\n".blue().bold());

        let mut builder = SkinBuilder::new(build_config)?;
        let result = builder.build().await?;

        if result.success {
            println!("{}", "\n✓ Skin package built successfully!".green().bold());
            if let Some(apk_path) = result.apk_path {
                println!("  {}: {}", "Output".cyan(), apk_path.display());
            }
        } else {
            println!("{}", "\n✗ Build failed:".red().bold());
            for error in &result.errors {
                println!("  - {}", error);
            }
            std::process::exit(1);
        }

        Ok(())
    }

    async fn run_build_multi(config_path: PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(&config_path)?;
        let multi_config: MultiModuleConfig = serde_json::from_str(&content)?;

        println!(
            "{}",
            format!("\nBuilding {} modules...\n", multi_config.modules.len())
                .blue()
                .bold()
        );

        let mut packages = Vec::new();

        for module_config in &multi_config.modules {
            info!("Building module: {}", module_config.name);
            println!("\n{}", format!("Module: {}", module_config.name).cyan());

            let mut builder = SkinBuilder::new(module_config.config.clone())?;
            let result = builder.build().await?;

            if !result.success {
                error!("Failed to build module: {}", module_config.name);
                for error in &result.errors {
                    error!("  - {}", error);
                }
                std::process::exit(1);
            }

            if let Some(apk_path) = result.apk_path {
                packages.push(ModuleSkinPackage {
                    module_name: module_config.name.clone(),
                    apk_path,
                });
            }
        }

        // Merge packages
        println!("\n{}", "Merging module packages...".blue().bold());
        SkinMerger::merge_packages(&packages, &multi_config.merged_output)?;

        println!("{}", "\n✓ Multi-module build completed!".green().bold());
        println!(
            "  {}: {}",
            "Merged output".cyan(),
            multi_config.merged_output.display()
        );

        Ok(())
    }

    fn run_clean(config_file: Option<PathBuf>, output_dir: Option<PathBuf>) -> Result<()> {
        let output = if let Some(config_path) = config_file {
            let content = std::fs::read_to_string(&config_path)?;
            let config: BuildConfig = serde_json::from_str(&content)?;
            config.output_dir
        } else if let Some(o) = output_dir {
            o
        } else {
            error!("Please provide either --config or --output");
            std::process::exit(1);
        };

        let compiled_dir = output.join("compiled");
        let temp_dir = output.join(".temp");
        let cache_dir = output.join(".build-cache");

        if compiled_dir.exists() {
            std::fs::remove_dir_all(&compiled_dir)?;
        }
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)?;
        }
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir)?;
        }

        println!("{}", "✓ Build artifacts cleaned".green());
        Ok(())
    }

    fn run_version(aapt2_path: Option<PathBuf>) -> Result<()> {
        let aapt2 = Aapt2::new(aapt2_path)?;
        let version = aapt2.version()?;
        println!("{}", "aapt2 version:".cyan());
        println!("{}", version);
        Ok(())
    }

    fn run_init(dir: PathBuf) -> Result<()> {
        let config_path = dir.join("asb.config.json");

        if config_path.exists() {
            println!("{}", "Configuration file already exists".yellow());
            return Ok(());
        }

        let sample_config = BuildConfig {
            resource_dir: PathBuf::from("./res"),
            manifest_path: PathBuf::from("./AndroidManifest.xml"),
            output_dir: PathBuf::from("./build"),
            package_name: "com.example.skin".to_string(),
            android_jar: PathBuf::from("${ANDROID_HOME}/platforms/android-30/android.jar"),
            aar_files: Some(vec![]),
            aapt2_path: None,
            incremental: Some(true),
            cache_dir: None,
            version_code: Some(1),
            version_name: Some("1.0.0".to_string()),
            additional_resource_dirs: None,
            compiled_dir: None,
            stable_ids_file: Some(PathBuf::from("./stable-ids.txt")),
            parallel_workers: None,
        };

        let content = serde_json::to_string_pretty(&sample_config)?;
        std::fs::write(&config_path, content)?;

        println!(
            "{}",
            format!("✓ Configuration file created: {}", config_path.display()).green()
        );
        println!("\n{}", "Edit the configuration file and run:".cyan());
        println!("  {}", "asb build --config asb.config.json".white());

        Ok(())
    }
}
