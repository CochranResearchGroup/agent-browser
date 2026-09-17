#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function checkDocumentationLinks({ files, root = process.cwd() }) {
  const findings = [];
  for (const file of files.filter(isDocumentationFile).sort()) {
    const absolute = resolve(root, file);
    if (!existsSync(absolute)) continue;
    const source = readFileSync(absolute, 'utf8');
    for (const target of localTargets(source)) {
      const path = target.split('#', 1)[0].split('?', 1)[0];
      if (!path) continue;
      const resolved = resolve(dirname(absolute), decodeURIComponent(path));
      if (!existsSync(resolved)) {
        findings.push({ file, target, reason: 'missing' });
      } else if (statSync(resolved).isDirectory() && !existsSync(resolve(resolved, 'README.md'))) {
        findings.push({ file, target, reason: 'directory_without_readme' });
      }
    }
  }
  return findings;
}

export function localTargets(source) {
  const targets = [];
  const patterns = [/\[[^\]]*\]\(([^)\s]+)(?:\s+['"][^'"]*['"])?\)/g, /\bhref=['"]([^'"]+)['"]/g];
  for (const pattern of patterns) {
    for (const match of source.matchAll(pattern)) {
      const target = match[1];
      if (!/^(?:[a-z][a-z0-9+.-]*:|\/|#)/i.test(target)) targets.push(target);
    }
  }
  return [...new Set(targets)];
}

function isDocumentationFile(file) {
  return /(?:^|\/)README\.md$/i.test(file) || /\.(?:md|mdx)$/i.test(file);
}

function changedFiles(base, head) {
  const output = execFileSync('git', ['diff', '--name-only', `${base}...${head}`], { encoding: 'utf8' });
  return output.split('\n').filter(Boolean);
}

function main() {
  const args = process.argv.slice(2);
  const baseIndex = args.indexOf('--base');
  const headIndex = args.indexOf('--head');
  if (baseIndex < 0 || !args[baseIndex + 1]) throw new Error('--base is required');
  const head = headIndex >= 0 ? args[headIndex + 1] : 'HEAD';
  if (!head) throw new Error('--head requires a value');
  const files = changedFiles(args[baseIndex + 1], head);
  const findings = checkDocumentationLinks({ files });
  if (findings.length > 0) {
    for (const finding of findings) {
      process.stderr.write(`${finding.file}: ${finding.target} (${finding.reason})\n`);
    }
    process.exitCode = 1;
    return;
  }
  process.stdout.write(`Documentation links verified for ${files.filter(isDocumentationFile).length} changed files\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
