/**
 * Configuration for building Android skin packages
 */
export interface BuildConfig {
  /**
   * Path to the resources directory (res/)
   */
  resourceDir: string;

  /**
   * Path to the Android manifest file
   */
  manifestPath: string;

  /**
   * Output directory for the skin package
   */
  outputDir: string;

  /**
   * Package name for the skin package
   */
  packageName: string;

  /**
   * Path to aapt2 binary (optional, will auto-detect if not provided)
   */
  aapt2Path?: string;

  /**
   * Path to Android platform JAR (android.jar)
   */
  androidJar: string;

  /**
   * Additional AAR files to include resources from
   */
  aarFiles?: string[];

  /**
   * Enable incremental build
   */
  incremental?: boolean;

  /**
   * Build cache directory
   */
  cacheDir?: string;

  /**
   * Version code for the skin package
   */
  versionCode?: number;

  /**
   * Version name for the skin package
   */
  versionName?: string;

  /**
   * Additional resource directories
   */
  additionalResourceDirs?: string[];

  /**
   * Compiled resource directory (for intermediate .flat files)
   */
  compiledDir?: string;
}

/**
 * Result of aapt2 compile operation
 */
export interface CompileResult {
  success: boolean;
  flatFiles: string[];
  errors?: string[];
}

/**
 * Result of aapt2 link operation
 */
export interface LinkResult {
  success: boolean;
  apkPath?: string;
  errors?: string[];
}

/**
 * AAR file information
 */
export interface AarInfo {
  path: string;
  resourceDir?: string;
  manifestPath?: string;
  extractedDir: string;
}
