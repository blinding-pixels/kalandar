import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const findTypescriptCompiler = (start) => {
  let current = start;
  while (true) {
    const candidate = path.join(current, 'node_modules', 'typescript', 'bin', 'tsc');
    if (fs.existsSync(candidate)) {
      return candidate;
    }

    const parent = path.dirname(current);
    if (parent === current) {
      return null;
    }
    current = parent;
  }
};

const tsc = findTypescriptCompiler(root);
if (!tsc) {
  console.error('TypeScript compiler not found. Run "npm install" inside package/ first.');
  process.exit(1);
}

const result = spawnSync(process.execPath, [tsc, ...process.argv.slice(2)], {
  cwd: root,
  stdio: 'inherit',
});

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
