import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

const path = new URL(
  '../../docs/dev/contracts/p220-p219-file-disposition.v1.json',
  import.meta.url,
);
const ledger = JSON.parse(await readFile(path, 'utf8'));
const allowed = new Set(['retain', 'adapt', 'retire', 'evidence-only']);
const dispositions = Object.entries(ledger.dispositions ?? {});

if (ledger.schemaVersion !== 1) throw new Error('unsupported ledger schema');
if (dispositions.some(([name]) => !allowed.has(name))) {
  throw new Error('ledger contains an unknown disposition');
}
if (dispositions.length !== allowed.size) {
  throw new Error('ledger must contain every disposition exactly once');
}

const files = dispositions.flatMap(([, entries]) => entries);
if (files.length !== ledger.sourceFileCount) {
  throw new Error(`expected ${ledger.sourceFileCount} files, found ${files.length}`);
}
if (new Set(files).size !== files.length) {
  throw new Error('every source file must have exactly one disposition');
}
if (files.some((file) => typeof file !== 'string' || !file || file.startsWith('/'))) {
  throw new Error('ledger file paths must be nonempty repository-relative strings');
}

const sortedFileList = `${files.toSorted().join('\n')}\n`;
const digest = createHash('sha256').update(sortedFileList).digest('hex');
if (digest !== ledger.sortedFileListSha256) {
  throw new Error(`sorted file-list digest drifted: ${digest}`);
}

console.log(
  `P220 P219 custody ledger passed: ${files.length} files, digest ${digest}`,
);
