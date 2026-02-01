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
        // Check if using defaults before moving config_file
        let using_defaults = config_file.is_none() && !PathBuf::from("./asb.config.json").exists();
        
        // Load config: explicit file > asb.config.json in current dir > defaults
        let mut build_config = BuildConfig::load_or_default(config_file)?;

        // Show info message if using defaults
        if using_defaults {
            info!("Using default configuration based on standard Android project structure");
            info!("Create asb.config.json in current directory to customize settings");
        }

        // Override config with CLI arguments (CLI args have highest priority)
        if let Some(rd) = resource_dir {
            build_config.resource_dir = rd;
        }
        if let Some(m) = manifest {
            build_config.manifest_path = m;
        }
        if let Some(o) = output {
            build_config.output_dir = o;
        }
        if let Some(p) = package {
            build_config.package_name = p;
        }
        if let Some(aj) = android_jar {
            build_config.android_jar = aj;
        }
        if !aar.is_empty() {
            build_config.aar_files = Some(aar);
        }
        if let Some(a) = aapt2 {
            build_config.aapt2_path = Some(a);
        }
        if incremental {
            build_config.incremental = Some(true);
        }
        if let Some(vc) = version_code {
            build_config.version_code = Some(vc);
        }
        if let Some(vn) = version_name {
            build_config.version_name = Some(vn);
        }
        if let Some(si) = stable_ids {
            build_config.stable_ids_file = Some(si);
        }
        if let Some(w) = workers {
            build_config.parallel_workers = Some(w);
        }

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

            // Clone and expand environment variables in the config
            let mut config = module_config.config.clone();
            config.expand_paths();
            
            let mut builder = SkinBuilder::new(config)?;
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

        // Use default config which follows standard Android structure
        let sample_config = BuildConfig::default_config();

        let content = serde_json::to_string_pretty(&sample_config)?;
        std::fs::write(&config_path, content)?;

        println!(
            "{}",
            format!("✓ Configuration file created: {}", config_path.display()).green()
        );
        println!("\n{}", "Default configuration uses standard Android project structure:".cyan());
        println!("  {}: src/main/res/", "Resources".white());
        println!("  {}: src/main/AndroidManifest.xml", "Manifest".white());
        println!("  {}: build/outputs/skin/", "Output".white());
        println!("\n{}", "Edit the configuration file and run:".cyan());
        println!("  {}", "asb build".white());
        println!("\n{}", "Or simply run 'asb build' without config (uses defaults or ./asb.config.json if exists)".cyan());

        Ok(())
    }
}
