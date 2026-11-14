import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// Public-directory copies are optional in Vite; this application asset is not.
export function requiredAppIcon() {
  const appIcon = fileURLToPath(new URL('../public/superfactory-icon.png', import.meta.url));
  const pngSignature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  return {
    name: 'required-app-icon',
    apply: 'build',
    buildStart() {
      this.addWatchFile(appIcon);
      const icon = readFileSync(appIcon);
      if (icon.length < 24 || !icon.subarray(0, 8).equals(pngSignature)
        || icon.toString('ascii', 12, 16) !== 'IHDR'
        || icon.readUInt32BE(16) === 0 || icon.readUInt32BE(20) === 0) {
        this.error('Application icon must be a materialized, nonempty PNG.');
      }
    },
    writeBundle(options) {
      const emitted = readFileSync(resolve(options.dir, 'superfactory-icon.png'));
      if (!emitted.equals(readFileSync(appIcon))) {
        this.error('Built application icon differs from its source PNG.');
      }
    },
  };
}
