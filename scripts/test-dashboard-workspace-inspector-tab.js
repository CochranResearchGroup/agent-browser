#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  buildSelectedWorkspaceContext,
} from '../packages/dashboard/src/lib/selected-workspace-context.ts';

const component = readFileSync('packages/dashboard/src/components/workspace-selection-panel.tsx', 'utf8');
const page = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const css = readFileSync('packages/dashboard/src/app/globals.css', 'utf8');
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'));
const fieldMap = readFileSync('docs/dev/notes/2026-06-01-workspace-tab-field-map.md', 'utf8');
const publishRuntime = readFileSync('scripts/publish-local-dashboard-runtime.js', 'utf8');
const runtimeSmoke = readFileSync('scripts/smoke-local-dashboard-runtime.js', 'utf8');

assert.match(
  component,
  /workspace-selection-header-strip[\s\S]*workspace-selection-indicators[\s\S]*workspace-selection-actions[\s\S]*<FactGrid rows=\{workspaceFactRows\(context\)\}/,
  'Workspace inspector must render a dense header strip, runtime indicators, action row, and fact grid before evidence.',
);

assert.match(
  component,
  /buildStatusFacts[\s\S]*label: "PID"[\s\S]*label: "RSS"[\s\S]*label: "CPU"[\s\S]*label: "CDP"[\s\S]*label: "Stream"/,
  'Workspace inspector must expose small PID, memory, CPU, CDP, and stream indicators.',
);

assert.match(
  component,
  /workspaceFactRows[\s\S]*\["Workspace"[\s\S]*\["Browser"[\s\S]*\["Session"[\s\S]*\["Profile"[\s\S]*\["Owner"[\s\S]*\["Attention"[\s\S]*\["Health"[\s\S]*\["Host"[\s\S]*\["Build"[\s\S]*\["Running"[\s\S]*\["PID"[\s\S]*\["Memory"[\s\S]*\["CPU"[\s\S]*\["Uptime", "not reported"[\s\S]*\["CDP"[\s\S]*\["Stream"[\s\S]*\["Last frame"[\s\S]*\["Provider"[\s\S]*\["Route"[\s\S]*\["Input"[\s\S]*\["View"[\s\S]*\["Control"[\s\S]*\["Title"[\s\S]*\["URL"[\s\S]*\["Target"[\s\S]*\["Lifecycle"[\s\S]*\["Jobs"[\s\S]*\["Incidents"[\s\S]*\["Diagnostics"/,
  'Workspace inspector fact grid must cover identity, attention, runtime, stream, page, jobs, incidents, and diagnostics.',
);

assert.match(
  component,
  /FRONTEND_RUNNABLE_ACTIONS[\s\S]*"copy-link"[\s\S]*"external-open"[\s\S]*"view"[\s\S]*"control"[\s\S]*unsupportedReason[\s\S]*data-action-reason=\{reason\}/,
  'Workspace inspector must distinguish runnable actions from advertised-but-unwired actions and expose reasons.',
);

assert.match(
  component,
  /copyTextToClipboard[\s\S]*navigator\.clipboard\?\.writeText[\s\S]*fallbackCopyTextToClipboard[\s\S]*document\.execCommand\("copy"\)/,
  'Workspace inspector copy actions must fall back when the Clipboard API is unavailable or denied.',
);

assert.match(
  component,
  /actionId === "copy-link"[\s\S]*copyTextToClipboard\(window\.location\.href\)[\s\S]*Clipboard write failed\. Use the address bar to copy this workspace link\./,
  'Workspace inspector copy-link action must surface clipboard failures instead of dropping rejected writes.',
);

assert.match(
  component,
  /<details className="workspace-selection-evidence">[\s\S]*<summary>Evidence<\/summary>/,
  'Workspace inspector raw evidence must remain collapsed by default.',
);

assert.match(
  css,
  /\.workspace-selection-header-strip[\s\S]*grid-template-columns: minmax\(7rem, 1\.25fr\) minmax\(5\.5rem, 0\.55fr\) minmax\(12rem, 1fr\) auto/,
  'Workspace inspector header must use a compact strip, not a bulky vertical panel.',
);

assert.match(
  css,
  /\.dashboard-right-tabs[\s\S]*overflow-x: auto[\s\S]*\.dashboard-right-tabs \[data-slot="tabs-trigger"\][\s\S]*flex: 0 0 auto/,
  'Right-pane tabs must stay visible and horizontally scrollable instead of clipping the active Workspace tab.',
);

assert.match(
  css,
  /\.dashboard-right-tabs[\s\S]*scrollbar-width: none[\s\S]*\.dashboard-right-tabs::-webkit-scrollbar[\s\S]*display: none/,
  'Right-pane tab overflow must not show a bulky scrollbar in the compact inspector.',
);

assert.match(
  page,
  /<TabsList variant="line" className="dashboard-right-tabs h-7 w-full">/,
  'Dashboard right-pane tab list must use the compact scrollable tab styling.',
);

assert.match(
  css,
  /\.workspace-selection-details[\s\S]*grid-template-columns: repeat\(3, minmax\(0, 1fr\)\)[\s\S]*\.workspace-selection-details div[\s\S]*display: flex/,
  'Workspace facts must render as a dense multi-column key-value grid.',
);

assert.match(
  css,
  /\.workspace-selection-indicators[\s\S]*display: flex[\s\S]*flex-wrap: wrap[\s\S]*\.workspace-selection-indicator[\s\S]*min-width: 4\.8rem[\s\S]*\.workspace-selection-alert[\s\S]*max-height: 5\.4rem/,
  'Workspace runtime indicators and alerts must remain readable in the narrow right pane.',
);

assert.equal(
  packageJson.scripts['test:dashboard-workspace-inspector-tab'],
  'node --no-warnings --experimental-strip-types scripts/test-dashboard-workspace-inspector-tab.js',
  'package.json must expose the focused Workspace inspector test.',
);

for (const expected of [
  'True browser process uptime is not reported',
  'Last frame age is represented',
  'Repair, close, kill, launch, seed, resume, and add-tab',
]) {
  assert.ok(fieldMap.includes(expected), `Field map must document: ${expected}`);
}

assert.match(
  publishRuntime,
  /browserProfile: ''[\s\S]*arg === '--browser-profile'[\s\S]*smokeArgs\.push\('--browser-profile', options\.browserProfile\)[\s\S]*Use an isolated runtime profile for browser smoke/,
  'Runtime publish helper must pass an isolated browser profile through to the dashboard smoke.',
);

assert.match(
  runtimeSmoke,
  /skipChat: false[\s\S]*arg === '--skip-chat'[\s\S]*hasWorkspaceDetail[\s\S]*hasPidIndicator[\s\S]*hasMemoryIndicator[\s\S]*hasCpuIndicator[\s\S]*hasCdpFact[\s\S]*hasStreamFact[\s\S]*options\.workspaceSession && !options\.skipChat/,
  'Runtime smoke must support Workspace-only hosted validation without requiring Chat.',
);

const emptySelection = {
  workspaceId: null,
  browserId: null,
  sessionId: null,
  tabId: null,
  profileId: null,
  jobId: null,
};

const active = buildSelectedWorkspaceContext({
  selection: { ...emptySelection, browserId: 'browser-live' },
  refreshedAt: 1710000000000,
  serviceBrowsers: [
    {
      id: 'browser-live',
      profileId: 'default',
      host: 'local_headless',
      health: 'ready',
      browserBuild: 'chrome',
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
    { id: 'session-live', browserIds: ['browser-live'], tabIds: ['tab-live'], profileId: 'default', serviceName: 'svc', agentName: 'agent', taskName: 'task' },
  ],
  serviceTabs: [
    { id: 'tab-live', browserId: 'browser-live', sessionId: 'session-live', targetId: 'target-live', lifecycle: 'active', title: 'Live', url: 'https://live.example.test/' },
  ],
  jobs: [{ id: 'job-live', state: 'running', target: { browserId: 'browser-live' } }],
  incidents: [{ id: 'incident-live', browserId: 'browser-live', severity: 'warning', latestMessage: 'needs inspection' }],
});

assert.equal(active.runtime.pid, 95444);
assert.equal(active.runtime.cdpPort, 37227);
assert.equal(active.runtime.streamPort, 38395);
assert.equal(active.viewable, true);
assert.equal(active.controllable, true);
assert.equal(active.primaryTab?.targetId, 'target-live');
assert.ok(active.actions.some((action) => action.id === 'control' && action.enabled));

const postTermination = buildSelectedWorkspaceContext({
  selection: { ...emptySelection, browserId: 'browser-retained' },
  serviceBrowsers: [
    { id: 'browser-retained', health: 'closed', lastError: 'process exited' },
  ],
});

assert.equal(postTermination.state, 'missing');
assert.equal(postTermination.live, false);
assert.equal(postTermination.retained, false);
assert.match(postTermination.missingReason ?? '', /no longer reported by the service/);

const retained = buildSelectedWorkspaceContext({
  selection: { ...emptySelection, browserId: 'browser-retained-waiting' },
  serviceBrowsers: [
    { id: 'browser-retained-waiting', health: 'waiting', lastError: 'profile lease unavailable' },
  ],
});

assert.equal(retained.retained, true);
assert.equal(retained.live, false);
assert.ok(retained.actions.some((action) => action.id === 'view' && !action.enabled && action.reason));

const missing = buildSelectedWorkspaceContext({
  selection: { ...emptySelection, browserId: 'missing-browser' },
  serviceBrowsers: [],
});

assert.equal(missing.state, 'missing');
assert.ok(missing.missingReason);

const streamUnavailable = buildSelectedWorkspaceContext({
  selection: { ...emptySelection, browserId: 'browser-no-stream' },
  serviceBrowsers: [
    { id: 'browser-no-stream', health: 'ready', pid: 111, cdpEndpoint: 'http://127.0.0.1:9222/json' },
  ],
});

assert.equal(streamUnavailable.live, true);
assert.equal(streamUnavailable.viewable, false);
assert.ok(streamUnavailable.actions.some((action) => action.id === 'view' && !action.enabled && action.reason?.includes('No embeddable')));

console.log('dashboard workspace inspector tab tests passed');
