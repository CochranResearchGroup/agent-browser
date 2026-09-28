import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { comparePrior, parseArgs, parseInput } from './p218-compiler-diagnostics.mjs';

function cargoMessage({ packageId = 'agent-browser 0.1.0', file = 'cli/src/lib.rs', code = 'E0432', message = 'unresolved import `gone`', line = 2, reason = 'compiler-message', level = 'error' } = {}) {
  return {
    reason,
    package_id: packageId,
    message: {
      level,
      message,
      code: code ? { code } : null,
      spans: [{ file_name: file, line_start: line, is_primary: true, text: [{ text: 'use gone;' }] }],
    },
  };
}

test('filters dependency chatter and non-error messages, then deduplicates normalized first-party errors', () => {
  const root = '/repo';
  const records = [
    cargoMessage(),
    cargoMessage({ line: 20 }),
    cargoMessage({ packageId: 'serde 1.0.0 (registry+https://example)', file: '/cargo/registry/serde/src/lib.rs' }),
    cargoMessage({ reason: 'build-finished' }),
    cargoMessage({ level: 'warning' }),
  ];
  const groups = parseInput(records.map(JSON.stringify).join('\n'), root);
  assert.equal(groups.length, 1);
  assert.equal(groups[0].count, 2);
  assert.equal(groups[0].category, 'mechanical_leaf_removal');
});

test('classifies only explicit patterns and sends ambiguous authority errors to primary review', () => {
  const root = '/repo';
  const records = [
    cargoMessage({ code: 'E0308', message: 'mismatched types: expected String, found usize' }),
    cargoMessage({ code: 'E0599', message: 'no method named `decode_legacy` found for struct Decoder' }),
    cargoMessage({ code: 'E0432', message: 'unresolved import `lease_authority::Claim`' }),
    cargoMessage({ code: 'E0425', message: 'missing generated client symbol handoff_url' }),
    cargoMessage({ code: 'E0277', message: 'the trait bound SessionOwner: Send is not satisfied' }),
  ];
  const groups = parseInput(records.map(JSON.stringify).join('\n'), root);
  assert.deepEqual(groups.map((item) => item.category).sort(), [
    'deleted_product_surface', 'migration_only_decoding', 'mechanical_leaf_removal', 'neutral_type_extraction', 'primary_architecture_decision',
  ].sort());
});

test('diff reports resolved, new, repeated, and regressed groups deterministically', () => {
  const root = '/repo';
  const first = parseInput([
    cargoMessage({ message: 'unresolved import `a`' }),
    cargoMessage({ file: 'cli/src/other.rs', message: 'mismatched types' , code: 'E0308' }),
  ].map(JSON.stringify).join('\n'), root);
  const current = parseInput([
    cargoMessage({ message: 'unresolved import `a`' }),
    cargoMessage({ message: 'unresolved import `a`' }),
    cargoMessage({ file: 'cli/src/new.rs', message: 'cannot find value `z` in this scope', code: 'E0425' }),
  ].map(JSON.stringify).join('\n'), root);
  const changes = comparePrior(current, { groups: first });
  assert.deepEqual(changes.map((item) => item.status).sort(), ['new', 'regressed', 'resolved'].sort());
});

test('stable output and raw-input preservation through the CLI', (t) => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'p218-'));
  t.after(() => fs.rmSync(temp, { recursive: true, force: true }));
  const root = path.join(temp, 'workspace');
  fs.mkdirSync(path.join(root, 'cli/src'), { recursive: true });
  const input = path.join(temp, 'cargo.jsonl');
  const original = `${JSON.stringify(cargoMessage({ file: path.join(root, 'cli/src/lib.rs') }))}\n`;
  fs.writeFileSync(input, original);
  const before = crypto.createHash('sha256').update(fs.readFileSync(input)).digest('hex');
  const outputs = [0, 1].map((index) => ({
    manifest: path.join(temp, `manifest-${index}.json`),
    worklist: path.join(temp, `worklist-${index}.md`),
  }));
  for (const output of outputs) {
    const result = spawnSync(process.execPath, [
      new URL('./p218-compiler-diagnostics.mjs', import.meta.url).pathname,
      '--input', input, '--workspace-root', root, '--manifest', output.manifest, '--worklist', output.worklist,
    ], { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
  }
  const after = crypto.createHash('sha256').update(fs.readFileSync(input)).digest('hex');
  assert.equal(after, before);
  assert.equal(fs.readFileSync(outputs[0].manifest, 'utf8'), fs.readFileSync(outputs[1].manifest, 'utf8'));
  assert.equal(fs.readFileSync(outputs[0].worklist, 'utf8'), fs.readFileSync(outputs[1].worklist, 'utf8'));
});

test('rejects malformed arguments and malformed JSON input with typed codes', () => {
  assert.throws(() => parseArgs(['--input']), (error) => error.code === 'P218_ARGS');
  assert.throws(() => parseInput('{broken', '/repo'), (error) => error.code === 'P218_INPUT_JSON');
});
