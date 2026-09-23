#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const CATEGORIES = [
  'mechanical_leaf_removal',
  'neutral_type_extraction',
  'migration_only_decoding',
  'deleted_product_surface',
  'primary_architecture_decision',
];

function usage() {
  return `Usage: node scripts/dev/p218-compiler-diagnostics.mjs --input <cargo.jsonl> --workspace-root <dir> --manifest <out.json> --worklist <out.md> [--prior <manifest.json>]\n\nReads Cargo JSON-lines compiler output without modifying it. Paths are resolved from the current directory.\n`;
}

function fail(code, message) {
  const error = new Error(message);
  error.code = code;
  throw error;
}

function parseArgs(argv) {
  const values = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--help' || arg === '-h') return { help: true };
    if (!['--input', '--workspace-root', '--manifest', '--worklist', '--prior'].includes(arg)) {
      fail('P218_ARGS', `unknown argument: ${arg}`);
    }
    if (values[arg]) fail('P218_ARGS', `duplicate argument: ${arg}`);
    const value = argv[++i];
    if (!value || value.startsWith('--')) fail('P218_ARGS', `missing value for ${arg}`);
    values[arg] = value;
  }
  for (const arg of ['--input', '--workspace-root', '--manifest', '--worklist']) {
    if (!values[arg]) fail('P218_ARGS', `required argument missing: ${arg}`);
  }
  return {
    input: path.resolve(values['--input']),
    workspaceRoot: path.resolve(values['--workspace-root']),
    manifest: path.resolve(values['--manifest']),
    worklist: path.resolve(values['--worklist']),
    prior: values['--prior'] ? path.resolve(values['--prior']) : null,
  };
}

function isWithin(root, candidate) {
  const relative = path.relative(root, candidate);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}

function normalizeFile(file, root) {
  if (typeof file !== 'string' || !file) return null;
  const absolute = path.resolve(root, file);
  if (!isWithin(root, absolute)) return null;
  return path.relative(root, absolute).split(path.sep).join('/');
}

function firstPartyDiagnostic(record, root) {
  if (record.reason !== 'compiler-message' || !record.message || record.message.level !== 'error') return null;
  const spans = Array.isArray(record.message.spans) ? record.message.spans : [];
  const primary = spans.find((span) => span.is_primary && normalizeFile(span.file_name, root));
  if (!primary) return null;
  return { primary, spans };
}

function normalizedMessage(message) {
  return String(message ?? '')
    .split('\n')[0]
    .replace(/`[^`]+`/g, '`<symbol>`')
    .replace(/\b\d+\b/g, '<n>')
    .replace(/\s+/g, ' ')
    .trim();
}

function discoverSymbol(message, spans, primary) {
  const spanText = spans.flatMap((span) => Array.isArray(span.text)
    ? span.text.map((item) => typeof item === 'string' ? item : item?.text ?? '')
    : [span.text ?? '']).join('\n');
  const text = `${message}\n${spanText}`;
  const patterns = [
    /(?:unresolved import|unresolved name|cannot find (?:value|type|function|module)|no method named|no field named|method named)\s+[`']?([A-Za-z_][\w:]*)/i,
    /(?:trait|type|struct|enum|function|method)\s+[`']([A-Za-z_][\w:]*)[`']/i,
    /`([A-Za-z_][\w:]*)`/,
  ];
  for (const pattern of patterns) {
    const match = text.match(pattern);
    if (match) return match[1];
  }
  return spanText.trim() || null;
}

function classify(code, message, symbol) {
  const text = `${message} ${symbol ?? ''}`.toLowerCase();
  if (/(?:migration|legacy|deserializ|decode|from_json|fromjson)/.test(text)
      && /(?:decode|deserializ|from_json|fromjson|migration|legacy)/.test(text)) {
    return 'migration_only_decoding';
  }
  if (['E0432', 'E0433'].includes(code) && /(?:unresolved import|unresolved module|could not find)/i.test(message)) {
    return 'mechanical_leaf_removal';
  }
  if (/(?:dashboard|generated.client|http|mcp|service.model|product.surface).*(?:removed|deleted|missing|unresolved|not found)|(?:removed|deleted|missing|unresolved|not found).*(?:dashboard|generated.client|http|mcp|service.model|product.surface)/.test(text)) {
    return 'deleted_product_surface';
  }
  if (['E0308', 'E0277', 'E0282', 'E0283', 'E0609', 'E0615'].includes(code)
      && !/(?:lease|owner|runtime.host|service.state|service model)/i.test(text)) {
    return 'neutral_type_extraction';
  }
  return 'primary_architecture_decision';
}

function parseInput(contents, root) {
  const groups = new Map();
  const lines = contents.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    if (!lines[index].trim()) continue;
    let record;
    try { record = JSON.parse(lines[index]); }
    catch { fail('P218_INPUT_JSON', `malformed JSON at input line ${index + 1}`); }
    const selected = firstPartyDiagnostic(record, root);
    if (!selected) continue;
    const { primary, spans } = selected;
    const file = normalizeFile(primary.file_name, root);
    const packageName = record.package_id?.match(/^([^ ]+)/)?.[1] ?? 'unknown-package';
    const symbol = discoverSymbol(record.message.message, spans, primary);
    const code = record.message.code?.code ?? null;
    const causalRoot = normalizedMessage(record.message.message);
    const category = classify(code, causalRoot, symbol);
    const keyParts = [packageName, file, symbol ?? '', code ?? '', causalRoot];
    const id = keyParts.map((part) => encodeURIComponent(part)).join('|');
    const prior = groups.get(id);
    const diagnostic = {
      package: packageName,
      file,
      symbol,
      code,
      causalRoot,
      category,
      line: Number.isInteger(primary.line_start) ? primary.line_start : null,
      message: normalizedMessage(record.message.message),
    };
    if (prior) {
      prior.count += 1;
    } else {
      groups.set(id, { id, ...diagnostic, count: 1 });
    }
  }
  return [...groups.values()].sort(compareGroup);
}

function compareGroup(a, b) {
  return CATEGORIES.indexOf(a.category) - CATEGORIES.indexOf(b.category)
    || a.package.localeCompare(b.package)
    || a.file.localeCompare(b.file) || String(a.symbol).localeCompare(String(b.symbol))
    || String(a.code).localeCompare(String(b.code)) || a.causalRoot.localeCompare(b.causalRoot);
}

function readPrior(file) {
  if (!file) return null;
  let parsed;
  try { parsed = JSON.parse(fs.readFileSync(file, 'utf8')); }
  catch (error) { fail('P218_PRIOR', `cannot read prior manifest: ${error.message}`); }
  if (!parsed || parsed.schema !== 'p218-compiler-diagnostics.v1' || !Array.isArray(parsed.groups)) {
    fail('P218_PRIOR', 'prior manifest has an unsupported schema');
  }
  return parsed;
}

function comparePrior(groups, prior) {
  const old = new Map((prior?.groups ?? []).map((group) => [group.id, group]));
  const current = new Map(groups.map((group) => [group.id, group]));
  const changes = [];
  for (const before of old.values()) {
    const after = current.get(before.id);
    if (!after) changes.push({ id: before.id, status: 'resolved', category: before.category, count: 0 });
    else changes.push({ id: after.id, status: after.count > before.count ? 'regressed' : 'repeated', category: after.category, count: after.count });
  }
  for (const after of groups) {
    if (!old.has(after.id)) changes.push({ id: after.id, status: 'new', category: after.category, count: after.count });
  }
  const order = { resolved: 0, new: 1, repeated: 2, regressed: 3 };
  return changes.sort((a, b) => order[a.status] - order[b.status] || a.category.localeCompare(b.category) || a.id.localeCompare(b.id));
}

function makeWorklist(groups, changes) {
  const lines = [
    'P218 compiler diagnostic worklist',
    `Total groups: ${groups.length}; total occurrences: ${groups.reduce((sum, group) => sum + group.count, 0)}`,
    'Category counts: ' + CATEGORIES.map((category) => `${category}=${groups.filter((group) => group.category === category).length}`).join(', '),
    '',
  ];
  if (groups.length === 0) lines.push('No first-party compiler errors found.');
  for (const [index, group] of groups.entries()) {
    lines.push(`${index + 1}. [${group.category}] ${group.package} — ${group.file}${group.line ? `:${group.line}` : ''}`);
    lines.push(`   ${group.count} occurrence(s); ${group.code ?? 'no code'}; ${group.symbol ?? 'symbol unknown'}`);
    lines.push(`   ${group.message}`);
  }
  if (changes) {
    lines.push('', 'Prior wave comparison:');
    if (!changes.length) lines.push('No groups in either wave.');
    for (const status of ['resolved', 'new', 'repeated', 'regressed']) {
      const count = changes.filter((change) => change.status === status).length;
      lines.push(`- ${status}: ${count}`);
    }
  }
  lines.push('', 'Next compile gate: resolve one coherent accepted batch, then run one bounded Cargo JSON wave through the repository WSL Cargo wrapper.');
  return `${lines.join('\n')}\n`;
}

function mkdirFor(file) { fs.mkdirSync(path.dirname(file), { recursive: true }); }

function main(argv) {
  const options = parseArgs(argv);
  if (options.help) { process.stdout.write(usage()); return; }
  if (options.input === options.manifest || options.input === options.worklist
      || options.manifest === options.worklist || options.prior === options.manifest
      || options.prior === options.worklist) {
    fail('P218_ARGS', 'input, prior, manifest, and worklist paths must not overwrite one another');
  }
  let contents;
  try { contents = fs.readFileSync(options.input, 'utf8'); }
  catch (error) { fail('P218_INPUT_READ', `cannot read input: ${error.message}`); }
  const groups = parseInput(contents, options.workspaceRoot);
  const prior = readPrior(options.prior);
  const changes = prior ? comparePrior(groups, prior) : null;
  const manifest = {
    schema: 'p218-compiler-diagnostics.v1',
    groups,
    comparison: changes ? { counts: Object.fromEntries(['resolved', 'new', 'repeated', 'regressed'].map((status) => [status, changes.filter((item) => item.status === status).length])), groups: changes } : null,
  };
  mkdirFor(options.manifest);
  mkdirFor(options.worklist);
  fs.writeFileSync(options.manifest, `${JSON.stringify(manifest, null, 2)}\n`);
  fs.writeFileSync(options.worklist, makeWorklist(groups, changes));
  process.stdout.write(`P218_OK groups=${groups.length} manifest=${options.manifest} worklist=${options.worklist}\n`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(process.argv.slice(2)); }
  catch (error) {
    process.stderr.write(`${error.code ?? 'P218_ERROR'}: ${error.message}\n`);
    process.exitCode = 2;
  }
}

export { classify, comparePrior, parseArgs, parseInput };
