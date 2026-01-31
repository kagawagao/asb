import * as fs from 'fs-extra';
import * as path from 'path';
import * as crypto from 'crypto';

interface CacheEntry {
  hash: string;
  timestamp: number;
  flatFile: string;
}

interface CacheData {
  version: string;
  entries: Record<string, CacheEntry>;
}

/**
 * Utility class for managing build cache for incremental builds
 */
export class CacheUtil {
  private cacheDir: string;
  private cacheFile: string;
  private cache: CacheData;

  constructor(cacheDir: string) {
    this.cacheDir = cacheDir;
    this.cacheFile = path.join(cacheDir, 'build-cache.json');
    this.cache = {
      version: '1.0',
      entries: {},
    };
  }

  /**
   * Initialize cache
   */
  async init(): Promise<void> {
    await fs.ensureDir(this.cacheDir);
    
    if (await fs.pathExists(this.cacheFile)) {
      try {
        const data = await fs.readJson(this.cacheFile);
        if (data.version === this.cache.version) {
          this.cache = data;
        }
      } catch (e) {
        // If cache cannot be read or parsed (corrupted, invalid JSON, etc.), start fresh
        this.cache = {
          version: '1.0',
          entries: {},
        };
      }
    }
  }

  /**
   * Calculate file hash
   */
  private async calculateHash(filePath: string): Promise<string> {
    const content = await fs.readFile(filePath);
    return crypto.createHash('sha256').update(content).digest('hex');
  }

  /**
   * Check if a file needs recompilation
   */
  async needsRecompile(resourceFile: string): Promise<boolean> {
    const entry = this.cache.entries[resourceFile];
    
    if (!entry) {
      return true;
    }

    // Check if the flat file still exists
    if (!(await fs.pathExists(entry.flatFile))) {
      return true;
    }

    // Check if file has been modified
    try {
      const currentHash = await this.calculateHash(resourceFile);
      return currentHash !== entry.hash;
    } catch (e) {
      return true;
    }
  }

  /**
   * Get cached flat file for a resource
   */
  getCachedFlatFile(resourceFile: string): string | null {
    const entry = this.cache.entries[resourceFile];
    return entry ? entry.flatFile : null;
  }

  /**
   * Update cache entry
   */
  async updateEntry(resourceFile: string, flatFile: string): Promise<void> {
    try {
      const hash = await this.calculateHash(resourceFile);
      this.cache.entries[resourceFile] = {
        hash,
        timestamp: Date.now(),
        flatFile,
      };
    } catch (e) {
      // Ignore errors in cache update
    }
  }

  /**
   * Save cache to disk
   */
  async save(): Promise<void> {
    try {
      await fs.writeJson(this.cacheFile, this.cache, { spaces: 2 });
    } catch (e) {
      // Ignore save errors
    }
  }

  /**
   * Clear cache
   */
  async clear(): Promise<void> {
    this.cache.entries = {};
    try {
      await fs.remove(this.cacheFile);
    } catch (e) {
      // Ignore errors
    }
  }

  /**
   * Get all cached flat files
   */
  getAllCachedFlatFiles(): string[] {
    return Object.values(this.cache.entries).map((entry) => entry.flatFile);
  }
}
