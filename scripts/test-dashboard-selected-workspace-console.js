#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  buildSelectedWorkspaceConsoleEvidence,
  consoleEvidenceForChat,
  redactedConsoleEvidenceBundle,
} from '../packages/dashboard/src/lib/selected-workspace-console.ts';
import { buildSelectedWorkspaceContext } from '../packages/dashboard/src/lib/selected-workspace-context.ts';

const consolePanel = readFileSync('packages/dashboard/src/components/console-panel.tsx', 'utf8');
const chatPanel = readFileSync('packages/dashboard/src/components/chat-panel.tsx', 'utf8');
const page = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const streamStore = readFileSync('packages/dashboard/src/store/stream.ts', 'utf8');
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'));

assert.match(
  consolePanel,
  /buildSelectedWorkspaceConsoleEvidence[\s\S]*data-console-evidence-attribution[\s\S]*console-inspector-header[\s\S]*sourceReadiness[\s\S]*Send Console evidence to Chat/s,
  'Console panel must render selected-workspace evidence, source readiness, attribution, and Chat handoff.',
);
assert.doesNotMatch(
  consolePanel,
  /Evaluate JavaScript|execCommand|sessionArgs|activeSessionNameAtom|clearConsoleLogsAtom/,
  'Selected-workspace Console lane must not expose eval or browser-side clear controls.',
);
assert.match(
  chatPanel,
  /consoleLogsAtom[\s\S]*buildSelectedWorkspaceConsoleEvidence[\s\S]*consoleEvidence[\s\S]*buildSelectedWorkspaceChatPacket/s,
  'Chat must build selected-workspace packets with Console evidence.',
);
assert.match(
  page,
  /setSidePanelTab\("chat"\)[\s\S]*agent-browser-dashboard-console-send-to-chat/s,
  'Dashboard must switch the right pane to Chat when Console sends evidence.',
);
assert.match(
  streamStore,
  /case "console"[\s\S]*streamPort: port[\s\S]*case "page_error"[\s\S]*streamPort: port/s,
  'Stream sync must stamp Console and page-error messages with their source stream port.',
);
assert.equal(
  packageJson.scripts['test:dashboard-selected-workspace-console'],
  'node --no-warnings --experimental-strip-types scripts/test-dashboard-selected-workspace-console.js',
  'package.json must expose the focused Console evidence test.',
);

const emptySelection = {
  workspaceId: null,
  browserId: null,
  sessionId: null,
  tabId: null,
  profileId: null,
  jobId: null,
};

const context = buildSelectedWorkspaceContext({
  selection: { ...emptySelection, browserId: 'browser-live' },
  refreshedAt: 1710000000000,
  serviceBrowsers: [
    {
      id: 'browser-live',
      profileId: 'default',
      health: 'ready',
      pid: 95444,
      cdpEndpoint: 'http://127.0.0.1:37227/json',
      processStats: { pid: 95444, running: true, rssBytes: 104857600, cpuSeconds: 12.25 },
      viewStreams: [
        {
          provider: 'cdp_screencast',
          url: 'http://127.0.0.1:38395/',
          routeId: 'route-live',
          providerMode: 'single_controller',
          controlInput: 'cdp_input',
          readOnly: false,
        },
      ],
      activeSessionIds: ['session-live'],
    },
  ],
  serviceSessions: [
    { id: 'session-live', browserIds: ['browser-live'], tabIds: ['tab-live'], profileId: 'default', serviceName: 'svc' },
  ],
  serviceTabs: [
    { id: 'tab-live', browserId: 'browser-live', sessionId: 'session-live', targetId: 'target-live', lifecycle: 'active', title: 'Live', url: 'https://live.example.test/' },
  ],
});

const entries = [
  { type: 'console', level: 'error', text: 'failed with token=abc123', timestamp: 1710000000500, streamPort: 38395 },
  { type: 'console', level: 'warn', text: 'Content Security Policy warning', timestamp: 1710000000600, streamPort: 38395 },
  { type: 'page_error', text: 'Uncaught Error: boom', line: 10, column: 4, timestamp: 1710000000700, streamPort: 38395 },
  { type: 'console', level: 'error', text: 'wrong browser', timestamp: 1710000000800, streamPort: 48000 },
  { type: 'console', level: 'log', text: 'global fallback', timestamp: 1710000000900 },
];

const scoped = buildSelectedWorkspaceConsoleEvidence(context, entries);
assert.equal(scoped.workspaceId, 'browser:browser-live');
assert.equal(scoped.counts.scoped, 3);
assert.equal(scoped.counts.errors, 2);
assert.equal(scoped.counts.warnings, 1);
assert.equal(scoped.counts.pageErrors, 1);
assert.equal(scoped.counts.security, 1);
assert.equal(scoped.counts.unscoped, 0);
assert.equal(scoped.rows.every((row) => row.relatedIds.browserId === 'browser-live'), true);
assert.equal(scoped.rows.every((row) => row.relatedIds.streamPort === 38395), true);
assert.doesNotMatch(JSON.stringify(scoped), /abc123/);

const withFallback = buildSelectedWorkspaceConsoleEvidence(context, entries, { includeUnscoped: true });
assert.equal(withFallback.counts.scoped, 3);
assert.equal(withFallback.counts.unscoped, 2);
assert.equal(withFallback.rows.some((row) => row.text === 'wrong browser' && row.attribution === 'missing'), true);
assert.equal(withFallback.rows.some((row) => row.text === 'global fallback' && row.attribution === 'unscoped'), true);

const chatEvidence = consoleEvidenceForChat(scoped);
assert.equal(chatEvidence.available, true);
assert.equal(chatEvidence.facts.counts.scoped, 3);
assert.equal(chatEvidence.facts.rows.length, 3);
assert.doesNotMatch(JSON.stringify(chatEvidence), /abc123/);

const missingContextEvidence = buildSelectedWorkspaceConsoleEvidence(null, entries);
assert.equal(missingContextEvidence.counts.scoped, 0);
assert.match(missingContextEvidence.unavailableReason, /Select a workspace/);
assert.equal(consoleEvidenceForChat(missingContextEvidence).available, false);

const bundle = redactedConsoleEvidenceBundle(scoped);
assert.match(bundle, /browser:browser-live/);
assert.doesNotMatch(bundle, /abc123/);

console.log('dashboard selected workspace console tests passed');
