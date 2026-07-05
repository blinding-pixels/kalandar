import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const targets = [
  'src',
  'coreai',
  'rust/kalandar/src',
  'rust/kalandar/tests',
  'scripts',
  'package.json',
  'tsconfig.json',
  'tsconfig.build.json',
];
const sourceExtensions = new Set(['.ts', '.mjs', '.py', '.json', '.rs', '.md', '.toml']);
const excludedSegments = new Set(['node_modules', '.build', 'dist', 'target', 'checkpoints', '.venv', '__pycache__']);
const violations = [];

const shouldSkip = (absolutePath) =>
  absolutePath
    .split(path.sep)
    .some((segment) => excludedSegments.has(segment) || segment === 'artifacts' || segment === 'normalized');

const visit = (absolutePath) => {
  if (!fs.existsSync(absolutePath) || shouldSkip(absolutePath)) {
    return;
  }

  const stat = fs.statSync(absolutePath);
  if (stat.isDirectory()) {
    for (const entry of fs.readdirSync(absolutePath)) {
      visit(path.join(absolutePath, entry));
    }
    return;
  }

  if (!sourceExtensions.has(path.extname(absolutePath)) && path.basename(absolutePath) !== 'package.json') {
    return;
  }

  const content = fs.readFileSync(absolutePath, 'utf8');
  const lines = content.split('\n');
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (/\s+$/.test(line) && line.length > 0) {
      violations.push(`${path.relative(root, absolutePath)}:${index + 1} trailing whitespace`);
    }
    if (/\t/.test(line)) {
      violations.push(`${path.relative(root, absolutePath)}:${index + 1} tab character`);
    }
  }
};

for (const target of targets) {
  visit(path.join(root, target));
}

if (violations.length > 0) {
  console.error('Lint violations found:');
  for (const violation of violations) {
    console.error(`- ${violation}`);
  }
  process.exit(1);
}
