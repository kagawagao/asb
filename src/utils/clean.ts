import * as fs from 'fs-extra';
import * as path from 'path';

/**
 * Utility class for cleaning build artifacts
 */
export class CleanUtil {
  /**
   * Clean build artifacts from the output directory
   */
  static async clean(outputDir: string, cacheDir?: string): Promise<void> {
    const compiledDir = path.join(outputDir, 'compiled');
    const tempDir = path.join(outputDir, '.temp');
    const defaultCacheDir = path.join(outputDir, '.build-cache');
    const cacheDirToClean = cacheDir || defaultCacheDir;
    
    try {
      // Remove compiled directory
      if (await fs.pathExists(compiledDir)) {
        await fs.remove(compiledDir);
      }
      
      // Remove temp directory
      if (await fs.pathExists(tempDir)) {
        await fs.remove(tempDir);
      }
      
      // Remove cache directory
      if (await fs.pathExists(cacheDirToClean)) {
        await fs.remove(cacheDirToClean);
      }
    } catch (e) {
      // Ignore cleanup errors
    }
  }
}
