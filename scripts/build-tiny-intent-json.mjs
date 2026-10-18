import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { TINY_INTENT_MODEL } from '../src/intent/tiny-model.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const outputPath = path.join(root, 'rust', 'kalandar', 'resources', 'tiny_intent_model.json');

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify(TINY_INTENT_MODEL)}\n`);
console.log(`wrote ${path.relative(root, outputPath)}`);
