import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, normalize } from 'node:path';

import { auditDashboardFixture } from './p158-dashboard-oracle.js';
import { canonicalJson, sha256 } from './p158-campaign-controller.js';

const DENSE_COUNTS = Object.freeze({
  profiles: 100,
  browsers: 500,
  tabs: 2000,
  jobs: 10000,
  events: 10000,
});

const DENSITY_COUNTS = Object.freeze({
  empty: Object.freeze({ profiles: 0, browsers: 0, tabs: 0, jobs: 0, events: 0 }),
  sparse: Object.freeze({ profiles: 2, browsers: 5, tabs: 20, jobs: 100, events: 100 }),
  normal: Object.freeze({ profiles: 10, browsers: 50, tabs: 200, jobs: 1000, events: 1000 }),
  dense: DENSE_COUNTS,
});

export class P158W8DashboardLiveError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'P158W8DashboardLiveError';
    this.code = code;
  }
}

function fail(code, message) {
  throw new P158W8DashboardLiveError(code, message);
}

function exactDevelopmentTarget(target) {
  if (target?.runtimeLane !== 'development' || target.production === true ||
      target.foreign === true || target.tenantDataPresent === true ||
      target.ownership !== 'p158_campaign' || target.providerFree !== false ||
      target.serviceStopped !== true || typeof target.runId !== 'string' ||
      !target.runId.startsWith('p158-') || typeof target.disposableRoot !== 'string' ||
      !target.disposableRoot.startsWith('/tmp/') ||
      !normalize(target.disposableRoot).includes(target.runId)) {
    fail('development_target_unproven', 'D dashboard materialization requires a stopped, disposable P158 development target');
  }
  const expectedPseudoHome = join(normalize(target.disposableRoot), 'home');
  if (normalize(target.pseudoHome ?? '') !== expectedPseudoHome) {
    fail('development_target_unproven', 'Development pseudo-home is not inside the disposable campaign root');
  }
  const expectedStatePath = join(expectedPseudoHome, '.agent-browser', 'service', 'state.json');
  if (normalize(target.statePath ?? '') !== expectedStatePath) {
    fail('development_target_unproven', 'Service State path is not the exact disposable campaign path');
  }
  return expectedStatePath;
}

function ids(prefix, count) {
  return Array.from({ length: count }, (_, index) =>
    `${prefix}-${String(index + 1).padStart(5, '0')}`);
}

function inventoryIdentity(state) {
  return {
    profiles: Object.keys(state.profiles).sort(),
    browsers: Object.keys(state.browsers).sort(),
    tabs: Object.keys(state.tabs).sort(),
    jobs: Object.keys(state.jobs).sort(),
    events: state.events.map((entry) => entry.id).sort(),
  };
}

export function buildP158DashboardServiceState({ target, density = 'dense' }) {
  exactDevelopmentTarget(target);
  const counts = DENSITY_COUNTS[density];
  if (!counts) fail('density_invalid', `Unsupported D01 density ${density}`);
  const profileIds = ids(`p158-${density}-profile`, counts.profiles);
  const browserIds = ids(`p158-${density}-browser`, counts.browsers);
  const tabIds = ids(`p158-${density}-tab`, counts.tabs);
  const jobIds = ids(`p158-${density}-job`, counts.jobs);
  const eventIds = ids(`p158-${density}-event`, counts.events);
  const capturedAt = '2026-09-03T00:00:00.000Z';
  const state = {
    schemaVersion: 'agent-browser.service-state.v2',
    stateRevision: 158,
    profiles: Object.fromEntries(profileIds.map((id, index) => [id, {
      id,
      name: `P158 ${density} profile ${String(index + 1).padStart(3, '0')}`,
      description: 'Synthetic Plan 0158 dashboard load fixture.',
      profileOrigin: 'agent_browser_owned',
      profileClass: 'durable_named',
      userDataDir: join(target.disposableRoot, 'profiles', id),
      persistent: true,
      tags: ['p158', 'synthetic', density],
    }])),
    browsers: Object.fromEntries(browserIds.map((id, index) => [id, {
      id,
      profileId: profileIds[index % Math.max(1, profileIds.length)] ?? null,
      host: 'local_headed',
      health: 'not_started',
      viewStreams: [],
      activeSessionIds: [],
      tabHandles: [],
      recordProvenance: {
        source: 'persisted_state',
        authoritySource: 'legacy_unproven',
        lifecycleClassification: 'inert_legacy',
        recommendedAction: 'observe_only',
        recordRevision: 1,
        evidenceDigest: sha256(`${target.runId}:${density}:${id}`),
      },
    }])),
    tabs: Object.fromEntries(tabIds.map((id, index) => [id, {
      id,
      browserId: browserIds[index % Math.max(1, browserIds.length)] ?? '',
      targetId: `target-${id}`,
      lifecycle: 'closed',
      url: 'https://fixture.invalid/redacted',
      title: `P158 synthetic tab ${String(index + 1).padStart(5, '0')}`,
    }])),
    jobs: Object.fromEntries(jobIds.map((id, index) => [id, {
      id,
      action: 'status',
      serviceName: 'p158-dashboard-fixture',
      agentName: 'p158-synthetic-agent',
      taskName: `${density}-job-${String(index + 1).padStart(5, '0')}`,
      target: 'service',
      owner: 'system',
      state: 'succeeded',
      priority: 'normal',
      submittedAt: capturedAt,
      startedAt: capturedAt,
      completedAt: capturedAt,
      result: { synthetic: true },
    }])),
    events: eventIds.map((id, index) => ({
      id,
      timestamp: capturedAt,
      kind: 'reconciliation',
      message: `P158 synthetic ${density} event ${String(index + 1).padStart(5, '0')}`,
      browserId: browserIds[index % Math.max(1, browserIds.length)] ?? null,
      profileId: profileIds[index % Math.max(1, profileIds.length)] ?? null,
      serviceName: 'p158-dashboard-fixture',
      agentName: 'p158-synthetic-agent',
      taskName: `${density}-event`,
      details: { synthetic: true },
    })),
  };
  const identity = inventoryIdentity(state);
  const dashboardFixtureDescriptor = {
    generatorVersion: 'p158-dashboard-dense.v1',
    seed: 158,
    ...counts,
    idNamespace: `p158-${density}`,
    labelCardinality: 17,
  };
  const receiptBody = {
    schemaVersion: 'agent-browser.p158-dashboard-state-materialization.v1',
    planId: 'P158',
    runId: target.runId,
    density,
    statePath: target.statePath,
    counts: { ...counts },
    stateSha256: sha256(canonicalJson(state)),
    inventoryIdentitySha256: sha256(identity),
    dashboardFixtureDescriptor,
    dashboardFixtureDescriptorSha256: sha256(dashboardFixtureDescriptor),
    disposableRuntimeRootSha256: sha256(target.disposableRoot),
    syntheticOnly: true,
    developmentOnly: true,
    productionStateTouched: false,
    repairAttempted: false,
    retryAttempted: false,
  };
  return { state, receipt: { ...receiptBody, receiptSha256: sha256(receiptBody) } };
}

export async function materializeP158DashboardServiceState({
  target,
  density = 'dense',
  apply = false,
  validateState = null,
}) {
  const built = buildP158DashboardServiceState({ target, density });
  if (!apply) return { ...built, written: false };
  if (typeof validateState !== 'function') {
    fail('service_state_parser_unproven',
      'Writing a dense fixture requires an installed-candidate Service State parser receipt');
  }
  const stateBytes = canonicalJson(built.state);
  const parserReceipt = await validateState({
    statePath: target.statePath,
    stateBytes,
    stateSha256: built.receipt.stateSha256,
  });
  if (parserReceipt?.accepted !== true ||
      parserReceipt.stateSha256 !== built.receipt.stateSha256 ||
      !/^[a-f0-9]{64}$/.test(parserReceipt.parserIdentitySha256 ?? '')) {
    fail('service_state_parser_unproven',
      'Installed-candidate parser did not accept the exact Service State bytes');
  }
  await mkdir(dirname(target.statePath), { recursive: true, mode: 0o700 });
  await writeFile(target.statePath, stateBytes, { flag: 'wx', mode: 0o600 });
  return { ...built, parserReceipt: structuredClone(parserReceipt), written: true };
}

function collectionIdentity(snapshot) {
  const collection = (name) => (snapshot[name]?.[name] ?? snapshot[name] ?? []).map((entry) => entry.id).sort();
  return {
    profiles: collection('profiles'),
    browsers: collection('browsers'),
    tabs: collection('tabs'),
    jobs: collection('jobs'),
    events: collection('events'),
  };
}

export async function captureP158DashboardLiveProjection({
  page,
  materializationReceipt,
  externalProof,
  screenshotPath,
}) {
  if (!page || typeof page.evaluate !== 'function' || typeof page.screenshot !== 'function') {
    fail('playwright_page_missing', 'Dashboard capture requires a Playwright page');
  }
  if (externalProof?.offHost !== true || externalProof?.outsideServiceNetworkNamespace !== true ||
      externalProof?.publicHttps !== true || externalProof?.operatorVisibleState !== 'ready' ||
      !/^[a-f0-9]{64}$/.test(externalProof?.handoffUrlSha256 ?? '')) {
    fail('external_ingress_unproven', 'Dashboard capture is not bound to public off-host ingress');
  }
  const capture = await page.evaluate(async () => {
    const endpoints = [
      'profiles?limit=100',
      'browsers?limit=500',
      'tabs?limit=2000',
      'jobs?limit=10000',
      'events?limit=10000',
    ];
    const responses = await Promise.all(endpoints.map(async (endpoint) => {
      const response = await fetch(`/api/service/${endpoint}`, { credentials: 'same-origin', cache: 'no-store' });
      const body = await response.json();
      if (!response.ok || body.success !== true) throw new Error(`dashboard collection failed: ${endpoint}`);
      return [endpoint.split('?')[0], body.data];
    }));
    const railRows = Array.from(document.querySelectorAll(
      'button[aria-label^="Inspect browser "],button[aria-label^="Inspect profile allocation "]',
    )).map((button, index) => {
      const ariaLabel = button.getAttribute('aria-label');
      const resourceType = ariaLabel.startsWith('Inspect browser ') ? 'browser' : 'profile';
      const prefix = resourceType === 'browser' ? 'Inspect browser ' : 'Inspect profile allocation ';
      const resourceId = ariaLabel.slice(prefix.length);
      return {
        rowId: button.getAttribute('data-row-id') || `row-${resourceId}`,
        resourceId,
        resourceType,
        label: (button.textContent || '').trim(),
        state: button.getAttribute('data-state') || 'not_started',
        orderKey: index,
      };
    });
    const actionButtons = Array.from(document.querySelectorAll('button[data-action-id],button[aria-label]'))
      .map((button) => ({
        actionId: button.getAttribute('data-action-id') || button.getAttribute('aria-label'),
        targetResourceId: button.getAttribute('data-resource-id'),
        disabled: button.disabled || button.getAttribute('aria-disabled') === 'true',
      }));
    const warnings = Array.from(document.querySelectorAll('[role="alert"],[data-health-axis]'))
      .map((element) => ({ axis: element.getAttribute('data-health-axis'), text: (element.textContent || '').trim() }));
    return {
      collections: Object.fromEntries(responses),
      railRows,
      actionButtons,
      warnings,
      locationPath: location.pathname,
      domNodeCount: document.querySelectorAll('*').length,
      performance: performance.getEntriesByType('navigation').map((entry) => ({
        durationMs: entry.duration,
        domInteractiveMs: entry.domInteractive,
        loadEventEndMs: entry.loadEventEnd,
      })),
    };
  });
  const observedIdentity = collectionIdentity(capture.collections);
  if (sha256(observedIdentity) !== materializationReceipt.inventoryIdentitySha256) {
    fail('authoritative_snapshot_mismatch', 'Live Service collections do not match the frozen materialization');
  }
  const renderedRailIds = capture.railRows.map((entry) => entry.resourceId).sort();
  const expectedRailIds = [...observedIdentity.profiles, ...observedIdentity.browsers].sort();
  if (sha256(renderedRailIds) !== sha256(expectedRailIds)) {
    fail('rendered_projection_mismatch',
      'Rendered rail does not contain every authoritative profile and browser exactly once');
  }
  const browserRecords = capture.collections.browsers?.browsers ?? capture.collections.browsers ?? [];
  const browserHealth = new Map(browserRecords.map((entry) => [entry.id, entry.health]));
  if (capture.railRows.some((entry) => entry.resourceType === 'browser' &&
      browserHealth.get(entry.resourceId) !== entry.state)) {
    fail('rendered_projection_mismatch',
      'Rendered browser state differs from the authoritative Service browser health');
  }
  await page.screenshot({ path: screenshotPath, fullPage: false });
  const screenshotSha256 = createHash('sha256').update(await readFile(screenshotPath)).digest('hex');
  const authoritativeSnapshotSha256 = sha256(capture.collections);
  return {
    schemaVersion: 'agent-browser.p158-dashboard-live-projection.v1',
    planId: 'P158',
    density: materializationReceipt.density,
    counts: materializationReceipt.counts,
    stateSha256: materializationReceipt.stateSha256,
    inventoryIdentitySha256: materializationReceipt.inventoryIdentitySha256,
    authoritativeSnapshotSha256,
    renderedProjectionSha256: sha256({
      railRows: capture.railRows,
      actionButtons: capture.actionButtons,
      warnings: capture.warnings,
    }),
    externalProof: structuredClone(externalProof),
    capture,
    screenshot: { path: screenshotPath, sha256: screenshotSha256 },
    repairAttempted: false,
    retryAttempted: false,
  };
}

function sorted(values) {
  return [...values].sort((left, right) => left.localeCompare(right));
}

function assertFixtureProjectionBinding(projection, fixture) {
  if (sha256(fixture?.truth?.counts) !== sha256(projection.counts)) {
    fail('dashboard_fixture_binding_mismatch',
      'W5 fixture truth counts do not match the authoritative Service State materialization');
  }
  const expectedRailIds = sorted((fixture?.truth?.resources ?? [])
    .filter((entry) => ['browser', 'profile'].includes(entry.resourceType) && entry.rowExpected === true)
    .map((entry) => entry.resourceId));
  const fixtureRailIds = sorted((fixture?.railRows ?? []).map((entry) => entry.resourceId));
  const observedRailIds = sorted(projection.capture.railRows.map((entry) => entry.resourceId));
  if (sha256(expectedRailIds) !== sha256(observedRailIds) ||
      sha256(fixtureRailIds) !== sha256(observedRailIds)) {
    fail('dashboard_fixture_binding_mismatch',
      'W5 fixture rail resources do not match the externally rendered rail');
  }
  const projectedRows = (rows) => rows.map((entry) => ({
    resourceId: entry.resourceId,
    resourceType: entry.resourceType,
    label: entry.label,
    state: entry.state,
    orderKey: entry.orderKey,
  }));
  if (sha256(projectedRows(fixture.railRows)) !==
      sha256(projectedRows(projection.capture.railRows))) {
    fail('dashboard_fixture_binding_mismatch',
      'W5 fixture rail labels, states, or ordering differ from the externally rendered rail');
  }
  const renderedActions = new Map(projection.capture.actionButtons
    .filter((entry) => entry.actionId)
    .map((entry) => [entry.actionId, entry]));
  for (const action of fixture?.actions ?? []) {
    const observed = renderedActions.get(action.actionId);
    if (action.rendered === true && (!observed ||
        observed.targetResourceId !== action.invokedTargetResourceId ||
        observed.disabled === action.displayedEligible)) {
      fail('dashboard_fixture_binding_mismatch',
        `W5 fixture action ${action.actionId} does not match the externally rendered action`);
    }
  }
  const observedAxes = sorted(projection.capture.warnings.map((entry) => entry.axis).filter(Boolean));
  if (sha256(observedAxes) !== sha256(sorted(fixture?.warnings?.displayedAxes ?? []))) {
    fail('dashboard_fixture_binding_mismatch',
      'W5 fixture warning axes do not match the externally rendered warnings');
  }
}

export function auditP158DashboardLiveProjection({ projection, dashboardFixture }) {
  if (projection?.authoritativeSnapshotSha256 !== sha256(projection?.capture?.collections)) {
    fail('authoritative_snapshot_mismatch', 'Dashboard projection snapshot digest changed before audit');
  }
  if (projection?.renderedProjectionSha256 !== sha256({
    railRows: projection.capture.railRows,
    actionButtons: projection.capture.actionButtons,
    warnings: projection.capture.warnings,
  })) {
    fail('rendered_projection_mismatch', 'Dashboard rendered projection digest changed before audit');
  }
  assertFixtureProjectionBinding(projection, dashboardFixture);
  const report = auditDashboardFixture({ fixture: dashboardFixture });
  return {
    schemaVersion: 'agent-browser.p158-dashboard-live-oracle-binding.v1',
    planId: 'P158',
    density: projection.density,
    authoritativeSnapshotSha256: projection.authoritativeSnapshotSha256,
    renderedProjectionSha256: projection.renderedProjectionSha256,
    dashboardFixtureSha256: sha256(dashboardFixture),
    dashboardOracleReportSha256: sha256(report),
    passed: report.passed,
    findingCodes: report.findings.map((entry) => entry.code),
    repairAttempted: false,
  };
}

export const P158_DASHBOARD_DENSE_COUNTS = DENSE_COUNTS;
