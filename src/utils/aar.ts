import * as fs from 'fs-extra';
import * as path from 'path';
import AdmZip from 'adm-zip';
import { AarInfo } from '../types';

/**
 * Utility class for handling AAR files
 */
export class AarUtil {
  /**
   * Extract AAR file to a temporary directory
   */
  async extractAar(aarPath: string, extractDir: string): Promise<AarInfo> {
    // Ensure the AAR file exists
    if (!(await fs.pathExists(aarPath))) {
      throw new Error(`AAR file not found: ${aarPath}`);
    }

    // Create extract directory
    await fs.ensureDir(extractDir);

    try {
      // Extract AAR (which is essentially a ZIP file)
      const zip = new AdmZip(aarPath);
      zip.extractAllTo(extractDir, true);

      // Find resource directory and manifest
      const resDir = path.join(extractDir, 'res');
      const manifestPath = path.join(extractDir, 'AndroidManifest.xml');

      const aarInfo: AarInfo = {
        path: aarPath,
        extractedDir: extractDir,
      };

      if (await fs.pathExists(resDir)) {
        aarInfo.resourceDir = resDir;
      }

      if (await fs.pathExists(manifestPath)) {
        aarInfo.manifestPath = manifestPath;
      }

      return aarInfo;
    } catch (e: any) {
      throw new Error(`Failed to extract AAR file ${aarPath}: ${e.message}`);
    }
  }

  /**
   * Extract multiple AAR files
   */
  async extractAars(aarPaths: string[], baseTempDir: string): Promise<AarInfo[]> {
    const aarInfos: AarInfo[] = [];

    for (let i = 0; i < aarPaths.length; i++) {
      const aarPath = aarPaths[i];
      const extractDir = path.join(baseTempDir, `aar_${i}_${path.basename(aarPath, '.aar')}`);
      const aarInfo = await this.extractAar(aarPath, extractDir);
      aarInfos.push(aarInfo);
    }

    return aarInfos;
  }

  /**
   * Clean up extracted AAR directories
   */
  async cleanupAars(aarInfos: AarInfo[]): Promise<void> {
    for (const aarInfo of aarInfos) {
      try {
        await fs.remove(aarInfo.extractedDir);
      } catch (e) {
        // Ignore cleanup errors
      }
    }
  }
}
