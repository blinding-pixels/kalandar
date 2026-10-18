import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const crate = path.join(root, 'rust', 'kalandar-dates', 'wasm');
const outDir = path.join(root, 'src', 'dates', 'wasm');

if (!process.argv.includes('--skip-cargo')) {
  const build = spawnSync(path.join(crate, 'build.sh'), [], { cwd: crate, stdio: 'inherit', env: process.env });
  if (build.status !== 0) {
    process.exit(build.status ?? 1);
  }
}

const glue = fs.readFileSync(path.join(crate, 'pkg-web', 'kalandar_dates_wasm.js'), 'utf8');
const wasm = fs.readFileSync(path.join(crate, 'pkg-web', 'kalandar_dates_wasm_bg.wasm'));

fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, 'glue.ts'), `// @ts-nocheck\n${glue.replace(/[ \t]+$/gm, '')}`);
fs.writeFileSync(
  path.join(outDir, 'bytes.ts'),
  `export const KALANDAR_DATES_WASM_BASE64: string =\n  '${wasm.toString('base64')}';\n`,
);
fs.writeFileSync(
  path.join(outDir, 'source.json'),
  `${JSON.stringify({ crate: 'kalandar-dates-wasm', bytes: wasm.length, sha256: (await import('node:crypto')).createHash('sha256').update(wasm).digest('hex') }, null, 2)}\n`,
);
console.log(`kalandar-dates wasm: ${wasm.length} bytes`);
