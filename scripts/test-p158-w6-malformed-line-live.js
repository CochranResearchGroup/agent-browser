#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { appendFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  executeP158W6MalformedLineLive,
  p158W6ProtectedJournalPaths,
} from './run-p158-w6-malformed-line-live.js';

const root = await mkdtemp(join(tmpdir(), 'p158-malformed-test-'));
try {
  const generationId = 'development-generation-test';
  const generation = join(root, 'generations', generationId);
  const executable = join(generation, 'bin', 'agent-browser');
  const bytes = Buffer.from('provider-free exact candidate');
  await mkdir(join(generation, 'bin'), { recursive: true });
  await writeFile(executable, bytes, { mode: 0o700 });
  const executableSha256 = createHash('sha256').update(bytes).digest('hex');
  const candidate = { candidateSha256: '11'.repeat(32), executableSha256, installedGenerationId: generationId };
  const defaultProtectedPaths = p158W6ProtectedJournalPaths({
    pseudoHome: join(root, 'protected-development-home'),
  });
  assert.equal(defaultProtectedPaths.length, 2);
  assert(defaultProtectedPaths.every((path) => typeof path === 'string'));
  let isolatedEnv;
  const records = [];
  const launchDashboard = async (_command, environment) => {
    isolatedEnv = environment;
    return { child: { exitCode: null }, stderr: () => '', stop: async () => {} };
  };
  const response = (url, status, body, headers = {}) => ({
    ok: status >= 200 && status < 300, status, url, redirected: false,
    headers: { get: (name) => headers[name.toLowerCase()] ?? null },
    json: async () => structuredClone(body),
  });
  const fetch = async (url, init = {}) => {
    const path = new URL(url).pathname;
    if (path === '/api/runtime/manifest') {
      await mkdir(isolatedEnv.AGENT_BROWSER_DASHBOARD_AUTH_DIR, { recursive: true });
      await writeFile(join(isolatedEnv.AGENT_BROWSER_DASHBOARD_AUTH_DIR, 'dashboard-auth.env'),
        'AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME="admin"\nAGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD="private"\n', { mode: 0o600 });
      return response(url, 200, { runtimeEnvironment: 'development', executable: { sha256: executableSha256 } });
    }
    if (path === '/api/dashboard-auth/login') {
      return response(url, 200, { authenticated: true }, { 'set-cookie': 'session=opaque; HttpOnly' });
    }
    if (path === '/api/service/failure-observation') {
      const input = JSON.parse(init.body);
      const record = {
        schemaVersion: 'agent-browser.service-failure-record.v1', occurrenceId: `occurrence-${records.length}`,
        occurredAt: '2026-09-04T23:00:00.000Z', runtimeEnvironment: 'development',
        category: input.category, source: 'dashboard_client', stage: input.stage, code: input.code,
        summary: input.summary, action: input.action, references: {},
      };
      records.push(record);
      const journal = join(isolatedEnv.HOME, '.agent-browser/service/failure-journal.jsonl');
      await mkdir(join(isolatedEnv.HOME, '.agent-browser/service'), { recursive: true });
      await appendFile(journal, `${JSON.stringify(record)}\n`);
      return response(url, 202, { success: true, data: { occurrenceId: record.occurrenceId } });
    }
    if (path === '/api/service/failures') {
      const journal = await readFile(join(isolatedEnv.HOME, '.agent-browser/service/failure-journal.jsonl'), 'utf8');
      let malformedLineCount = 0;
      const parsed = journal.trim().split(/\n/u).flatMap((line) => {
        try { return [JSON.parse(line)]; } catch { malformedLineCount += 1; return []; }
      });
      return response(url, 200, { success: true, data: {
        schemaVersion: 'agent-browser.service-failure-journal-readback.v1', records: parsed,
        malformedLineCount, writeFailureCount: 0,
      } });
    }
    throw new Error(`unexpected ${url}`);
  };
  const outputPath = join(root, 'receipt.json');
  const artifact = await executeP158W6MalformedLineLive({
    candidate, outputPath, isolationParent: root,
    descriptor: { current: generation, pseudoHome: join(root, 'protected-development-home') },
    protectedJournals: [join(root, 'protected-production-journal'), join(root, 'protected-development-journal')],
    fetch, launchDashboard, clock: () => '2026-09-04T23:00:00.000Z', env: { PATH: '/usr/bin' },
  });
  assert.equal(artifact.receipt.malformedLineCount, 1);
  assert.equal(artifact.receipt.validRecordBeforeMalformed, true);
  assert.equal(artifact.receipt.validRecordAfterMalformed, true);
  assert.equal(artifact.receipt.executableSha256, executableSha256);
  assert.equal(artifact.liveJournalMutated, false);
  assert.equal(artifact.candidateWriterUsed, true);
  assert.equal(artifact.candidateReadbackUsed, true);
  assert.deepEqual(JSON.parse(await readFile(outputPath, 'utf8')), artifact);
  assert.doesNotMatch(JSON.stringify(artifact), /private|session=opaque/u);

  await assert.rejects(() => executeP158W6MalformedLineLive({
    candidate: { ...candidate, executableSha256: '22'.repeat(32) },
    outputPath: join(root, 'must-not-exist.json'), isolationParent: root,
    descriptor: { current: generation, pseudoHome: join(root, 'protected-development-home') },
    protectedJournals: [join(root, 'protected-production-journal'), join(root, 'protected-development-journal')],
    fetch, launchDashboard, env: { PATH: '/usr/bin' },
  }), (error) => error.code === 'malformed_line_candidate_mismatch');
} finally {
  await rm(root, { recursive: true, force: true });
}

process.stdout.write('P158 W6 malformed-line live producer provider-free test passed\n');
