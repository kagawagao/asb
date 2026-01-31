#!/usr/bin/env node
import { Command } from 'commander';
import * as fs from 'fs-extra';
import * as path from 'path';
import chalk from 'chalk';
import ora from 'ora';
import { SkinBuilder } from './builder';
import { BuildConfig } from './types';
import { Aapt2Util } from './utils/aapt2';
import { CleanUtil } from './utils/clean';

const program = new Command();

program
  .name('asb')
  .description('Android Skin Builder - Build resource-only skin packages using aapt2')
  .version('1.0.0');

program
  .command('build')
  .description('Build a skin package from resources')
  .option('-c, --config <path>', 'Path to configuration file')
  .option('-r, --resource-dir <path>', 'Path to resources directory')
  .option('-m, --manifest <path>', 'Path to AndroidManifest.xml')
  .option('-o, --output <path>', 'Output directory')
  .option('-p, --package <name>', 'Package name for the skin')
  .option('-a, --android-jar <path>', 'Path to android.jar')
  .option('--aar <paths...>', 'Paths to AAR files to include')
  .option('--aapt2 <path>', 'Path to aapt2 binary')
  .option('--incremental', 'Enable incremental build', false)
  .option('--version-code <number>', 'Version code', parseInt)
  .option('--version-name <string>', 'Version name')
  .action(async (options) => {
    const spinner = ora('Loading configuration...').start();

    try {
      let config: BuildConfig;

      // Load config from file if provided
      if (options.config) {
        const configPath = path.resolve(options.config);
        if (!(await fs.pathExists(configPath))) {
          spinner.fail(chalk.red(`Config file not found: ${configPath}`));
          process.exit(1);
        }
        config = await fs.readJson(configPath);
      } else {
        // Build config from command line options
        if (!options.resourceDir || !options.manifest || !options.output || !options.androidJar) {
          spinner.fail(
            chalk.red(
              'Missing required options. Provide either --config or --resource-dir, --manifest, --output, and --android-jar'
            )
          );
          process.exit(1);
        }

        config = {
          resourceDir: path.resolve(options.resourceDir),
          manifestPath: path.resolve(options.manifest),
          outputDir: path.resolve(options.output),
          androidJar: path.resolve(options.androidJar),
          packageName: options.package || 'com.example.skin',
          aarFiles: options.aar ? options.aar.map((p: string) => path.resolve(p)) : [],
          aapt2Path: options.aapt2 ? path.resolve(options.aapt2) : undefined,
          incremental: options.incremental,
          versionCode: options.versionCode,
          versionName: options.versionName,
        };
      }

      // Validate config
      spinner.text = 'Validating configuration...';
      
      if (!(await fs.pathExists(config.resourceDir))) {
        spinner.fail(chalk.red(`Resource directory not found: ${config.resourceDir}`));
        process.exit(1);
      }

      if (!(await fs.pathExists(config.manifestPath))) {
        spinner.fail(chalk.red(`Manifest file not found: ${config.manifestPath}`));
        process.exit(1);
      }

      if (!(await fs.pathExists(config.androidJar))) {
        spinner.fail(chalk.red(`android.jar not found: ${config.androidJar}`));
        process.exit(1);
      }

      spinner.succeed(chalk.green('Configuration loaded'));

      // Build
      console.log(chalk.blue('\nBuilding skin package...\n'));
      
      const builder = new SkinBuilder(config);
      const result = await builder.build();

      if (result.success) {
        console.log(chalk.green(`\n✓ Skin package built successfully!`));
        console.log(chalk.cyan(`  Output: ${result.apkPath}`));
      } else {
        console.log(chalk.red('\n✗ Build failed:'));
        if (result.errors) {
          result.errors.forEach((error) => {
            console.log(chalk.red(`  - ${error}`));
          });
        }
        process.exit(1);
      }
    } catch (e: any) {
      spinner.fail(chalk.red(`Error: ${e.message}`));
      process.exit(1);
    }
  });

program
  .command('clean')
  .description('Clean build artifacts')
  .option('-c, --config <path>', 'Path to configuration file')
  .option('-o, --output <path>', 'Output directory')
  .action(async (options) => {
    try {
      let outputDir: string;
      let cacheDir: string | undefined;

      if (options.config) {
        const configPath = path.resolve(options.config);
        const config = await fs.readJson(configPath);
        outputDir = config.outputDir;
        cacheDir = config.cacheDir;
      } else if (options.output) {
        outputDir = path.resolve(options.output);
      } else {
        console.log(chalk.red('Please provide either --config or --output'));
        process.exit(1);
      }

      await CleanUtil.clean(outputDir, cacheDir);
      console.log(chalk.green('✓ Build artifacts cleaned'));
    } catch (e: any) {
      console.log(chalk.red(`Error: ${e.message}`));
      process.exit(1);
    }
  });

program
  .command('version')
  .description('Show aapt2 version')
  .action(() => {
    try {
      const aapt2 = new Aapt2Util();
      const version = aapt2.getVersion();
      console.log(chalk.cyan('aapt2 version:'));
      console.log(version);
    } catch (e: any) {
      console.log(chalk.red(`Error: ${e.message}`));
      process.exit(1);
    }
  });

program
  .command('init')
  .description('Initialize a new skin project with sample configuration')
  .option('-d, --dir <path>', 'Project directory', process.cwd())
  .action(async (options) => {
    const projectDir = path.resolve(options.dir);
    const configPath = path.join(projectDir, 'asb.config.json');

    if (await fs.pathExists(configPath)) {
      console.log(chalk.yellow('Configuration file already exists'));
      return;
    }

    const sampleConfig = {
      resourceDir: './res',
      manifestPath: './AndroidManifest.xml',
      outputDir: './build',
      packageName: 'com.example.skin',
      androidJar: '${ANDROID_HOME}/platforms/android-30/android.jar',
      aarFiles: [],
      incremental: true,
      versionCode: 1,
      versionName: '1.0.0',
    };

    await fs.writeJson(configPath, sampleConfig, { spaces: 2 });
    console.log(chalk.green(`✓ Configuration file created: ${configPath}`));
    console.log(chalk.cyan('\nEdit the configuration file and run:'));
    console.log(chalk.white('  asb build --config asb.config.json'));
  });

program.parse();
