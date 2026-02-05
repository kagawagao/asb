use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

use crate::aapt2::Aapt2;
use crate::builder::SkinBuilder;
use crate::cache::CommonDependencyCache;
use crate::dependency::{extract_common_dependencies, group_configs_by_dependencies};
use crate::types::BuildConfig;

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

        /// Maximum number of parallel builds for multiple configurations
        /// Controls how many configs can be built simultaneously (default: CPU core count)
        #[arg(long)]
        max_parallel_builds: Option<usize>,

        /// Package ID for resources (e.g., "0x7f")
        /// Critical for dynamic resource loading via new Resources()
        #[arg(long)]
        package_id: Option<String>,

        /// Filter packages to build (comma-separated package names)
        /// Only build configurations matching these package names
        #[arg(long, value_delimiter = ',')]
        packages: Vec<String>,
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
                max_parallel_builds,
                package_id,
                packages,
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
                    max_parallel_builds,
                    package_id,
                    packages,
                )
                .await
            }
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
        max_parallel_builds: Option<usize>,
        package_id: Option<String>,
        packages: Vec<String>,
    ) -> Result<()> {
        // Initialize rayon thread pool with CPU cores * 2
        // This is for resource compilation within each build
        let worker_threads = num_cpus::get() * 2;
        if rayon::ThreadPoolBuilder::new()
            .num_threads(worker_threads)
            .build_global()
            .is_ok()
        {
            info!(
                "Initialized resource compilation thread pool with {} workers (CPU cores * 2)",
                worker_threads
            );
        }

        // Check if CLI arguments are provided
        let has_cli_args = resource_dir.is_some()
            || manifest.is_some()
            || output.is_some()
            || package.is_some()
            || android_jar.is_some()
            || !aar.is_empty()
            || aapt2.is_some()
            || incremental
            || version_code.is_some()
            || version_name.is_some()
            || stable_ids.is_some()
            || max_parallel_builds.is_some()
            || package_id.is_some();

        // Check if using defaults before moving config_file
        let using_defaults = config_file.is_none() && !PathBuf::from("./asb.config.json").exists();

        // Load configs: support both single and array mode
        let loaded = BuildConfig::load_configs(config_file)?;

        // Save all package names before moving configs (for error messages)
        let all_package_names: Vec<String> = loaded
            .configs
            .iter()
            .map(|c| c.package_name.clone())
            .collect();

        let mut build_configs = loaded.configs;
        let config_max_parallel = loaded.max_parallel_builds;

        // Filter configs by package names if specified
        if !packages.is_empty() {
            let original_count = build_configs.len();
            build_configs.retain(|config| packages.contains(&config.package_name));
            let filtered_count = build_configs.len();

            if filtered_count == 0 {
                anyhow::bail!(
                    "No configurations found matching specified packages: {}. Available packages: {}",
                    packages.join(", "),
                    all_package_names.join(", ")
                );
            }

            info!(
                "Filtered {} out of {} configurations by package names: {}",
                filtered_count,
                original_count,
                packages.join(", ")
            );
        }

        info!(
            "Config maximum parallel builds setting: {:?}",
            config_max_parallel
        );
        info!(
            "CLI maximum parallel builds setting: {:?}",
            max_parallel_builds
        );

        // Get max parallel builds setting (CLI > config > default: CPU cores)
        let max_parallel = max_parallel_builds
            .or(config_max_parallel)
            .unwrap_or_else(|| num_cpus::get());

        info!("Maximum parallel builds: {} configs", max_parallel);

        // Show info message if using defaults
        if using_defaults {
            info!("Using default configuration based on standard Android project structure");
            info!("Create asb.config.json in current directory to customize settings");
        }

        // If CLI arguments are provided and we have multiple configs, warn the user
        if has_cli_args && build_configs.len() > 1 {
            warn!("CLI arguments provided with array configuration. CLI arguments will override ALL configurations.");
        }

        // Override configs with CLI arguments (CLI args have highest priority)
        if has_cli_args {
            for build_config in &mut build_configs {
                if let Some(ref rd) = resource_dir {
                    build_config.resource_dir = rd.clone();
                }
                if let Some(ref m) = manifest {
                    build_config.manifest_path = m.clone();
                }
                if let Some(ref o) = output {
                    build_config.output_dir = o.clone();
                }
                if let Some(ref p) = package {
                    build_config.package_name = p.clone();
                }
                if let Some(ref aj) = android_jar {
                    build_config.android_jar = aj.clone();
                }
                if !aar.is_empty() {
                    build_config.aar_files = Some(aar.clone());
                }
                if let Some(ref a) = aapt2 {
                    build_config.aapt2_path = Some(a.clone());
                }
                if incremental {
                    build_config.incremental = Some(true);
                }
                if let Some(vc) = version_code {
                    build_config.version_code = Some(vc);
                }
                if let Some(ref vn) = version_name {
                    build_config.version_name = Some(vn.clone());
                }
                if let Some(ref si) = stable_ids {
                    build_config.stable_ids_file = Some(si.clone());
                }
                if let Some(ref pid) = package_id {
                    build_config.package_id = Some(pid.clone());
                }
            }
        }

        // Resolve aapt2 path once for all configs if not explicitly provided
        // This avoids repeated searches when building multiple packages
        let resolved_aapt2_path = if build_configs.iter().all(|c| c.aapt2_path.is_none()) {
            // Only search once if no config has an explicit aapt2 path
            Some(Aapt2::new(None)?.into_path())
        } else {
            None
        };

        // For array mode with multiple configs, ensure each config has a unique compiled directory
        // to avoid conflicts when building multiple packages from the same output directory
        if build_configs.len() > 1 {
            for build_config in build_configs.iter_mut() {
                // Only set if not explicitly configured
                // Use package name as compiled directory for stable, identifiable output
                if build_config.compiled_dir.is_none() {
                    let unique_compiled_dir =
                        build_config.output_dir.join(&build_config.package_name);
                    build_config.compiled_dir = Some(unique_compiled_dir);
                }

                // Apply resolved aapt2 path if config doesn't have one
                if build_config.aapt2_path.is_none() {
                    build_config.aapt2_path = resolved_aapt2_path.clone();
                }
            }
        } else if let Some(ref path) = resolved_aapt2_path {
            // Single config mode - apply resolved path if needed
            if build_configs[0].aapt2_path.is_none() {
                build_configs[0].aapt2_path = Some(path.clone());
            }
        }

        if build_configs.len() == 1 {
            // Single configuration mode - keep backward compatibility
            let config = build_configs.into_iter().next().unwrap();
            let package_name = config.package_name.clone();
            let output_dir = config.output_dir.clone();
            println!("{}", "\nBuilding skin package...\n".blue().bold());
            let start_time = std::time::Instant::now();
            let mut builder = SkinBuilder::new(config)?;
            let result = builder.build().await?;
            let elapsed = start_time.elapsed();

            if result.success {
                println!("{}", "\n✓ Skin package built successfully!".green().bold());
                if let Some(apk_path) = result.apk_path {
                    println!("  {}: {}", "Output".cyan(), apk_path.display());
                }
                println!("  {}: {:.2}s", "Total time".cyan(), elapsed.as_secs_f64());
                println!(
                    "  {}: {:.2}s",
                    "Build time".cyan(),
                    result.build_duration.as_secs_f64()
                );
            } else {
                println!(
                    "{}",
                    format!("\n✗ Build failed for package '{}'", package_name)
                        .red()
                        .bold()
                );
                for error in &result.errors {
                    println!("  - {}", error);
                }

                // Save failure log
                match Self::save_failure_log(&package_name, &result.errors, &output_dir) {
                    Ok(log_path) => {
                        println!("\n  {}: {}", "Log saved to".yellow(), log_path.display());
                    }
                    Err(e) => {
                        warn!("Failed to save error log: {}", e);
                    }
                }

                std::process::exit(1);
            }
        } else {
            // Multiple configurations mode
            // Keep a copy of original configs for displaying package names later
            let original_configs = build_configs.clone();

            println!(
                "{}",
                format!("\nBuilding {} skin packages...\n", build_configs.len())
                    .blue()
                    .bold()
            );

            let start_time = std::time::Instant::now();

            // Extract common dependencies
            let common_deps = extract_common_dependencies(&build_configs);

            if !common_deps.is_empty() {
                info!(
                    "Found {} common dependencies to compile first",
                    common_deps.len()
                );

                // Determine cache directory for common dependencies
                // Use the first config's cache directory as the base, since all configs should use compatible settings
                // for shared common dependency compilation
                let base_cache_dir = build_configs[0]
                    .cache_dir
                    .clone()
                    .unwrap_or_else(|| build_configs[0].output_dir.join(".build-cache"));
                let common_cache_dir = base_cache_dir.join("common-deps");

                // Initialize common dependency cache
                let mut common_dep_cache = CommonDependencyCache::new(common_cache_dir.clone())?;
                common_dep_cache.init()?;

                // Use aapt2 path from first config (all configs should use the same aapt2)
                let aapt2 = Aapt2::new(build_configs[0].aapt2_path.clone())?;

                // Compile common dependencies
                for common_dep in &common_deps {
                    info!(
                        "Compiling common dependency: {} (used by {} apps)",
                        common_dep.resource_dir.display(),
                        common_dep.dependent_configs.len()
                    );

                    // Check if we need to recompile
                    let needs_recompile =
                        common_dep_cache.needs_recompile(&common_dep.resource_dir)?;

                    if needs_recompile {
                        // Compile the common dependency
                        let compiled_dir = common_cache_dir.join("compiled");
                        std::fs::create_dir_all(&compiled_dir)?;

                        let compile_result =
                            aapt2.compile_dir(&common_dep.resource_dir, &compiled_dir)?;

                        if compile_result.success {
                            info!(
                                "  ✓ Compiled {} resources into {} flat files",
                                common_dep.resource_dir.display(),
                                compile_result.flat_files.len()
                            );

                            // Update cache
                            common_dep_cache.update_entry(
                                &common_dep.resource_dir,
                                compile_result.flat_files,
                            )?;
                        } else {
                            error!(
                                "  ✗ Failed to compile common dependency: {}",
                                common_dep.resource_dir.display()
                            );
                            for err in &compile_result.errors {
                                error!("    {}", err);
                            }
                        }
                    } else {
                        info!(
                            "  ✓ Using cached compiled resources for {}",
                            common_dep.resource_dir.display()
                        );
                    }
                }

                // Save common dependency cache
                common_dep_cache.save()?;
            }

            // Group configs by dependencies
            let (independent_configs, dependent_groups) =
                group_configs_by_dependencies(build_configs)?;

            info!(
                "Found {} independent configs and {} dependency groups",
                independent_configs.len(),
                dependent_groups.len()
            );

            let mut all_results = Vec::new();
            let mut success_count = 0;
            let mut fail_count = 0;

            // Build independent configs in parallel
            if !independent_configs.is_empty() {
                info!(
                    "Building {} independent configs in parallel (max {} concurrent)...",
                    independent_configs.len(),
                    max_parallel
                );

                // Use semaphore to limit concurrent builds
                let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_parallel));
                let mut tasks: tokio::task::JoinSet<
                    Result<(usize, String, crate::types::BuildResult), (String, anyhow::Error)>,
                > = tokio::task::JoinSet::new();

                for config_with_idx in independent_configs {
                    let idx = config_with_idx.index;
                    let config = config_with_idx.config.clone();
                    let package_name = config.package_name.clone();
                    let sem = semaphore.clone();

                    tasks.spawn(async move {
                        // Acquire semaphore permit
                        let _permit = sem.acquire().await.unwrap();

                        match SkinBuilder::new(config) {
                            Ok(mut builder) => match builder.build().await {
                                Ok(result) => Ok((idx, package_name, result)),
                                Err(e) => Err((package_name.clone(), e)),
                            },
                            Err(e) => Err((package_name.clone(), e)),
                        }
                    });
                }

                while let Some(result) = tasks.join_next().await {
                    match result {
                        Ok(Ok((idx, _package_name, build_result))) => {
                            all_results.push((idx, build_result));
                        }
                        Ok(Err((package_name, e))) => {
                            error!("Build error for package '{}': {}", package_name, e);
                            // Print full error chain for debugging
                            let mut source = e.source();
                            let mut depth = 1;
                            while let Some(err) = source {
                                error!("  Caused by ({}): {}", depth, err);
                                source = err.source();
                                depth += 1;
                            }
                            fail_count += 1;
                        }
                        Err(e) => {
                            error!("Task join error: {}", e);
                            fail_count += 1;
                        }
                    }
                }
            }

            // Build dependent groups sequentially
            for (group_number, group) in dependent_groups.into_iter().enumerate() {
                info!(
                    "Building dependency group {} with {} configs sequentially...",
                    group_number + 1,
                    group.len()
                );

                for config_with_idx in group {
                    let config = config_with_idx.config.clone();
                    let package_name = config.package_name.clone();
                    match Self::build_single_config(config).await {
                        Ok(result) => {
                            all_results.push((config_with_idx.index, result));
                        }
                        Err(e) => {
                            error!("Build error for package '{}': {}", package_name, e);
                            // Print full error chain for debugging
                            let mut source = e.source();
                            let mut depth = 1;
                            while let Some(err) = source {
                                error!("  Caused by ({}): {}", depth, err);
                                source = err.source();
                                depth += 1;
                            }
                            fail_count += 1;
                        }
                    }
                }
            }

            // Count successes and failures
            for (_, result) in &all_results {
                if result.success {
                    success_count += 1;
                } else {
                    fail_count += 1;
                }
            }

            let elapsed = start_time.elapsed();

            // Display results
            println!("\n{}", "Build Summary:".blue().bold());
            println!("  {}: {}", "Successful".green(), success_count);
            println!("  {}: {}", "Failed".red(), fail_count);
            println!("  {}: {:.2}s", "Total time".cyan(), elapsed.as_secs_f64());

            // Show individual results
            // Create a mapping from index to package name for display
            let package_names: std::collections::HashMap<usize, String> = original_configs
                .iter()
                .enumerate()
                .map(|(idx, cfg)| (idx, cfg.package_name.clone()))
                .collect();

            println!("\n{}", "Individual Results:".blue().bold());
            for (idx, result) in &all_results {
                let package_name = package_names
                    .get(idx)
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                if result.success {
                    if let Some(ref apk_path) = result.apk_path {
                        println!(
                            "  {} Config #{} [{}]: {} ({:.2}s)",
                            "✓".green(),
                            idx + 1,
                            package_name,
                            apk_path.display(),
                            result.build_duration.as_secs_f64()
                        );
                    } else {
                        println!(
                            "  {} Config #{} [{}] ({:.2}s)",
                            "✓".green(),
                            idx + 1,
                            package_name,
                            result.build_duration.as_secs_f64()
                        );
                    }
                } else {
                    println!(
                        "  {} Config #{} [{}]: Build failed ({:.2}s)",
                        "✗".red(),
                        idx + 1,
                        package_name,
                        result.build_duration.as_secs_f64()
                    );
                    for error in &result.errors {
                        println!("      - {}", error);
                    }
                    // Save failure log
                    if let Some(config) = original_configs.get(*idx) {
                        match Self::save_failure_log(
                            package_name,
                            &result.errors,
                            &config.output_dir,
                        ) {
                            Ok(log_path) => {
                                println!(
                                    "      {}: {}",
                                    "Log saved to".yellow(),
                                    log_path.display()
                                );
                            }
                            Err(e) => {
                                warn!("Failed to save error log for {}: {}", package_name, e);
                            }
                        }
                    }
                }
            }

            if fail_count > 0 {
                std::process::exit(1);
            }
        }

        Ok(())
    }

    async fn build_single_config(config: BuildConfig) -> Result<crate::types::BuildResult> {
        let mut builder = SkinBuilder::new(config)?;
        builder.build().await
    }

    fn save_failure_log(
        package_name: &str,
        errors: &[String],
        output_dir: &Path,
    ) -> Result<PathBuf> {
        use std::io::Write;

        // Create logs directory if it doesn't exist
        let logs_dir = output_dir.join(".logs");
        std::fs::create_dir_all(&logs_dir)?;

        // Generate log file name with timestamp
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let log_filename = format!("build_failure_{}_{}.log", package_name, timestamp);
        let log_path = logs_dir.join(&log_filename);

        // Write log content
        let mut log_file = std::fs::File::create(&log_path)?;
        writeln!(log_file, "Build Failure Log")?;
        writeln!(log_file, "==================")?;
        writeln!(log_file, "Package: {}", package_name)?;
        writeln!(
            log_file,
            "Time: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        )?;
        writeln!(log_file, "\nErrors:")?;
        writeln!(log_file, "--------")?;

        for (i, error) in errors.iter().enumerate() {
            writeln!(log_file, "\n{}. {}", i + 1, error)?;
        }

        Ok(log_path)
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

        // Create template AndroidManifest.xml if it doesn't exist
        let manifest_path = dir.join("src/main/AndroidManifest.xml");
        if !manifest_path.exists() {
            std::fs::create_dir_all(
                manifest_path
                    .parent()
                    .expect("manifest path must have parent directory"),
            )?;
            // Note: uses-sdk is deprecated in modern Gradle-based Android development,
            // but is appropriate here as ASB builds APKs directly with aapt2, not Gradle
            let manifest_content = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.example.skin">
    
    <uses-sdk android:minSdkVersion="26" />
    
    <application
        android:label="@string/app_name">
    </application>
    
</manifest>
"#;
            std::fs::write(&manifest_path, manifest_content)?;
            println!(
                "{}",
                format!("✓ Template manifest created: {}", manifest_path.display()).green()
            );
        }

        // Create template resource directory structure
        let res_dir = dir.join("src/main/res");

        // Create values directory with sample colors and strings
        let values_dir = res_dir.join("values");
        if !values_dir.exists() {
            std::fs::create_dir_all(&values_dir)?;

            // Create colors.xml
            let colors_content = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="colorPrimary">#6200EE</color>
    <color name="colorPrimaryDark">#3700B3</color>
    <color name="colorAccent">#03DAC5</color>
</resources>
"#;
            std::fs::write(values_dir.join("colors.xml"), colors_content)?;
            println!(
                "{}",
                format!(
                    "✓ Template colors created: {}",
                    values_dir.join("colors.xml").display()
                )
                .green()
            );

            // Create strings.xml
            let strings_content = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">Skin Package</string>
</resources>
"#;
            std::fs::write(values_dir.join("strings.xml"), strings_content)?;
            println!(
                "{}",
                format!(
                    "✓ Template strings created: {}",
                    values_dir.join("strings.xml").display()
                )
                .green()
            );
        }

        // Create mipmap-anydpi-v26 directory for adaptive icon (with proper version qualifier)
        let mipmap_dir = res_dir.join("mipmap-anydpi-v26");
        if !mipmap_dir.exists() {
            std::fs::create_dir_all(&mipmap_dir)?;
            let ic_launcher_content = r#"<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/colorPrimary"/>
    <foreground android:drawable="@color/colorAccent"/>
</adaptive-icon>
"#;
            std::fs::write(mipmap_dir.join("ic_launcher.xml"), ic_launcher_content)?;
            println!(
                "{}",
                format!(
                    "✓ Template launcher icon created: {}",
                    mipmap_dir.join("ic_launcher.xml").display()
                )
                .green()
            );
        }

        println!(
            "{}",
            format!("✓ Configuration file created: {}", config_path.display()).green()
        );
        println!(
            "\n{}",
            "Default configuration uses standard Android project structure:".cyan()
        );
        println!("  {}: src/main/res/", "Resources".white());
        println!("  {}: src/main/AndroidManifest.xml", "Manifest".white());
        println!("  {}: build/outputs/skin/", "Output".white());
        println!("\n{}", "Edit the configuration file and run:".cyan());
        println!("  {}", "asb build".white());
        println!("\n{}", "Or simply run 'asb build' without config (uses defaults or ./asb.config.json if exists)".cyan());

        Ok(())
    }
}
