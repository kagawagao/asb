import { execSync, spawn } from 'child_process';
import * as path from 'path';
import * as fs from 'fs-extra';
import * as os from 'os';
import { CompileResult, LinkResult } from '../types';

/**
 * Utility class for interacting with aapt2
 */
export class Aapt2Util {
  private aapt2Path: string;

  constructor(aapt2Path?: string) {
    this.aapt2Path = aapt2Path || this.findAapt2();
  }

  /**
   * Find aapt2 binary in the system
   */
  private findAapt2(): string {
    // Try common locations
    const possiblePaths = [
      'aapt2', // In PATH
      path.join(process.env.ANDROID_HOME || '', 'build-tools'),
    ];

    // Check if aapt2 is in PATH
    try {
      const command = os.platform() === 'win32' ? 'where aapt2' : 'which aapt2';
      const result = execSync(command, {
        encoding: 'utf8',
        stdio: ['pipe', 'pipe', 'ignore'],
      });
      if (result.trim()) {
        return result.trim().split('\n')[0];
      }
    } catch (e) {
      // Continue searching
    }

    // Check Android SDK build-tools
    if (process.env.ANDROID_HOME) {
      const buildToolsDir = path.join(process.env.ANDROID_HOME, 'build-tools');
      if (fs.existsSync(buildToolsDir)) {
        const versions = fs.readdirSync(buildToolsDir).sort().reverse();
        for (const version of versions) {
          const aapt2Path = path.join(
            buildToolsDir,
            version,
            os.platform() === 'win32' ? 'aapt2.exe' : 'aapt2'
          );
          if (fs.existsSync(aapt2Path)) {
            return aapt2Path;
          }
        }
      }
    }

    throw new Error(
      'aapt2 not found. Please install Android SDK and set ANDROID_HOME environment variable, or provide aapt2Path in config.'
    );
  }

  /**
   * Get aapt2 version
   */
  getVersion(): string {
    try {
      const result = execSync(`"${this.aapt2Path}" version`, {
        encoding: 'utf8',
      });
      return result.trim();
    } catch (e) {
      return 'Unknown';
    }
  }

  /**
   * Compile resource files to .flat format
   */
  async compile(resourceFiles: string[], outputDir: string): Promise<CompileResult> {
    const flatFiles: string[] = [];
    const errors: string[] = [];

    // Ensure output directory exists
    await fs.ensureDir(outputDir);

    for (const resourceFile of resourceFiles) {
      try {
        const relativePath = path.basename(resourceFile);
        const outputFile = path.join(
          outputDir,
          relativePath.replace(/\.[^.]+$/, '.flat')
        );

        const args = ['compile', '-o', outputDir, resourceFile];

        await this.executeCommand(args);
        
        // Check if output file was created
        const expectedFlat = path.join(
          outputDir,
          path.basename(resourceFile) + '.flat'
        );
        if (await fs.pathExists(expectedFlat)) {
          flatFiles.push(expectedFlat);
        }
      } catch (e: any) {
        errors.push(`Failed to compile ${resourceFile}: ${e.message}`);
      }
    }

    return {
      success: errors.length === 0,
      flatFiles,
      errors: errors.length > 0 ? errors : undefined,
    };
  }

  /**
   * Compile a directory of resources
   */
  async compileDir(resourceDir: string, outputDir: string): Promise<CompileResult> {
    const args = ['compile', '--dir', resourceDir, '-o', outputDir];

    try {
      await this.executeCommand(args);
      
      // List all .flat files in output directory
      const flatFiles: string[] = [];
      if (await fs.pathExists(outputDir)) {
        const files = await fs.readdir(outputDir);
        for (const file of files) {
          if (file.endsWith('.flat')) {
            flatFiles.push(path.join(outputDir, file));
          }
        }
      }

      return {
        success: true,
        flatFiles,
      };
    } catch (e: any) {
      return {
        success: false,
        flatFiles: [],
        errors: [e.message],
      };
    }
  }

  /**
   * Link compiled resources into an APK
   */
  async link(
    flatFiles: string[],
    manifestPath: string,
    androidJar: string,
    outputApk: string,
    options: {
      packageName?: string;
      versionCode?: number;
      versionName?: string;
      additionalArgs?: string[];
    } = {}
  ): Promise<LinkResult> {
    try {
      const args = [
        'link',
        '--manifest',
        manifestPath,
        '-I',
        androidJar,
        '-o',
        outputApk,
        '--auto-add-overlay',
        '--no-version-vectors',
      ];

      // Add version information
      if (options.versionCode) {
        args.push('--version-code', options.versionCode.toString());
      }
      if (options.versionName) {
        args.push('--version-name', options.versionName);
      }

      // Add package name rename if needed
      if (options.packageName) {
        args.push('--rename-manifest-package', options.packageName);
      }

      // Add additional arguments
      if (options.additionalArgs) {
        args.push(...options.additionalArgs);
      }

      // Add flat files
      for (const flatFile of flatFiles) {
        args.push(flatFile);
      }

      await this.executeCommand(args);

      return {
        success: true,
        apkPath: outputApk,
      };
    } catch (e: any) {
      return {
        success: false,
        errors: [e.message],
      };
    }
  }

  /**
   * Execute aapt2 command
   */
  private executeCommand(args: string[]): Promise<void> {
    return new Promise((resolve, reject) => {
      const child = spawn(this.aapt2Path, args, {
        stdio: ['ignore', 'pipe', 'pipe'],
      });

      let stdout = '';
      let stderr = '';

      child.stdout?.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr?.on('data', (data) => {
        stderr += data.toString();
      });

      child.on('close', (code) => {
        if (code === 0) {
          resolve();
        } else {
          reject(new Error(stderr || stdout || `aapt2 exited with code ${code}`));
        }
      });

      child.on('error', (err) => {
        reject(err);
      });
    });
  }
}
