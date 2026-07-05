/// <reference types="node" />

import { strict as assert } from 'node:assert';
import fs from 'node:fs/promises';
import path from 'node:path';
import { describe, test } from 'node:test';

import { loadMlArtifact } from '../index.ts';
import { createCalendarTestBundle, createReminderTestBundle, createTaskTestBundle } from './helpers.ts';

describe('runtime artifact loader', () => {
  test('accepts a valid calendar artifact manifest', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir });
      assert.equal(artifact.parser, 'calendar');
      assert.equal(artifact.artifact_id, bundle.manifest.artifact_id);
      assert.equal(artifact.manifest.tensor_contract.inputs[0].name, 'input_ids');
      assert.equal(artifact.tokenizer.encode('lunch with sara').inputIds.length, 5);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects parser mismatch', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.parser = 'task';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'calendar'), /parser mismatch/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects missing required files', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      await fs.unlink(path.join(bundle.dir, 'vocab.txt'));
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }), /vocab/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects bad or missing artifact_id', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.artifact_id = 'bad-id';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }), /artifact_id/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects tensor contract version mismatch', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.tensor_contract_version = 'calendar-bert-token-classification-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }), /tensor_contract_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects normalization version mismatch', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.normalization_version = 'calendar-bert-uncased-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }), /normalization_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects postprocess version mismatch', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.postprocess_version = 'calendar-event-postprocess-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }), /postprocess_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects partial bundles', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      await fs.unlink(path.join(bundle.dir, 'model.onnx'));
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }), /model/i);
    } finally {
      await bundle.cleanup();
    }
  });
});

describe('runtime artifact loader for task', () => {
  test('accepts a valid task artifact manifest', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task');
      assert.equal(artifact.parser, 'task');
      assert.equal(artifact.artifact_id, bundle.manifest.artifact_id);
      assert.equal(artifact.manifest.tensor_contract.inputs[0].name, 'input_ids');
      assert.equal(artifact.tokenizer.encode('finish report tomorrow').inputIds.length, 5);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects parser mismatch', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.parser = 'calendar';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task'), /parser mismatch/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects missing required files', async () => {
    const bundle = await createTaskTestBundle();
    try {
      await fs.unlink(path.join(bundle.dir, 'vocab.txt'));
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task'), /vocab/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects bad or missing artifact_id', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.artifact_id = 'bad-id';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task'), /artifact_id/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects tensor contract version mismatch', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.tensor_contract_version = 'task-bert-token-classification-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task'), /tensor_contract_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects normalization version mismatch', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.normalization_version = 'task-bert-uncased-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task'), /normalization_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects postprocess version mismatch', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.postprocess_version = 'task-postprocess-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task'), /postprocess_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects partial bundles', async () => {
    const bundle = await createTaskTestBundle();
    try {
      await fs.unlink(path.join(bundle.dir, 'model.onnx'));
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task'), /model/i);
    } finally {
      await bundle.cleanup();
    }
  });
});

describe('runtime artifact loader for reminder', () => {
  test('accepts a valid reminder artifact manifest', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder');
      assert.equal(artifact.parser, 'reminder');
      assert.equal(artifact.artifact_id, bundle.manifest.artifact_id);
      assert.equal(artifact.manifest.tensor_contract.inputs[0].name, 'input_ids');
      assert.equal(artifact.tokenizer.encode('water plants tomorrow').inputIds.length, 5);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects parser mismatch', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.parser = 'task';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder'), /parser mismatch/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects missing required files', async () => {
    const bundle = await createReminderTestBundle();
    try {
      await fs.unlink(path.join(bundle.dir, 'vocab.txt'));
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder'), /vocab/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects bad or missing artifact_id', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.artifact_id = 'bad-id';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder'), /artifact_id/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects tensor contract version mismatch', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.tensor_contract_version = 'reminder-bert-token-classification-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder'), /tensor_contract_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects normalization version mismatch', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.normalization_version = 'reminder-bert-uncased-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder'), /normalization_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects postprocess version mismatch', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.postprocess_version = 'reminder-postprocess-v0';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder'), /postprocess_version/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('rejects partial bundles', async () => {
    const bundle = await createReminderTestBundle();
    try {
      await fs.unlink(path.join(bundle.dir, 'model.onnx'));
      await assert.rejects(() => loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder'), /model/i);
    } finally {
      await bundle.cleanup();
    }
  });
});
