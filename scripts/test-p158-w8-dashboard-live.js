#!/usr/bin/env node

import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';

import {
  P158_DASHBOARD_DENSE_COUNTS,
  P158W8DashboardLiveError,
  auditP158DashboardLiveProjection,
  buildP158DashboardServiceState,
  captureP158DashboardLiveProjection,
  materializeP158DashboardServiceState,
} from './lib/p158-w8-dashboard-live.js';
import { generateDenseDashboardFixture } from './lib/p158-dashboard-oracle.js';
import { sha256 } from './lib/p158-campaign-controller.js';

const corpus = JSON.parse(readFileSync(
  new URL('../docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json', import.meta.url),
  'utf8',
));

function target(root) {
  return {
    runtimeLane: 'development',
    production: false,
    foreign: false,
    tenantDataPresent: false,
    ownership: 'p158_campaign',
    providerFree: false,
    serviceStopped: true,
    runId: basename(root),
    disposableRoot: root,
    pseudoHome: join(root, 'home'),
    statePath: join(root, 'home', '.agent-browser', 'service', 'state.json'),
  };
}

function values(record) {
  return Object.values(record);
}

for (const [density, counts] of Object.entries({
  empty: { profiles: 0, browsers: 0, tabs: 0, jobs: 0, events: 0 },
  sparse: { profiles: 2, browsers: 5, tabs: 20, jobs: 100, events: 100 },
  normal: { profiles: 10, browsers: 50, tabs: 200, jobs: 1000, events: 1000 },
  dense: P158_DASHBOARD_DENSE_COUNTS,
})) {
  const root = `/tmp/p158-${density}-determinism`;
  const first = buildP158DashboardServiceState({ target: target(root), density });
  const second = buildP158DashboardServiceState({ target: target(root), density });
  assert.deepEqual(first, second);
  assert.equal(first.state.schemaVersion, 'agent-browser.service-state.v2');
  assert.deepEqual(first.receipt.counts, counts);
  assert.equal(Object.keys(first.state.profiles).length, counts.profiles);
  assert.equal(Object.keys(first.state.browsers).length, counts.browsers);
  assert.equal(Object.keys(first.state.tabs).length, counts.tabs);
  assert.equal(Object.keys(first.state.jobs).length, counts.jobs);
  assert.equal(first.state.events.length, counts.events);
  assert(values(first.state.profiles).every((entry) => entry.userDataDir.startsWith(`${root}/profiles/`)));
  assert(values(first.state.browsers).every((entry) => entry.pid === undefined && entry.health === 'not_started'));
  assert(values(first.state.jobs).every((entry) => entry.state === 'succeeded'));
  assert.equal(first.receipt.productionStateTouched, false);
  assert.equal(first.receipt.repairAttempted, false);
  assert.equal(first.receipt.retryAttempted, false);
}

assert.throws(
  () => buildP158DashboardServiceState({
    target: { ...target('/tmp/p158-invalid-target'), production: true },
  }),
  (error) => error instanceof P158W8DashboardLiveError && error.code === 'development_target_unproven',
);

const dryRoot = mkdtempSync('/tmp/p158-dashboard-dry-');
try {
  const dry = await materializeP158DashboardServiceState({ target: target(dryRoot), density: 'sparse' });
  assert.equal(dry.written, false);
  assert.equal(existsSync(target(dryRoot).statePath), false);
  await assert.rejects(
    () => materializeP158DashboardServiceState({ target: target(dryRoot), density: 'sparse', apply: true }),
    (error) => error instanceof P158W8DashboardLiveError && error.code === 'service_state_parser_unproven',
  );
} finally {
  rmSync(dryRoot, { recursive: true, force: true });
}

const writeRoot = mkdtempSync('/tmp/p158-dashboard-write-');
try {
  const written = await materializeP158DashboardServiceState({
    target: target(writeRoot),
    density: 'sparse',
    apply: true,
    validateState: async ({ stateBytes, stateSha256 }) => ({
      accepted: JSON.parse(stateBytes).schemaVersion === 'agent-browser.service-state.v2',
      stateSha256,
      parserIdentitySha256: sha256('installed-candidate-parser-fixture'),
    }),
  });
  assert.equal(written.written, true);
  assert.equal(sha256(readFileSync(target(writeRoot).statePath, 'utf8')), written.receipt.stateSha256);
  await assert.rejects(
    () => materializeP158DashboardServiceState({
      target: target(writeRoot), density: 'sparse', apply: true,
      validateState: async ({ stateSha256 }) => ({
        accepted: true, stateSha256, parserIdentitySha256: sha256('installed-candidate-parser-fixture'),
      }),
    }),
    (error) => error?.code === 'EEXIST',
  );
} finally {
  rmSync(writeRoot, { recursive: true, force: true });
}

const liveRoot = mkdtempSync('/tmp/p158-dashboard-live-');
try {
  const built = buildP158DashboardServiceState({ target: target(liveRoot), density: 'dense' });
  const profileRows = values(built.state.profiles).map((entry, index) => ({
    rowId: `row-${entry.id}`,
    resourceId: entry.id,
    resourceType: 'profile',
    label: entry.name,
    state: 'ready',
    orderKey: index,
  }));
  const browserRows = values(built.state.browsers).map((entry, index) => ({
    rowId: `row-${entry.id}`,
    resourceId: entry.id,
    resourceType: 'browser',
    label: entry.id,
    state: entry.health,
    orderKey: profileRows.length + index,
  }));
  const capture = {
    collections: {
      profiles: { profiles: values(built.state.profiles) },
      browsers: { browsers: values(built.state.browsers) },
      tabs: { tabs: values(built.state.tabs) },
      jobs: { jobs: values(built.state.jobs) },
      events: { events: built.state.events },
    },
    railRows: [...profileRows, ...browserRows],
    actionButtons: [],
    warnings: [],
    locationPath: '/service',
    domNodeCount: 4000,
    performance: [{ durationMs: 900, domInteractiveMs: 500, loadEventEndMs: 850 }],
  };
  const screenshotPath = join(liveRoot, 'dashboard.png');
  const page = {
    evaluate: async () => structuredClone(capture),
    screenshot: async ({ path }) => writeFileSync(path, 'synthetic-dashboard-pixels'),
  };
  const externalProof = {
    offHost: true,
    outsideServiceNetworkNamespace: true,
    publicHttps: true,
    operatorVisibleState: 'ready',
    handoffUrlSha256: sha256('https://public.example.test/remote-view/opaque'),
  };
  const projection = await captureP158DashboardLiveProjection({
    page,
    materializationReceipt: built.receipt,
    externalProof,
    screenshotPath,
  });
  assert.equal(projection.counts.jobs, 10000);
  assert.equal(projection.capture.railRows.length, 600);
  assert.match(projection.authoritativeSnapshotSha256, /^[a-f0-9]{64}$/);
  assert.match(projection.renderedProjectionSha256, /^[a-f0-9]{64}$/);

  const fixture = generateDenseDashboardFixture({ idNamespace: 'p158-dense' });
  const rowById = new Map(capture.railRows.map((entry) => [entry.resourceId, entry]));
  fixture.truth.resources = fixture.truth.resources.map((resource) => {
    const observed = rowById.get(resource.resourceId);
    return observed ? { ...resource, label: observed.label, state: observed.state } : resource;
  });
  fixture.railRows = capture.railRows.map((row) => ({
    ...row,
    snapshotRevision: fixture.truth.snapshotRevision,
    badge: null,
    count: 0,
  }));
  fixture.selection = {
    ...fixture.selection,
    selectedResourceId: fixture.railRows[0].resourceId,
    inspectorResourceId: fixture.railRows[0].resourceId,
    deepLinkRequestedId: fixture.railRows[0].resourceId,
    deepLinkResolvedId: fixture.railRows[0].resourceId,
  };
  fixture.handoffUrls = ['https://public.example.test/remote-view/opaque'];
  fixture.uiChecks = structuredClone(corpus.baseline.uiChecks);
  fixture.timings = structuredClone(corpus.baseline.timings);
  fixture.resourceSamples = structuredClone(corpus.baseline.resourceSamples).map((sample) => ({
    ...sample,
    browserProcessCount: 500,
    profileLeaseCount: 100,
  }));
  const audit = auditP158DashboardLiveProjection({ projection, dashboardFixture: fixture });
  assert.equal(audit.passed, true);
  assert.deepEqual(audit.findingCodes, []);
  assert.throws(
    () => auditP158DashboardLiveProjection({
      projection,
      dashboardFixture: { ...fixture, truth: { ...fixture.truth, counts: { ...fixture.truth.counts, jobs: 9999 } } },
    }),
    (error) => error instanceof P158W8DashboardLiveError &&
      error.code === 'dashboard_fixture_binding_mismatch',
  );
  await assert.rejects(
    () => captureP158DashboardLiveProjection({
      page: { ...page, evaluate: async () => ({ ...capture, railRows: capture.railRows.slice(1) }) },
      materializationReceipt: built.receipt,
      externalProof,
      screenshotPath,
    }),
    (error) => error instanceof P158W8DashboardLiveError && error.code === 'rendered_projection_mismatch',
  );
} finally {
  rmSync(liveRoot, { recursive: true, force: true });
}

process.stdout.write('Plan 0158 W8 dashboard live foundation provider-free checks passed\n');
