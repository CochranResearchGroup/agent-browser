import { classifyOperatorUrl } from './p158-external-handoff-oracle.js';
import { sha256 } from './p158-campaign-controller.js';

export const P158_W9_ENDURANCE_PREPARATION_SCHEMA =
  'agent-browser.p158-w9-endurance-postcondition-preparation.v1';

const SHA256 = /^[a-f0-9]{64}$/u;
const COMMIT = /^[a-f0-9]{40}$/u;

export class P158W9EndurancePreparationError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W9EndurancePreparationError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W9EndurancePreparationError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function digestBytes(content) {
  if (!(content instanceof Uint8Array)) fail('capture_bytes_missing', 'Visual evidence must be captured bytes');
  return sha256(content);
}

function assertNoRawUrls(value, path = 'receipt') {
  if (typeof value === 'string') {
    if (/^(?:https?|wss?|file):\/\//iu.test(value) || /(?:localhost|127\.0\.0\.1|\[::1\]|\/remote-view\/)/iu.test(value)) {
      fail('raw_url_persistence_prohibited', `${path} contains URL material`);
    }
    return;
  }
  if (!value || typeof value !== 'object') return;
  for (const [field, entry] of Object.entries(value)) assertNoRawUrls(entry, `${path}.${field}`);
}

function validateConfig(config) {
  let url;
  try { url = new URL(config?.handoffUrl); } catch { fail('external_ingress_unproven', 'Handoff URL is invalid'); }
  const classification = classifyOperatorUrl(url.href, {
    role: 'starting_handoff', resolvedAddresses: config.resolvedAddresses,
  });
  if (classification.findingCodes.length > 0 || url.protocol !== 'https:' ||
      sha256(url.href) !== config.handoffUrlSha256) {
    fail('external_ingress_unproven', 'Calibration requires the exact reviewed public HTTPS handoff');
  }
  if (config.runtimeLane !== 'development' || config.production !== false || config.syntheticOnly !== true ||
      !['C04', 'C05'].includes(config.caseId) || typeof config.runId !== 'string' || !config.runId ||
      !COMMIT.test(config.sourceCommit ?? '') || !SHA256.test(config.candidateSha256 ?? '') ||
      !SHA256.test(config.scheduleSha256 ?? '') || !SHA256.test(config.handoffUrlSha256 ?? '') ||
      !SHA256.test(config.retainedIdentitySha256 ?? '') ||
      !SHA256.test(config.syntheticFixtureAttestationSha256 ?? '') ||
      !SHA256.test(config.externalRunnerIdentitySha256 ?? '') ||
      !/^\d+$/u.test(config.workflowRunId ?? '') || !Number.isInteger(config.workflowRunAttempt) ||
      config.workflowRunAttempt < 1 || typeof config.workflowJob !== 'string' || !config.workflowJob ||
      (config.caseId === 'C05' && (!Number.isInteger(config.leaseExpiryTimeoutMs) || config.leaseExpiryTimeoutMs < 1))) {
    fail('preparation_binding_invalid', 'Calibration target or frozen source binding is incomplete');
  }
  return url.href;
}

function exactStartProof(proof, config) {
  const readyAt = Date.parse(proof?.readyObservedAt);
  const pixelsAt = Date.parse(proof?.pixelsObservedAt);
  if (proof?.operatorVisibleState !== 'ready' || proof.readyBeforePixels !== true ||
      proof.offHost !== true || proof.outsideServiceHost !== true ||
      proof.outsideServiceNetworkNamespace !== true ||
      proof.externalRunnerIdentitySha256 !== config.externalRunnerIdentitySha256 ||
      proof.handoffUrlSha256 !== config.handoffUrlSha256 ||
      proof.retainedIdentitySha256 !== config.retainedIdentitySha256 ||
      proof.candidateSha256 !== config.candidateSha256 || proof.scheduleSha256 !== config.scheduleSha256 ||
      proof.runId !== config.runId || !Number.isFinite(readyAt) || !Number.isFinite(pixelsAt) || readyAt > pixelsAt) {
    fail('handoff_readiness_mismatch', 'External browser did not prove ready-before-pixels and exact frozen identity');
  }
}

async function persistCapture(artifactStore, { artifactId, relativePath, content }) {
  const expected = { artifactId, relativePath, sha256: digestBytes(content), byteCount: content.byteLength };
  const observed = await artifactStore.writeArtifact({ artifactId, relativePath, content });
  if (observed?.artifactId !== expected.artifactId || observed.relativePath !== expected.relativePath ||
      observed.sha256 !== expected.sha256 || observed.byteCount !== expected.byteCount) {
    fail('artifact_receipt_mismatch', `${artifactId} append-only receipt differs from captured bytes`);
  }
  return expected;
}

function exactActions(config, actions, dashboardProbes) {
  if (!Array.isArray(actions) || !Array.isArray(dashboardProbes)) {
    fail('preparation_action_set_invalid', 'Actions and dashboard probes must be arrays');
  }
  const dashboard = actions.filter((action) => action.caseId === config.caseId && action.kind === 'dashboard_action');
  const probes = new Map(dashboardProbes.map((probe) => [probe.actionId, probe]));
  if (new Set(actions.map((action) => action.actionId)).size !== actions.length || probes.size !== dashboardProbes.length ||
      probes.size !== dashboard.length || dashboard.some((action) => !probes.has(action.actionId))) {
    fail('preparation_action_set_invalid', 'Dashboard probes must bijectively cover the frozen action IDs');
  }
  for (const probe of probes.values()) {
    const region = probe.region;
    if (!region || !['x', 'y', 'width', 'height'].every((field) => Number.isInteger(region[field]) && region[field] >= 0) ||
        region.width < 1 || region.height < 1 || !probe.interaction || typeof probe.interaction !== 'object') {
      fail('preparation_action_set_invalid', `${probe.actionId} probe geometry or interaction is invalid`);
    }
  }
  return { dashboard, probes };
}

function exactLease(records, role) {
  const matching = records.filter((lease) => (lease.viewerRole ?? lease.role) === role && lease.state === 'active');
  if (matching.length !== 1 || !Number.isInteger(matching[0].generation ?? matching[0].leaseGeneration) ||
      (matching[0].generation ?? matching[0].leaseGeneration) < 1 ||
      typeof (matching[0].id ?? matching[0].viewerLeaseId) !== 'string') {
    fail('lease_baseline_unproven', `Expected one active ${role} lease with a positive generation`);
  }
  const lease = matching[0];
  return {
    leaseIdSha256: sha256(lease.id ?? lease.viewerLeaseId), viewerRole: role, state: 'active',
    generation: lease.generation ?? lease.leaseGeneration,
  };
}

export async function prepareP158W9EndurancePostconditions({
  config, actions, dashboardProbes = [], browser, artifactStore,
  clock = { wallNow: () => new Date().toISOString() },
}) {
  const handoffUrl = validateConfig(config);
  if (!browser || !artifactStore || typeof browser.openHandoff !== 'function' ||
      typeof browser.resetSyntheticFixture !== 'function' ||
      typeof browser.captureRegion !== 'function' || typeof browser.performDashboardAction !== 'function' ||
      typeof artifactStore.writeArtifact !== 'function' || (config.caseId === 'C05' &&
        (typeof browser.readViewerLeases !== 'function' || typeof browser.probeNetworkRecovery !== 'function' ||
          typeof browser.probeClientRestart !== 'function'))) {
    fail('preparation_primitive_missing', 'External browser, API, and append-only artifact primitives are required');
  }
  const { dashboard, probes } = exactActions(config, actions, dashboardProbes);
  const startProof = await browser.openHandoff({ handoffUrl, expected: clone(config) });
  exactStartProof(startProof, config);
  const artifactReceipts = [];
  const preparedActions = [];
  for (const action of dashboard) {
    const probe = probes.get(action.actionId);
    const resetProof = await browser.resetSyntheticFixture({ action: clone(action), probe: clone(probe), handoffUrl });
    exactStartProof(resetProof, config);
    const before = await browser.captureRegion({ action: clone(action), region: clone(probe.region), phase: 'before' });
    const interaction = await browser.performDashboardAction({ action: clone(action), interaction: clone(probe.interaction) });
    if (interaction?.actionId !== action.actionId || interaction.observed !== true) {
      fail('dashboard_interaction_unproven', `${action.actionId} interaction was not observed`);
    }
    const after = await browser.captureRegion({ action: clone(action), region: clone(probe.region), phase: 'after' });
    const beforeSha256 = digestBytes(before);
    const afterSha256 = digestBytes(after);
    if (beforeSha256 === afterSha256) {
      fail('static_visual_hash_rejected', `${action.actionId} before and after pixels are identical`);
    }
    const beforeReceipt = await persistCapture(artifactStore, {
      artifactId: `p158:${config.runId}:${action.actionId}:before`,
      relativePath: `postconditions/${action.actionId}-before.png`, content: before,
    });
    const afterReceipt = await persistCapture(artifactStore, {
      artifactId: `p158:${config.runId}:${action.actionId}:after`,
      relativePath: `postconditions/${action.actionId}-after.png`, content: after,
    });
    artifactReceipts.push(beforeReceipt, afterReceipt);
    preparedActions.push({ ...clone(action), postcondition: {
      kind: 'pixel_region_transition', region: clone(probe.region), beforeSha256, afterSha256,
      beforeArtifactId: beforeReceipt.artifactId, afterArtifactId: afterReceipt.artifactId,
    } });
  }
  let leaseBaselines = [];
  let eventPostconditions = {};
  if (config.caseId === 'C05') {
    const leases = await browser.readViewerLeases();
    if (!Array.isArray(leases)) fail('lease_baseline_unproven', 'Viewer lease API did not return an array');
    const viewer = exactLease(leases, 'viewer');
    const controller = exactLease(leases, 'controller');
    if (viewer.leaseIdSha256 === controller.leaseIdSha256) {
      fail('lease_baseline_unproven', 'Viewer and controller lease identities were not distinct');
    }
    leaseBaselines = [viewer, controller];
    const network = await browser.probeNetworkRecovery({ handoffUrl, expected: clone(config) });
    const restart = await browser.probeClientRestart({ handoffUrl, expected: clone(config) });
    for (const [name, proof] of Object.entries({ network, restart })) {
      exactStartProof(proof, config);
      if (name === 'network' && proof.offlineFailureObserved !== true) {
        fail('network_recovery_baseline_unproven', 'Offline failure was not observed');
      }
      if (name === 'restart' && proof.clientRestartObserved !== true) {
        fail('client_restart_baseline_unproven', 'Client restart was not observed');
      }
      const receipt = await persistCapture(artifactStore, {
        artifactId: `p158:${config.runId}:${config.caseId}:${name}-baseline`,
        relativePath: `postconditions/${config.caseId}-${name}-baseline.png`, content: proof.pixelBytes,
      });
      artifactReceipts.push(receipt);
    }
    eventPostconditions = {
    viewer_expiry: { kind: 'authoritative_lease_expiry', leaseIdSha256: viewer.leaseIdSha256,
      viewerRole: 'viewer', fromState: 'active', toState: 'expired', baselineGeneration: viewer.generation,
      timeoutMs: config.leaseExpiryTimeoutMs },
    controller_expiry: { kind: 'authoritative_lease_expiry', leaseIdSha256: controller.leaseIdSha256,
      viewerRole: 'controller', fromState: 'active', toState: 'expired', baselineGeneration: controller.generation,
      timeoutMs: config.leaseExpiryTimeoutMs },
    client_restart: { kind: 'retained_identity_reopen', retainedIdentitySha256: config.retainedIdentitySha256,
      baselineArtifactId: artifactReceipts.find((entry) => entry.artifactId.endsWith(':restart-baseline')).artifactId },
    scheduled_network_profile: { kind: 'offline_failure_then_unchanged_handoff_recovery',
      baselineArtifactId: artifactReceipts.find((entry) => entry.artifactId.endsWith(':network-baseline')).artifactId },
    };
  }
  const body = {
    schemaVersion: P158_W9_ENDURANCE_PREPARATION_SCHEMA, planId: 'P158', caseId: config.caseId,
    runId: config.runId, sourceCommit: config.sourceCommit, candidateSha256: config.candidateSha256,
    scheduleSha256: config.scheduleSha256, handoffUrlSha256: config.handoffUrlSha256,
    retainedIdentitySha256: config.retainedIdentitySha256,
    externalRunnerIdentitySha256: config.externalRunnerIdentitySha256,
    workflowRunId: config.workflowRunId, workflowRunAttempt: config.workflowRunAttempt,
    workflowJob: config.workflowJob,
    syntheticFixtureAttestationSha256: config.syntheticFixtureAttestationSha256,
    preparedAt: clock.wallNow(), externalIngress: true, providerFree: false, syntheticOnly: true,
    dashboardActionCount: preparedActions.length, actionPostconditionsSha256: sha256(preparedActions.map((action) => ({
      actionId: action.actionId, postcondition: action.postcondition,
    }))),
    leaseBaselines, eventPostconditionsSha256: sha256(eventPostconditions),
    artifactReceipts, retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
  };
  assertNoRawUrls(body);
  const receipt = Object.freeze({ ...body, postconditionPreparationSha256: sha256(body) });
  return Object.freeze({ receipt, preparedActions: Object.freeze(preparedActions),
    eventPostconditions: Object.freeze(eventPostconditions) });
}
