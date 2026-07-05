import { spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const result = spawnSync(
  process.execPath,
  [path.join(root, 'scripts', 'tsc.mjs'), '-p', path.join(root, 'tsconfig.json'), '--noEmit'],
  {
    cwd: root,
    stdio: 'inherit',
  },
);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
