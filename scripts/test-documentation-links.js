#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { checkDocumentationLinks, localTargets } from './check-documentation-links.js';

assert.deepEqual(
  localTargets('[local](guide.md) [anchor](#part) [web](https://example.com) <a href="nested/page.mdx">page</a>'),
  ['guide.md', 'nested/page.mdx'],
);

const root = mkdtempSync(join(tmpdir(), 'agent-browser-doc-links-'));
try {
  mkdirSync(join(root, 'docs', 'nested'), { recursive: true });
  writeFileSync(join(root, 'docs', 'guide.md'), '[good](nested/page.mdx)\n[bad](missing.md)\n');
  writeFileSync(join(root, 'docs', 'nested', 'page.mdx'), '# Page\n');
  assert.deepEqual(checkDocumentationLinks({ root, files: ['docs/guide.md'] }), [
    { file: 'docs/guide.md', target: 'missing.md', reason: 'missing' },
  ]);
  writeFileSync(join(root, 'docs', 'missing.md'), '# Present\n');
  assert.deepEqual(checkDocumentationLinks({ root, files: ['docs/guide.md'] }), []);
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Documentation link checks passed');
