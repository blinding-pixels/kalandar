#!/usr/bin/env node
// Rebuilds resources/capture-parser from retrained ONNX bundle directories.
//
// node --experimental-strip-types scripts/build-capture-parser-resources.mjs \
//   --bundles /path/to/bundles/repair [--check] [--coreai-artifacts .build/coreai-artifacts-v2]
//
// For task, calendar and reminder it copies model.onnx and config.json, copies
// sequence_decoder.json for BIO families, rewrites each family manifest.json
// (JavaScript format), rewrites the suite manifest.json (Rust format), and
// updates TRUSTED_CAPTURE_PARSER_MANIFEST_SHA256. Tokenizer files must already
// match the shipped ones. The intent bundle only receives the new suite version.
// With --check nothing is written and the script fails if any output differs.
// With --coreai-artifacts it also adds decoding metadata to exported Core AI
// family manifests and copies sequence_decoder.json beside BIO models.

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { computeArtifactId } from '../src/runtime/artifacts.ts';
import { validateBioLabels, validateSequenceDecoderSidecar } from '../src/runtime/sequence-decoder.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const DEFAULTS = {
  bundles: process.env.KALANDAR_MODEL_BUNDLES || '',
  resources: path.join(root, 'resources', 'capture-parser'),
  artifactsRs: path.join(root, 'rust', 'kalandar', 'src', 'artifacts.rs'),
  suiteVersion: 'kalandar-capture-parser-onnx-v2',
  decoding: { task: 'argmax', calendar: 'bio', reminder: 'argmax' },
};

const POSTPROCESS_VERSIONS = {
  task: 'task-postprocess-v2',
  calendar: 'calendar-event-postprocess-v2',
  reminder: 'reminder-postprocess-v2',
};

const FAMILIES = ['task', 'calendar', 'reminder'];
const COPIED_FILES = ['model.onnx', 'config.json'];
const MATCHING_FILES = ['vocab.txt', 'tokenizer_config.json'];
const FILE_ORDER = [
  ['model', 'model.onnx'],
  ['config', 'config.json'],
  ['tokenizer_config', 'tokenizer_config.json'],
  ['special_tokens_map', 'special_tokens_map.json'],
  ['vocab', 'vocab.txt'],
];
const SEQUENCE_DECODER_FILE = 'sequence_decoder.json';

const parseArgs = (argv) => {
  const options = { ...DEFAULTS, decoding: { ...DEFAULTS.decoding }, check: false, coreaiArtifacts: null };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    const next = () => {
      index += 1;
      if (index >= argv.length) {
        throw new Error(`${flag} requires a value`);
      }
      return argv[index];
    };
    if (flag === '--bundles') {
      options.bundles = path.resolve(next());
    } else if (flag === '--resources') {
      options.resources = path.resolve(next());
    } else if (flag === '--suite-version') {
      options.suiteVersion = next();
    } else if (flag === '--decoding') {
      for (const pair of next().split(',')) {
        const [family, decoding] = pair.split('=');
        if (!FAMILIES.includes(family) || !['argmax', 'bio'].includes(decoding)) {
          throw new Error(`Invalid --decoding entry ${pair}`);
        }
        options.decoding[family] = decoding;
      }
    } else if (flag === '--coreai-artifacts') {
      options.coreaiArtifacts = path.resolve(next());
    } else if (flag === '--check') {
      options.check = true;
    } else {
      throw new Error(`Unknown argument ${flag}`);
    }
  }
  return options;
};

const sha256Hex = (bytes) => createHash('sha256').update(bytes).digest('hex');

const readJson = (filePath) => JSON.parse(fs.readFileSync(filePath, 'utf8'));

const assert = (condition, message) => {
  if (!condition) {
    throw new Error(message);
  }
};

const configLabels = (config) => {
  const count = Object.keys(config.id2label).length;
  const labels = Array.from({ length: count }, (_, id) => config.id2label[String(id)]);
  labels.forEach((label, id) => {
    assert(typeof label === 'string', `config.json id2label is missing id ${id}`);
    assert(config.label2id[label] === id, `config.json label2id disagrees for ${label}`);
  });
  return labels;
};

// Mirrors ArtifactIdFingerprint and validate_bundle_artifact_id in rust/kalandar/src/artifacts.rs.
const rustArtifactId = (bundle) => {
  const tokenizer = bundle.tokenizer
    ? {
        kind: bundle.tokenizer.kind,
        doLowerCase: bundle.tokenizer.doLowerCase,
        unkToken: bundle.tokenizer.unkToken,
        clsToken: bundle.tokenizer.clsToken,
        sepToken: bundle.tokenizer.sepToken,
        padToken: bundle.tokenizer.padToken,
        maxSequenceLength: bundle.tokenizer.maxSequenceLength,
      }
    : null;
  const files = [...bundle.files]
    .sort((left, right) => (left.logicalName < right.logicalName ? -1 : left.logicalName > right.logicalName ? 1 : 0))
    .map((file) => ({ logicalName: file.logicalName, path: file.path, sha256: file.sha256, sizeBytes: file.sizeBytes }));
  const fingerprint = {
    parser: bundle.parser,
    runtimeKind: bundle.runtimeKind,
    tensorContractVersion: bundle.tensorContractVersion,
    normalizationVersion: bundle.normalizationVersion,
    postprocessVersion: bundle.postprocessVersion,
    tokenizer,
    labels: bundle.labels,
    files,
    ...(bundle.decoding === undefined ? {} : { decoding: bundle.decoding }),
  };
  return `sha256:${sha256Hex(Buffer.from(JSON.stringify(fingerprint), 'utf8'))}`;
};

const INLINE_PATHS = [/^tensor_contract\.inputs\.\d+$/, /^tensor_contract\.outputs$/, /^files\.[^.]+$/];

// Matches the hand-formatted style of the shipped family manifests.
const formatFamilyManifest = (value) => {
  const inline = (entry) => {
    if (Array.isArray(entry)) {
      return `[${entry.map(inline).join(', ')}]`;
    }
    if (entry && typeof entry === 'object') {
      return `{ ${Object.entries(entry)
        .map(([key, nested]) => `${JSON.stringify(key)}: ${inline(nested)}`)
        .join(', ')} }`;
    }
    return JSON.stringify(entry);
  };
  const format = (entry, keyPath, depth) => {
    if (INLINE_PATHS.some((pattern) => pattern.test(keyPath))) {
      return inline(entry);
    }
    const pad = '  '.repeat(depth + 1);
    const closePad = '  '.repeat(depth);
    if (Array.isArray(entry)) {
      return `[\n${entry.map((nested, index) => `${pad}${format(nested, `${keyPath}.${index}`, depth + 1)}`).join(',\n')}\n${closePad}]`;
    }
    if (entry && typeof entry === 'object') {
      const prefix = keyPath ? `${keyPath}.` : '';
      return `{\n${Object.entries(entry)
        .map(([key, nested]) => `${pad}${JSON.stringify(key)}: ${format(nested, `${prefix}${key}`, depth + 1)}`)
        .join(',\n')}\n${closePad}}`;
    }
    return JSON.stringify(entry);
  };
  return `${format(value, '', 0)}\n`;
};

const sortKeysDeep = (value) => {
  if (Array.isArray(value)) {
    return value.map(sortKeysDeep);
  }
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, sortKeysDeep(value[key])]),
    );
  }
  return value;
};

const main = () => {
  const options = parseArgs(process.argv.slice(2));
  if (!options.bundles) {
    throw new Error('Pass --bundles <dir> or set KALANDAR_MODEL_BUNDLES to the directory holding the task, calendar and reminder bundles.');
  }
  const writes = new Map();
  const removals = [];
  const stage = (filePath, content) => writes.set(filePath, typeof content === 'string' ? Buffer.from(content, 'utf8') : content);
  const pendingBytes = (filePath) => writes.get(filePath) ?? fs.readFileSync(filePath);

  const suitePath = path.join(options.resources, 'manifest.json');
  const suite = readJson(suitePath);
  const intent = suite.bundles.find((bundle) => bundle.parser === 'intent');
  assert(intent && rustArtifactId(intent) === intent.artifactId, 'Rust artifact id reimplementation does not reproduce the intent bundle id');

  const summary = { suiteVersion: options.suiteVersion, families: {} };

  for (const family of FAMILIES) {
    const bundleDir = path.join(options.bundles, family);
    const familyDir = path.join(options.resources, family);
    const decoding = options.decoding[family];

    for (const fileName of MATCHING_FILES) {
      const shipped = fs.readFileSync(path.join(familyDir, fileName));
      const candidate = fs.readFileSync(path.join(bundleDir, fileName));
      assert(shipped.equals(candidate), `${family}/${fileName} differs from the shipped tokenizer file`);
    }
    const shippedSpecial = readJson(path.join(familyDir, 'special_tokens_map.json'));
    const candidateSpecial = readJson(path.join(bundleDir, 'special_tokens_map.json'));
    for (const [key, token] of Object.entries(shippedSpecial)) {
      const candidateToken = candidateSpecial[key];
      const content = typeof candidateToken === 'string' ? candidateToken : candidateToken?.content;
      assert(content === token, `${family}/special_tokens_map.json changes ${key}`);
    }

    const suiteBundle = suite.bundles.find((bundle) => bundle.parser === family);
    assert(suiteBundle, `Suite manifest is missing the ${family} bundle`);
    const labels = configLabels(readJson(path.join(bundleDir, 'config.json')));
    assert(JSON.stringify(labels) === JSON.stringify(suiteBundle.labels), `${family} labels differ from the shipped label set`);
    validateBioLabels(labels);

    for (const fileName of COPIED_FILES) {
      stage(path.join(familyDir, fileName), fs.readFileSync(path.join(bundleDir, fileName)));
    }
    const sidecarTarget = path.join(familyDir, SEQUENCE_DECODER_FILE);
    if (decoding === 'bio') {
      const sidecarBytes = fs.readFileSync(path.join(bundleDir, SEQUENCE_DECODER_FILE));
      validateSequenceDecoderSidecar(JSON.parse(sidecarBytes.toString('utf8')), labels);
      stage(sidecarTarget, sidecarBytes);
    } else if (fs.existsSync(sidecarTarget)) {
      removals.push(sidecarTarget);
    }

    const entryFor = (fileName) => {
      const bytes = pendingBytes(path.join(familyDir, fileName));
      return { path: fileName, sha256: sha256Hex(bytes), size_bytes: bytes.byteLength };
    };
    const files = Object.fromEntries(FILE_ORDER.map(([logicalName, fileName]) => [logicalName, entryFor(fileName)]));
    if (decoding === 'bio') {
      files.sequence_decoder = entryFor(SEQUENCE_DECODER_FILE);
    }

    const previous = readJson(path.join(familyDir, 'manifest.json'));
    const { artifact_id: previousId, ...previousBase } = previous;
    assert(computeArtifactId(previousBase) === previousId, `${family}/manifest.json artifact_id does not verify before regeneration`);
    const manifestBase = {
      manifest_version: previous.manifest_version,
      parser: family,
      suite_version: options.suiteVersion,
      tensor_contract_version: previous.tensor_contract_version,
      normalization_version: previous.normalization_version,
      postprocess_version: POSTPROCESS_VERSIONS[family],
      decoding,
      tokenizer: previous.tokenizer,
      tensor_contract: previous.tensor_contract,
      files,
    };
    const artifactId = computeArtifactId(manifestBase);
    const { manifest_version, parser, suite_version, ...rest } = manifestBase;
    stage(
      path.join(familyDir, 'manifest.json'),
      formatFamilyManifest({ manifest_version, parser, suite_version, artifact_id: artifactId, ...rest }),
    );

    const rustFiles = Object.entries(files).map(([logicalName, entry]) => ({
      logicalName,
      path: `${family}/${entry.path}`,
      sha256: entry.sha256,
      sizeBytes: entry.size_bytes,
    }));
    const rustBundle = {
      parser: suiteBundle.parser,
      runtimeKind: suiteBundle.runtimeKind,
      artifactId: '',
      tensorContractVersion: suiteBundle.tensorContractVersion,
      normalizationVersion: suiteBundle.normalizationVersion,
      postprocessVersion: POSTPROCESS_VERSIONS[family],
      decoding,
      tokenizer: suiteBundle.tokenizer,
      labels: suiteBundle.labels,
      files: rustFiles,
    };
    rustBundle.artifactId = rustArtifactId(rustBundle);
    suite.bundles[suite.bundles.indexOf(suiteBundle)] = rustBundle;

    summary.families[family] = {
      decoding,
      javascriptArtifactId: artifactId,
      rustArtifactId: rustBundle.artifactId,
      modelSha256: files.model.sha256,
      modelSizeBytes: files.model.size_bytes,
      ...(files.sequence_decoder ? { sequenceDecoderSha256: files.sequence_decoder.sha256 } : {}),
    };
  }

  suite.suiteVersion = options.suiteVersion;
  const suiteText = `${JSON.stringify(suite, null, 2)}\n`;
  stage(suitePath, suiteText);
  const suiteSha = sha256Hex(Buffer.from(suiteText, 'utf8'));
  summary.suiteManifestSha256 = suiteSha;

  const artifactsRs = fs.readFileSync(options.artifactsRs, 'utf8');
  const trustedPattern = /(pub const TRUSTED_CAPTURE_PARSER_MANIFEST_SHA256: &str =\s*")[0-9a-f]{64}(";)/;
  assert(trustedPattern.test(artifactsRs), 'Could not find TRUSTED_CAPTURE_PARSER_MANIFEST_SHA256 in artifacts.rs');
  stage(options.artifactsRs, artifactsRs.replace(trustedPattern, `$1${suiteSha}$2`));

  if (options.coreaiArtifacts) {
    for (const family of FAMILIES) {
      const familyDir = path.join(options.coreaiArtifacts, family);
      const manifestPath = path.join(familyDir, 'manifest.json');
      const manifest = readJson(manifestPath);
      const decoding = options.decoding[family];
      const labels = Object.keys(manifest.labels)
        .map(Number)
        .sort((left, right) => left - right)
        .map((id) => manifest.labels[String(id)]);
      validateBioLabels(labels);
      manifest.decoding = decoding;
      delete manifest.sequence_decoder;
      const sidecarTarget = path.join(familyDir, SEQUENCE_DECODER_FILE);
      if (decoding === 'bio') {
        const sidecarBytes = fs.readFileSync(path.join(options.bundles, family, SEQUENCE_DECODER_FILE));
        validateSequenceDecoderSidecar(JSON.parse(sidecarBytes.toString('utf8')), labels);
        stage(sidecarTarget, sidecarBytes);
        manifest.sequence_decoder = SEQUENCE_DECODER_FILE;
      } else if (fs.existsSync(sidecarTarget)) {
        removals.push(sidecarTarget);
      }
      stage(manifestPath, `${JSON.stringify(sortKeysDeep(manifest), null, 2)}\n`);
    }
  }

  if (options.check) {
    const stale = [...writes.entries()]
      .filter(([filePath, bytes]) => !fs.existsSync(filePath) || !fs.readFileSync(filePath).equals(bytes))
      .map(([filePath]) => path.relative(root, filePath))
      .concat(removals.map((filePath) => path.relative(root, filePath)));
    if (stale.length > 0) {
      console.error(`Capture parser resources are out of date:\n${stale.map((entry) => `- ${entry}`).join('\n')}`);
      process.exit(1);
    }
  } else {
    for (const [filePath, bytes] of writes) {
      fs.writeFileSync(filePath, bytes);
    }
    for (const filePath of removals) {
      fs.rmSync(filePath);
    }
  }

  console.log(JSON.stringify(summary, null, 2));
};

main();
