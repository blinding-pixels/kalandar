import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const source = fs.readFileSync(path.join(root, 'src', 'dates', 'wasm', 'bytes.ts'), 'utf8');
const match = source.match(/'([A-Za-z0-9+/=]+)'/);
if (!match) {
  console.error('emit-dates-wasm: src/dates/wasm/bytes.ts has no base64 literal');
  process.exit(1);
}
const target = path.join(root, 'dist', 'dates', 'wasm', 'kalandar_dates_wasm_bg.wasm');
fs.mkdirSync(path.dirname(target), { recursive: true });
fs.writeFileSync(target, Buffer.from(match[1], 'base64'));
console.log(`kalandar-dates wasm: wrote ${path.relative(root, target)}`);
