import { classifyOperatorUrl } from './p158-external-handoff-oracle.js';
import { sha256 } from './p158-campaign-controller.js';

export const P158_W8_H03_MANIFEST_SCHEMA = 'agent-browser.p158-w8-h03-external-manifest.v1';
export const P158_W8_H03_RESULT_SCHEMA = 'agent-browser.p158-w8-h03-external-result.v1';

const SHA256 = /^[a-f0-9]{64}$/u;
const COMMIT = /^[a-f0-9]{40}$/u;
const TRANSITIONS = Object.freeze([
  'viewer_expiry', 'route_switch', 'display_replacement', 'provider_session_replacement',
]);
const STABLE_IDENTITY_FIELDS = Object.freeze([
  'browserIdSha256', 'profileIdSha256', 'sessionIdSha256', 'tabIdSha256', 'targetIdSha256',
]);
const CHANGED_AXIS = Object.freeze({
  viewer_expiry: 'viewerLeaseIdSha256', route_switch: 'routeIdSha256',
  display_replacement: 'displayAllocationIdSha256', provider_session_replacement: 'connectionIdSha256',
});
export const P158_W8_H03_PRODUCER_PATHS = Object.freeze({
  workflowPath: '.github/workflows/p158-w8-h03-external.yml',
  runnerPath: 'scripts/run-p158-w8-h03-external.js',
  libraryPath: 'scripts/lib/p158-w8-h03-external.js',
});

export class P158W8H03ExternalError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W8H03ExternalError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W8H03ExternalError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function without(value, field) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== field));
}

function assertNoUnsafeMaterial(value, path = 'value') {
  if (typeof value === 'string') {
    if (/^(?:https?|wss?|file):\/\//iu.test(value) || /(?:localhost|127\.0\.0\.1|\[::1\]|\/remote-view\/)/iu.test(value)) {
      fail('unsafe_url_prohibited', `${path} contains URL material`);
    }
    return;
  }
  if (!value || typeof value !== 'object') return;
  for (const [field, entry] of Object.entries(value)) {
    if (/(?:authorization|cookie|credential|password|privateContent|rawBody|secret|token|vault)/iu.test(field)) {
      fail('private_content_prohibited', `${path}.${field} is prohibited`);
    }
    assertNoUnsafeMaterial(entry, `${path}.${field}`);
  }
}

function artifactReceipt(receipt, expected) {
  if (receipt?.artifactId !== expected.artifactId || receipt.relativePath !== expected.relativePath ||
      receipt.sha256 !== expected.sha256 || receipt.byteCount !== expected.byteCount) {
    fail('artifact_receipt_invalid', `${expected.artifactId} append-only receipt differs from captured bytes`);
  }
  return expected;
}

function validatePublicOrigin(externalIngress) {
  let origin;
  try { origin = new URL(externalIngress?.publicOrigin); } catch {
    fail('external_ingress_unproven', 'H03 external origin is invalid');
  }
  const classification = classifyOperatorUrl(origin.href, {
    role: 'location_header', resolvedAddresses: externalIngress.resolvedAddresses,
  });
  if (origin.protocol !== 'https:' || origin.origin !== origin.href.replace(/\/$/u, '') ||
      classification.findingCodes.length > 0 || externalIngress.offHost !== true ||
      externalIngress.outsideServiceHost !== true || externalIngress.outsideServiceNetworkNamespace !== true ||
      !SHA256.test(externalIngress.runnerAttestationSha256 ?? '')) {
    fail('external_ingress_unproven', 'H03 requires reviewed off-host public HTTPS ingress');
  }
  return origin.origin;
}

function exactH03Actions({ registry, schedule }) {
  const testCase = registry?.cases?.find((entry) => entry.id === 'H03');
  const attempts = schedule?.attempts?.filter((attempt) => attempt.caseId === 'H03') ?? [];
  if (!testCase || attempts.length !== 4 || attempts.some((attempt) => attempt.environmentId !== 'E2' ||
      attempt.externalIngressRequired !== true)) {
    fail('schedule_invalid', 'Frozen H03 schedule is not the four external E2 transitions');
  }
  return attempts.map((attempt) => {
    const transition = attempt.executionUnit?.dimensionAssignment?.value;
    if (!TRANSITIONS.includes(transition)) fail('schedule_invalid', `${attempt.attemptId} transition is invalid`);
    return { actionId: `${attempt.attemptId}:action:001`, attemptId: attempt.attemptId,
      caseId: 'H03', environmentId: 'E2', transition };
  });
}

function validateBinding(binding, action) {
  if (binding?.actionId !== action.actionId || binding.transition !== action.transition ||
      binding.handoffUrlSha256 === undefined) {
    fail('transition_binding_invalid', `${action.actionId} transition binding is missing`);
  }
  if (action.transition === 'viewer_expiry') {
    if (!SHA256.test(binding.viewerLeaseIdSha256 ?? '') || !Number.isInteger(binding.baselineGeneration) ||
        binding.baselineGeneration < 1 || !Number.isInteger(binding.timeoutMs) || binding.timeoutMs < 1) {
      fail('transition_binding_invalid', `${action.actionId} viewer-expiry binding is incomplete`);
    }
  } else if (binding.request?.action !== 'service_remote_view_route_switch' ||
      !binding.request.params || typeof binding.request.params !== 'object' ||
      !SHA256.test(binding.expectedAfterProjectionSha256 ?? '')) {
    fail('transition_binding_invalid', `${action.actionId} route-switch request or expected projection is incomplete`);
  }
  assertNoUnsafeMaterial(binding);
}

export function buildP158W8H03ExternalManifest({
  registry, schedule, seals, sourceCommit, externalIngress, transitionBindings, producer,
}) {
  const publicOrigin = validatePublicOrigin(externalIngress);
  const actions = exactH03Actions({ registry, schedule });
  const retainedIdentitySha256 = seals?.retainedIdentitySha256 ?? sha256(seals?.expectedIdentity);
  if (!COMMIT.test(sourceCommit ?? '') || !SHA256.test(schedule?.scheduleSha256 ?? '') ||
      schedule.registrySha256 !== seals?.registrySha256 || schedule.scheduleSha256 !== seals.scheduleSha256 ||
      !SHA256.test(seals.candidateSha256 ?? '') || !SHA256.test(seals.workflowSha256 ?? '') ||
      !SHA256.test(seals.handoffUrlSha256 ?? '') || !SHA256.test(retainedIdentitySha256 ?? '') ||
      !Array.isArray(transitionBindings) || transitionBindings.length !== actions.length ||
      Object.entries(P158_W8_H03_PRODUCER_PATHS).some(([field, path]) => producer?.[field] !== path ||
        !SHA256.test(producer?.[field.replace('Path', 'Sha256')] ?? ''))) {
    fail('manifest_binding_invalid', 'H03 manifest source, schedule, candidate, or producer binding is incomplete');
  }
  const frozenRunnerAttestationSha256 = sha256({
    provider: 'github_actions',
    runnerLabel: 'ubuntu-latest',
    sourceCommit,
    workflowPath: producer.workflowPath,
    workflowSha256: producer.workflowSha256,
    offHost: true,
    outsideServiceHost: true,
    outsideServiceNetworkNamespace: true,
  });
  if (externalIngress.runnerAttestationSha256 !== frozenRunnerAttestationSha256) {
    fail('external_ingress_unproven', 'H03 runner attestation is not bound to the frozen workflow source');
  }
  const byId = new Map(transitionBindings.map((entry) => [entry.actionId, entry]));
  if (byId.size !== transitionBindings.length) fail('transition_binding_invalid', 'H03 bindings duplicate an action ID');
  const boundActions = actions.map((action) => {
    const binding = byId.get(action.actionId);
    validateBinding(binding, action);
    if (binding.handoffUrlSha256 !== seals.handoffUrlSha256) {
      fail('handoff_digest_mismatch', `${action.actionId} uses another durable handoff`);
    }
    return { ...action, binding: clone(binding) };
  });
  const body = {
    schemaVersion: P158_W8_H03_MANIFEST_SCHEMA, planId: 'P158', sourceCommit,
    scheduleSha256: schedule.scheduleSha256, registrySha256: schedule.registrySha256,
    candidateSha256: seals.candidateSha256, workflowSha256: seals.workflowSha256,
    handoffUrlSha256: seals.handoffUrlSha256, retainedIdentitySha256,
    externalIngress: { publicOriginSha256: sha256(publicOrigin),
      runnerAttestationSha256: externalIngress.runnerAttestationSha256, offHost: true,
      outsideServiceHost: true, outsideServiceNetworkNamespace: true },
    actions: boundActions, producer: clone(producer), repairAllowed: false, retryAllowed: false,
    garbageCollectionAllowed: false,
  };
  assertNoUnsafeMaterial(body);
  return Object.freeze({ ...body, manifestSha256: sha256(body) });
}

export function validateP158W8H03ExternalManifest({ manifest, registry, schedule, seals }) {
  if (manifest?.schemaVersion !== P158_W8_H03_MANIFEST_SCHEMA ||
      manifest.manifestSha256 !== sha256(without(manifest, 'manifestSha256'))) {
    fail('manifest_integrity_mismatch', 'H03 manifest is absent or changed');
  }
  const rebuilt = buildP158W8H03ExternalManifest({ registry, schedule, seals,
    sourceCommit: manifest.sourceCommit,
    externalIngress: { publicOrigin: 'https://reconstruction.example.test', resolvedAddresses: ['203.0.113.10'],
      runnerAttestationSha256: manifest.externalIngress.runnerAttestationSha256,
      offHost: true, outsideServiceHost: true, outsideServiceNetworkNamespace: true },
    transitionBindings: manifest.actions.map((action) => action.binding), producer: manifest.producer });
  const comparable = { ...rebuilt, externalIngress: { ...rebuilt.externalIngress,
    publicOriginSha256: manifest.externalIngress.publicOriginSha256 } };
  comparable.manifestSha256 = sha256(without(comparable, 'manifestSha256'));
  if (comparable.manifestSha256 !== manifest.manifestSha256) {
    fail('manifest_projection_mismatch', 'H03 manifest does not match the frozen action set');
  }
  return manifest;
}

function exactContinuity(proof, manifest, label) {
  const ready = Date.parse(proof?.readyObservedAt);
  const pixels = Date.parse(proof?.pixelsObservedAt);
  if (proof?.operatorVisibleState !== 'ready' || proof.readyBeforePixels !== true ||
      proof.offHost !== true || proof.outsideServiceHost !== true || proof.outsideServiceNetworkNamespace !== true ||
      proof.runnerAttestationSha256 !== manifest.externalIngress.runnerAttestationSha256 ||
      proof.handoffUrlSha256 !== manifest.handoffUrlSha256 ||
      proof.retainedIdentitySha256 !== manifest.retainedIdentitySha256 ||
      proof.browserLaunchCount !== 1 || !Number.isFinite(ready) || !Number.isFinite(pixels) || ready > pixels ||
      !SHA256.test(proof.websocketEndpointSha256 ?? '') || !SHA256.test(proof.runnerIdentitySha256 ?? '') ||
      !(proof.pixelBytes instanceof Uint8Array)) {
    fail('continuity_unproven', `${label} does not prove ready external pixels and retained identity`);
  }
  for (const field of [...STABLE_IDENTITY_FIELDS, 'routeIdSha256', 'displayAllocationIdSha256',
    'connectionIdSha256', 'viewerLeaseIdSha256']) {
    if (!SHA256.test(proof[field] ?? '')) fail('continuity_unproven', `${label}.${field} is missing`);
  }
  if (!Number.isInteger(proof.presentationGeneration) || proof.presentationGeneration < 1) {
    fail('continuity_unproven', `${label} presentation generation is invalid`);
  }
}

async function storePixels(store, manifest, action, phase, bytes) {
  const expected = { artifactId: `p158:${action.actionId}:${phase}`,
    relativePath: `h03/${action.actionId}-${phase}.png`, sha256: sha256(bytes), byteCount: bytes.byteLength };
  return artifactReceipt(await store.writeArtifact({ artifactId: expected.artifactId,
    relativePath: expected.relativePath, content: bytes }), expected);
}

export async function executeP158W8H03ExternalManifest({ manifest, driver, artifactStore, clock }) {
  if (manifest?.schemaVersion !== P158_W8_H03_MANIFEST_SCHEMA ||
      manifest.manifestSha256 !== sha256(without(manifest, 'manifestSha256'))) {
    fail('manifest_integrity_mismatch', 'H03 manifest is absent or changed');
  }
  if (!driver || !artifactStore || typeof driver.captureContinuity !== 'function' ||
      typeof driver.applyTransition !== 'function' || typeof artifactStore.writeArtifact !== 'function') {
    fail('primitive_missing', 'H03 requires external Playwright, service, and append-only artifact primitives');
  }
  const receipts = [];
  for (const action of manifest.actions) {
    const before = await driver.captureContinuity({ action: clone(action), phase: 'before' });
    exactContinuity(before, manifest, `${action.actionId}:before`);
    if (action.transition === 'viewer_expiry' &&
        (before.viewerLeaseIdSha256 !== action.binding.viewerLeaseIdSha256 ||
         before.presentationGeneration !== action.binding.baselineGeneration)) {
      fail('transition_unproven', `${action.actionId} active viewer lease differs from the frozen binding`);
    }
    const transition = await driver.applyTransition({ action: clone(action), before: clone(before) });
    if (transition?.actionId !== action.actionId || transition.observed !== true ||
        transition.requestAttemptCount !== 1 || transition.retryAttempted !== false || transition.repairAttempted !== false) {
      fail('transition_unproven', `${action.actionId} transition was not observed exactly once`);
    }
    const after = await driver.captureContinuity({ action: clone(action), phase: 'after' });
    exactContinuity(after, manifest, `${action.actionId}:after`);
    for (const field of STABLE_IDENTITY_FIELDS) {
      if (before[field] !== after[field]) fail('identity_mismatch', `${action.actionId} changed retained ${field}`);
    }
    if (sha256(before.pixelBytes) !== sha256(after.pixelBytes)) {
      fail('pixel_continuity_mismatch', `${action.actionId} changed the frozen synthetic pixel marker`);
    }
    if (before.runnerIdentitySha256 !== after.runnerIdentitySha256) {
      fail('continuity_unproven', `${action.actionId} changed off-host runner identity`);
    }
    if (after.presentationGeneration <= before.presentationGeneration ||
        before[CHANGED_AXIS[action.transition]] === after[CHANGED_AXIS[action.transition]]) {
      fail('transition_unproven', `${action.actionId} did not change its declared presentation axis`);
    }
    if (action.transition !== 'viewer_expiry' &&
        sha256({ routeIdSha256: after.routeIdSha256, displayAllocationIdSha256: after.displayAllocationIdSha256,
          connectionIdSha256: after.connectionIdSha256, presentationGeneration: after.presentationGeneration }) !==
          action.binding.expectedAfterProjectionSha256) {
      fail('transition_unproven', `${action.actionId} after projection differs from the frozen target`);
    }
    const artifacts = [await storePixels(artifactStore, manifest, action, 'before', before.pixelBytes),
      await storePixels(artifactStore, manifest, action, 'after', after.pixelBytes)];
    const projection = (proof) => ({
      routeIdSha256: proof.routeIdSha256, displayAllocationIdSha256: proof.displayAllocationIdSha256,
      connectionIdSha256: proof.connectionIdSha256, viewerLeaseIdSha256: proof.viewerLeaseIdSha256,
      presentationGeneration: proof.presentationGeneration,
    });
    const body = {
      schemaVersion: 'agent-browser.p158-w8-h03-action-receipt.v1', planId: 'P158',
      actionId: action.actionId, attemptId: action.attemptId, caseId: 'H03',
      candidateSha256: manifest.candidateSha256, workflowSha256: manifest.workflowSha256,
      manifestSha256: manifest.manifestSha256, handoffUrlSha256: manifest.handoffUrlSha256,
      runnerAttestationSha256: manifest.externalIngress.runnerAttestationSha256,
      runnerIdentitySha256: before.runnerIdentitySha256,
      transition: action.transition, before: projection(before), after: projection(after),
      artifactReceipts: artifacts, resultState: 'passed', terminalState: 'completed',
      scenarioOraclePassed: true, attemptNumber: 1, observedAt: clock.wallNow(),
      repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
    };
    assertNoUnsafeMaterial(body);
    receipts.push(Object.freeze({ ...body, receiptSha256: sha256(body) }));
  }
  const body = { schemaVersion: P158_W8_H03_RESULT_SCHEMA, planId: 'P158',
    manifestSha256: manifest.manifestSha256, candidateSha256: manifest.candidateSha256,
    scheduleSha256: manifest.scheduleSha256, actionCount: receipts.length, receipts,
    success: true, retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false };
  return Object.freeze({ ...body, resultSha256: sha256(body) });
}

export function validateP158W8H03ExternalResult({ result, manifest }) {
  const actions = new Map(manifest.actions.map((action) => [action.actionId, action]));
  if (result?.schemaVersion !== P158_W8_H03_RESULT_SCHEMA ||
      result.resultSha256 !== sha256(without(result, 'resultSha256')) || result.success !== true ||
      result.manifestSha256 !== manifest.manifestSha256 || result.actionCount !== manifest.actions.length ||
      !Array.isArray(result.receipts) || new Set(result.receipts.map((receipt) => receipt.actionId)).size !== result.actionCount ||
      manifest.actions.some((action) => !result.receipts.some((receipt) => receipt.actionId === action.actionId)) ||
      result.receipts.some((receipt) => {
        const action = actions.get(receipt.actionId);
        const artifacts = receipt.artifactReceipts ?? [];
        return !action || receipt.receiptSha256 !== sha256(without(receipt, 'receiptSha256')) ||
          receipt.manifestSha256 !== manifest.manifestSha256 || receipt.candidateSha256 !== manifest.candidateSha256 ||
          receipt.workflowSha256 !== manifest.workflowSha256 || receipt.handoffUrlSha256 !== manifest.handoffUrlSha256 ||
          receipt.runnerAttestationSha256 !== manifest.externalIngress.runnerAttestationSha256 ||
          !SHA256.test(receipt.runnerIdentitySha256 ?? '') ||
          receipt.attemptId !== action.attemptId || receipt.caseId !== 'H03' || receipt.transition !== action.transition ||
          receipt.resultState !== 'passed' || receipt.terminalState !== 'completed' ||
          receipt.scenarioOraclePassed !== true || receipt.attemptNumber !== 1 ||
          receipt.retryAttempted !== false || receipt.repairAttempted !== false ||
          receipt.garbageCollectionAttempted !== false || !Number.isFinite(Date.parse(receipt.observedAt)) ||
          !Number.isInteger(receipt.before?.presentationGeneration) ||
          !Number.isInteger(receipt.after?.presentationGeneration) ||
          receipt.after.presentationGeneration <= receipt.before.presentationGeneration ||
          receipt.before[CHANGED_AXIS[action.transition]] === receipt.after[CHANGED_AXIS[action.transition]] ||
          artifacts.length !== 2 || new Set(artifacts.map((entry) => entry.artifactId)).size !== 2 ||
          artifacts.some((entry) => !SHA256.test(entry.sha256 ?? '') || !Number.isInteger(entry.byteCount) ||
            entry.byteCount < 1 || typeof entry.relativePath !== 'string' || !entry.relativePath.startsWith('h03/'));
      })) {
    fail('result_invalid', 'H03 external result is incomplete, changed, or unbound');
  }
  assertNoUnsafeMaterial(result);
  return result;
}

export const P158_W8_H03_H06_EXECUTION_CLASSIFICATION = Object.freeze({
  H03: Object.freeze({ executableAttemptCount: 4, blockedAttemptCount: 0 }),
  H04: Object.freeze({ executableAttemptCount: 0, blockedAttemptCount: 1,
    blocker: 'off_host_guacamole_input_transport_to_viewer_lease_fencing_binding_missing' }),
  H05: Object.freeze({ executableAttemptCount: 0, blockedAttemptCount: 3,
    blocker: 'operator_assisted_human_controller_barrier_not_supplied' }),
  H06: Object.freeze({ executableAttemptCount: 0, blockedAttemptCount: 1,
    blocker: 'native_prompt_minimize_obscure_external_pixel_driver_missing' }),
});
