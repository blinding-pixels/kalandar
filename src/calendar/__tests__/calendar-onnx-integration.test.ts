/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { createCalendarParser, eventResultToPlanningObject } from '../index.ts';
import {
  ARTIFACT_MANIFEST_VERSION,
  CALENDAR_NORMALIZATION_VERSION,
  CALENDAR_POSTPROCESS_VERSION,
  CALENDAR_TENSOR_CONTRACT_VERSION,
  computeArtifactId,
  loadMlArtifact,
} from '../../runtime/index.ts';
import { createLocalOnnxInferenceAdapter } from '../../runtime/onnx/index.ts';
import type { CalendarArtifactManifest } from '../../runtime/types.ts';

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');

const hashFile = async (absolutePath: string): Promise<{ path: string; sha256: string; size_bytes: number }> => {
  const bytes = await fs.readFile(absolutePath);
  return {
    path: path.basename(absolutePath),
    sha256: createHash('sha256').update(bytes).digest('hex'),
    size_bytes: bytes.byteLength,
  };
};

describe('calendar local ONNX integration', () => {
  test('runs the checked-in artifact end to end when onnxruntime-node is available', async (t) => {
    try {
      const dynamicImport = new Function('specifier', 'return import(specifier)') as (specifier: string) => Promise<unknown>;
      await dynamicImport('onnxruntime-node');
    } catch {
      t.skip('onnxruntime-node is not installed in this environment');
      return;
    }

    const sourceFiles = {
      model: path.join(workspaceRoot, 'training/data/artifacts/calendar_distilled_minilm_l12_384_latest_onnx/quint8_per_channel.onnx'),
      config: path.join(workspaceRoot, 'training/data/artifacts/calendar_distilled_minilm_l12_384_latest/best/config.json'),
      tokenizer_config: path.join(workspaceRoot, 'training/data/artifacts/calendar_distilled_minilm_l12_384_latest/best/tokenizer_config.json'),
      special_tokens_map: path.join(workspaceRoot, 'training/data/artifacts/calendar_distilled_minilm_l12_384_latest/best/special_tokens_map.json'),
      vocab: path.join(workspaceRoot, 'training/data/artifacts/calendar_distilled_minilm_l12_384_latest/best/vocab.txt'),
      tokenizer_json: path.join(workspaceRoot, 'training/data/artifacts/calendar_distilled_minilm_l12_384_latest/best/tokenizer.json'),
    };

    const bundleDir = await fs.mkdtemp(path.join(os.tmpdir(), 'kalandar-calendar-onnx-'));
    try {
      for (const absolutePath of Object.values(sourceFiles)) {
        await fs.symlink(absolutePath, path.join(bundleDir, path.basename(absolutePath)));
      }

      const manifestBase = {
        manifest_version: ARTIFACT_MANIFEST_VERSION,
        parser: 'calendar' as const,
        suite_version: 'suite-real-calendar-onnx',
        tensor_contract_version: CALENDAR_TENSOR_CONTRACT_VERSION,
        normalization_version: CALENDAR_NORMALIZATION_VERSION,
        postprocess_version: CALENDAR_POSTPROCESS_VERSION,
        tokenizer: {
          kind: 'bert_wordpiece' as const,
          do_lower_case: true,
          unk_token: '[UNK]',
          cls_token: '[CLS]',
          sep_token: '[SEP]',
          pad_token: '[PAD]',
          max_sequence_length: 512,
        },
        tensor_contract: {
          inputs: [
            { name: 'input_ids', dtype: 'int64' as const, rank: 2 },
            { name: 'attention_mask', dtype: 'int64' as const, rank: 2 },
          ],
          outputs: [{ name: 'logits', dtype: 'float32' as const, rank: 3 }],
          max_sequence_length: 512,
          padding: 'none' as const,
          truncation: 'tail' as const,
          empty_input_behavior: 'return_empty_result' as const,
          whitespace_input_behavior: 'return_empty_result' as const,
          oversized_input_behavior: 'truncate' as const,
        },
        files: {
          model: await hashFile(path.join(bundleDir, 'quint8_per_channel.onnx')),
          config: await hashFile(path.join(bundleDir, 'config.json')),
          tokenizer_config: await hashFile(path.join(bundleDir, 'tokenizer_config.json')),
          special_tokens_map: await hashFile(path.join(bundleDir, 'special_tokens_map.json')),
          vocab: await hashFile(path.join(bundleDir, 'vocab.txt')),
          tokenizer_json: await hashFile(path.join(bundleDir, 'tokenizer.json')),
        },
      };

      const manifest: CalendarArtifactManifest = {
        ...manifestBase,
        artifact_id: computeArtifactId(manifestBase),
      };
      await fs.writeFile(path.join(bundleDir, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

      const artifact = await loadMlArtifact({ kind: 'directory', path: bundleDir }, 'calendar');
      const parser = await createCalendarParser({
        mode: 'ml',
        artifact,
        inference: createLocalOnnxInferenceAdapter(),
      });

      const result = await parser.parse('Lunch with Sara every other Tuesday starting next month', {
        now: '2026-04-18T10:00:00-04:00',
        timezone: 'America/New_York',
      });

      assert.equal(result.meta.runtime?.mode, 'ml');
      assert.ok(result.fields.title);
      const planning = eventResultToPlanningObject(result);
      assert.equal(planning.kind, 'event');

      await parser.dispose();
    } finally {
      await fs.rm(bundleDir, { recursive: true, force: true });
    }
  });
});
