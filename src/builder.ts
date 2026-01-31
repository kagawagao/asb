import * as fs from 'fs-extra';
import * as path from 'path';
import { glob } from 'glob';
import { BuildConfig, CompileResult, LinkResult, AarInfo } from './types';
import { Aapt2Util } from './utils/aapt2';
import { AarUtil } from './utils/aar';
import { CacheUtil } from './utils/cache';
import { CleanUtil } from './utils/clean';

/**
 * Main builder class for Android skin packages
 */
export class SkinBuilder {
  private config: BuildConfig;
  private aapt2: Aapt2Util;
  private aarUtil: AarUtil;
  private cacheUtil?: CacheUtil;

  constructor(config: BuildConfig) {
    this.config = config;
    this.aapt2 = new Aapt2Util(config.aapt2Path);
    this.aarUtil = new AarUtil();

    // Initialize cache if incremental build is enabled
    if (config.incremental) {
      const cacheDir = config.cacheDir || path.join(config.outputDir, '.build-cache');
      this.cacheUtil = new CacheUtil(cacheDir);
    }
  }

  /**
   * Build the skin package
   */
  async build(): Promise<{ success: boolean; apkPath?: string; errors?: string[] }> {
    const errors: string[] = [];

    try {
      // Initialize cache if needed
      if (this.cacheUtil) {
        await this.cacheUtil.init();
      }

      // Ensure output directories exist
      const compiledDir = this.config.compiledDir || path.join(this.config.outputDir, 'compiled');
      await fs.ensureDir(compiledDir);
      await fs.ensureDir(this.config.outputDir);

      // Extract AAR files if provided
      let aarInfos: AarInfo[] = [];
      const tempDir = path.join(this.config.outputDir, '.temp');
      
      if (this.config.aarFiles && this.config.aarFiles.length > 0) {
        console.log('Extracting AAR files...');
        aarInfos = await this.aarUtil.extractAars(this.config.aarFiles, tempDir);
      }

      // Collect all resource directories
      const resourceDirs = [this.config.resourceDir];
      
      // Add AAR resource directories
      for (const aarInfo of aarInfos) {
        if (aarInfo.resourceDir) {
          resourceDirs.push(aarInfo.resourceDir);
        }
      }

      // Add additional resource directories
      if (this.config.additionalResourceDirs) {
        resourceDirs.push(...this.config.additionalResourceDirs);
      }

      // Compile resources
      console.log('Compiling resources...');
      const flatFiles: string[] = [];

      for (const resDir of resourceDirs) {
        if (await fs.pathExists(resDir)) {
          const compileResult = await this.compileResourceDir(resDir, compiledDir);
          
          if (!compileResult.success) {
            errors.push(...(compileResult.errors || []));
          } else {
            flatFiles.push(...compileResult.flatFiles);
          }
        } else {
          console.warn(`Resource directory not found: ${resDir}`);
        }
      }

      if (errors.length > 0) {
        return { success: false, errors };
      }

      if (flatFiles.length === 0) {
        return { success: false, errors: ['No resources to compile'] };
      }

      // Save cache
      if (this.cacheUtil) {
        await this.cacheUtil.save();
      }

      // Link resources into APK
      console.log('Linking resources...');
      const outputApk = path.join(
        this.config.outputDir,
        `skin-${this.config.packageName || 'default'}.apk`
      );

      const linkResult = await this.aapt2.link(
        flatFiles,
        this.config.manifestPath,
        this.config.androidJar,
        outputApk,
        {
          packageName: this.config.packageName,
          versionCode: this.config.versionCode,
          versionName: this.config.versionName,
        }
      );

      // Cleanup AAR extraction directories
      if (aarInfos.length > 0) {
        await this.aarUtil.cleanupAars(aarInfos);
        await fs.remove(tempDir);
      }

      if (!linkResult.success) {
        return { success: false, errors: linkResult.errors };
      }

      return { success: true, apkPath: linkResult.apkPath };
    } catch (e: any) {
      errors.push(e.message);
      return { success: false, errors };
    }
  }

  /**
   * Compile a resource directory
   */
  private async compileResourceDir(
    resDir: string,
    compiledDir: string
  ): Promise<CompileResult> {
    // If incremental build is disabled, compile the whole directory
    if (!this.cacheUtil) {
      return await this.aapt2.compileDir(resDir, compiledDir);
    }

    // For incremental builds, check each file individually
    const resourceFiles = await this.findResourceFiles(resDir);
    const flatFiles: string[] = [];
    const errors: string[] = [];

    for (const resourceFile of resourceFiles) {
      try {
        const needsRecompile = await this.cacheUtil.needsRecompile(resourceFile);
        
        if (needsRecompile) {
          // Compile the file
          const result = await this.aapt2.compile([resourceFile], compiledDir);
          
          if (result.success && result.flatFiles.length > 0) {
            const flatFile = result.flatFiles[0];
            flatFiles.push(flatFile);
            await this.cacheUtil.updateEntry(resourceFile, flatFile);
          } else {
            errors.push(...(result.errors || []));
          }
        } else {
          // Use cached flat file
          const cachedFlat = this.cacheUtil.getCachedFlatFile(resourceFile);
          if (cachedFlat && (await fs.pathExists(cachedFlat))) {
            flatFiles.push(cachedFlat);
          } else {
            // Cache is invalid, recompile
            const result = await this.aapt2.compile([resourceFile], compiledDir);
            if (result.success && result.flatFiles.length > 0) {
              const flatFile = result.flatFiles[0];
              flatFiles.push(flatFile);
              await this.cacheUtil.updateEntry(resourceFile, flatFile);
            }
          }
        }
      } catch (e: any) {
        errors.push(`Error processing ${resourceFile}: ${e.message}`);
      }
    }

    return {
      success: errors.length === 0,
      flatFiles,
      errors: errors.length > 0 ? errors : undefined,
    };
  }

  /**
   * Find all resource files in a directory
   */
  private async findResourceFiles(resDir: string): Promise<string[]> {
    const pattern = path.join(resDir, '**', '*.*').replace(/\\/g, '/');
    const files = await glob(pattern, {
      nodir: true,
      ignore: ['**/.DS_Store', '**/Thumbs.db'],
    });
    return files;
  }

  /**
   * Clean build artifacts
   */
  async clean(): Promise<void> {
    await CleanUtil.clean(this.config.outputDir, this.config.cacheDir);
  }
}
