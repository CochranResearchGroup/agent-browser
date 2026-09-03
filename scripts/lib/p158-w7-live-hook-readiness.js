import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { sha256 } from './p158-campaign-controller.js';

const SOURCE_PATH = 'scripts/lib/p158-w7-live-hook-readiness.js';
const REQUESTED_CASES = Object.freeze([
  'A01', 'A02', 'A03', 'A04', 'A05', 'A06', 'A07', 'A08', 'A09', 'A10', 'A13', 'A15',
  'X01', 'X02', 'X03', 'X04', 'X05', 'X07', 'X08', 'X09', 'X10',
]);
const PRODUCT_BLOCKERS = Object.freeze(['A04', 'A06', 'A11', 'A12', 'A14']);

const FINDINGS = Object.freeze({
  A01: ['distinct_client_transport_identity_unproven', 'resource_ownership_postcondition_missing'],
  A02: ['shared_browser_barrier_driver_missing', 'per_client_tab_ownership_postcondition_missing'],
  A03: ['distinct_connection_transport_missing', 'same_label_connection_oracle_missing'],
  A04: ['acl_fixture_materializer_missing', 'acl_decision_oracle_missing'],
  A05: ['revisioned_policy_barrier_harness_missing', 'effect_time_profile_ownership_unproven'],
  A06: ['revocation_barrier_harness_missing', 'exact_tab_eviction_postcondition_missing'],
  A07: ['command_boundary_crash_matrix_unexecutable'],
  A08: ['unsafe_identity_fixture_materializer_missing', 'identity_action_oracle_missing'],
  A09: ['seven_target_pathology_materializers_missing', 'target_identity_oracle_incomplete'],
  A10: ['owned_foreign_process_fixture_missing', 'effect_time_process_ownership_unproven'],
  A13: ['retained_generation_transition_bundle_missing'],
  A15: ['cross_transport_marker_coordinator_missing', 'history_reconciliation_oracle_missing'],
  X01: ['private_tmp_generation_fixture_missing', 'orphan_display_ownership_receipt_missing'],
  X02: ['multi_daemon_barrier_harness_missing', 'allocator_assignment_oracle_missing'],
  X03: ['display_evidence_fixture_materializer_missing', 'pid_socket_owner_oracle_incomplete'],
  X04: ['forty_display_fixture_materializer_missing', 'foreign_display_non_interference_oracle_missing'],
  X05: ['x11_authority_fixture_materializer_missing', 'authority_matrix_oracle_incomplete'],
  X07: ['supervisor_transition_fixture_missing', 'duplicate_listener_oracle_missing'],
  X08: ['install_transition_driver_incomplete', 'full_shutdown_product_seam_blocked'],
  X09: ['generation_mismatch_fixture_materializer_missing', 'six_axis_digest_oracle_incomplete'],
  X10: ['disposable_host_epoch_driver_missing', 'cross_epoch_identity_oracle_missing'],
  A11: ['five_scheduler_terminal_fault_boundaries_remain_missing',
    'predispatch_denial_live_probe_not_yet_bound_to_frozen_campaign'],
  A12: ['effect_boundary_lock_product_seam_missing'],
  A14: ['development_full_shutdown_product_seam_missing'],
});

const REVIEWED_SOURCES = Object.freeze([
  'packages/client/src/service-request.js',
  'packages/client/src/service-observability.js',
  'cli/src/native/x11_scene.rs',
  'cli/src/native/desktop_input_provider/x11.rs',
  'cli/src/process_identity.rs',
  'cli/src/native/authentication_run.rs',
  'cli/src/native/service_model.rs',
  'scripts/lib/p158-w7-development-adapters.js',
  'scripts/lib/p158-w7-a01-a03-live.js',
  'scripts/lib/p158-w7-a04-a06-live.js',
  'scripts/lib/p158-w7-a07-a13-live.js',
  'scripts/lib/p158-w7-a08-live.js',
  'scripts/lib/p158-w7-a11-predispatch-live.js',
]);

const BUNDLE_SPECS = Object.freeze({
  a01A03LiveBundle: Object.freeze({
    schemaVersion: 'agent-browser.p158-w7-a01-a03-live-bundle.v1',
    caseIds: Object.freeze(['A01', 'A02', 'A03']),
    actionCounts: Object.freeze({ A01: 250, A02: 400, A03: 20 }),
    sourcePath: 'scripts/lib/p158-w7-a01-a03-live.js',
    liveHookIds: Object.freeze(['w7.a01_a03.service_concurrency']),
  }),
  a04A06LiveBundle: Object.freeze({
    schemaVersion: 'agent-browser.p158-w7-a04-a06-live-bundle.v1',
    caseIds: Object.freeze(['A05']),
    actionCounts: Object.freeze({ A05: 12 }),
    sourcePath: 'scripts/lib/p158-w7-a04-a06-live.js',
    liveHookIds: Object.freeze(['w7.a04_a06.profile_policy']),
  }),
  a07A13LiveBundle: Object.freeze({
    schemaVersion: 'agent-browser.p158-w7-a07-a13-live-bundle.v1',
    caseIds: Object.freeze(['A13']),
    actionCounts: Object.freeze({ A13: 25 }),
    sourcePath: 'scripts/lib/p158-w7-a07-a13-live.js',
    liveHookIds: Object.freeze(['w7.a07_a13.retained_generation']),
  }),
  a08LiveBundle: Object.freeze({
    schemaVersion: 'agent-browser.p158-w7-a08-live-bundle.v1',
    caseIds: Object.freeze(['A08']),
    actionCounts: Object.freeze({ A08: 8 }),
    sourcePath: 'scripts/lib/p158-w7-a08-live.js',
    liveHookIds: Object.freeze(['w7.a08.profile_identity_fixture_replay']),
    identityField: 'replayManifestSha256',
    environmentIds: Object.freeze(['E1']),
  }),
});

function sourceDigest(relativePath) {
  return createHash('sha256').update(readFileSync(new URL(`../../${relativePath}`, import.meta.url))).digest('hex');
}

function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value)) freeze(child);
  }
  return value;
}

function same(value, expected) {
  return JSON.stringify(value) === JSON.stringify(expected);
}

function validateLiveBundle(bundle, spec, candidateSha256, environmentSealSha256s) {
  const identityField = spec.identityField ?? 'ownershipManifestSha256';
  const environmentIds = spec.environmentIds ?? ['E0', 'E1'];
  if (!bundle || bundle.schemaVersion !== spec.schemaVersion || bundle.freezeEligible !== true ||
      bundle.providerFree !== false || bundle.candidateSha256 !== candidateSha256 ||
      !/^[a-f0-9]{64}$/u.test(bundle[identityField] ?? '') ||
      !/^[a-f0-9]{64}$/u.test(bundle.liveHookManifestSha256 ?? '') ||
      typeof bundle.campaignRunId !== 'string' || bundle.campaignRunId.length === 0 ||
      !same(bundle.concreteCaseIds, spec.caseIds) || !same(bundle.liveHookIds, spec.liveHookIds) ||
      bundle.driverSource?.sourcePath !== spec.sourcePath ||
      bundle.driverSource?.sourceSha256 !== sourceDigest(spec.sourcePath) ||
      !same(bundle.environmentSealSha256s,
        Object.fromEntries(environmentIds.map((id) => [id, environmentSealSha256s[id]]))) ||
      !Array.isArray(bundle.adapters) || bundle.adapters.length !== spec.caseIds.length ||
      !same(bundle.adapters.map((adapter) => adapter.caseId), spec.caseIds) ||
      bundle.adapters.some((adapter) => adapter.adapterId !== `p158.case.${adapter.caseId}.v1` ||
        !/^[a-f0-9]{64}$/u.test(adapter.executionContractSha256 ?? ''))) {
    return false;
  }
  const expectedBindingSha256 = sha256({
    caseIds: spec.caseIds,
    [identityField]: bundle[identityField],
    campaignRunId: bundle.campaignRunId,
    candidateSha256: bundle.candidateSha256,
    liveHookManifestSha256: bundle.liveHookManifestSha256,
    environmentSealSha256s: bundle.environmentSealSha256s,
    source: bundle.driverSource,
    liveHookIds: spec.liveHookIds,
  });
  if (bundle.adapterBindingSha256 !== expectedBindingSha256) return false;
  if (spec.caseIds.includes('A05')) {
    return bundle.readiness?.counts?.A04?.executable === 0 &&
      bundle.readiness?.counts?.A05?.scheduled === 12 &&
      bundle.readiness?.counts?.A05?.executable === 12 &&
      bundle.readiness?.counts?.A05?.blocked === 0 &&
      bundle.readiness?.counts?.A06?.executable === 0;
  }
  return true;
}

export function auditP158W7LiveHookReadiness({ candidateSha256, environmentSealSha256s,
  a01A03LiveBundle = null, a04A06LiveBundle = null, a07A13LiveBundle = null,
  a08LiveBundle = null }) {
  if (!/^[a-f0-9]{64}$/u.test(candidateSha256 ?? '') ||
      !environmentSealSha256s || ['E0', 'E1'].some((environmentId) =>
        !/^[a-f0-9]{64}$/u.test(environmentSealSha256s[environmentId] ?? '')) ||
      Object.values(environmentSealSha256s).some((digest) => !/^[a-f0-9]{64}$/u.test(digest))) {
    throw Object.assign(new Error('W7 readiness requires frozen candidate and environment seals'), {
      code: 'w7_readiness_seal_missing',
    });
  }
  const sourceEvidence = REVIEWED_SOURCES.map((sourcePath) => ({
    sourcePath,
    sourceSha256: sourceDigest(sourcePath),
  }));
  const concreteActionCounts = new Map();
  const bundles = { a01A03LiveBundle, a04A06LiveBundle, a07A13LiveBundle, a08LiveBundle };
  const validity = Object.fromEntries(Object.entries(bundles).map(([inputName, bundle]) => [inputName,
    validateLiveBundle(bundle, BUNDLE_SPECS[inputName], candidateSha256, environmentSealSha256s)]));
  const validBundles = Object.entries(bundles).filter(([inputName]) => validity[inputName])
    .map(([, bundle]) => bundle);
  if (new Set(validBundles.map((bundle) => bundle.campaignRunId)).size > 1 ||
      new Set(validBundles.map((bundle) => bundle.liveHookManifestSha256)).size > 1) {
    for (const inputName of Object.keys(validity)) validity[inputName] = false;
  }
  for (const [inputName] of Object.entries(bundles)) {
    const spec = BUNDLE_SPECS[inputName];
    if (validity[inputName]) {
      for (const caseId of spec.caseIds) concreteActionCounts.set(caseId, spec.actionCounts[caseId]);
    }
  }
  const cases = [...new Set([...REQUESTED_CASES, ...PRODUCT_BLOCKERS])].map((caseId) => {
    if (concreteActionCounts.has(caseId)) return {
      caseId, requestedMode: 'concrete_live', implementationKind: 'concrete_live',
      blockerKind: null, findingCodes: [], effectsAllowed: true,
      implementedActionCount: concreteActionCounts.get(caseId),
      ownershipReceiptState: 'frozen_and_effect_time_revalidated',
    };
    return {
      caseId, requestedMode: 'concrete_live', implementationKind: 'explicit_blocked',
      blockerKind: PRODUCT_BLOCKERS.includes(caseId) ? 'product_source' : 'campaign_harness',
      findingCodes: [...FINDINGS[caseId]], effectsAllowed: false, implementedActionCount: 0,
      ownershipReceiptState: 'missing_or_not_effect_time_revalidated',
    };
  });
  const concreteCaseIds = cases.filter((entry) => entry.implementationKind === 'concrete_live')
    .map((entry) => entry.caseId);
  const explicitBlockedCaseIds = cases.filter((entry) => entry.implementationKind === 'explicit_blocked')
    .map((entry) => entry.caseId);
  const body = {
    schemaVersion: 'agent-browser.p158-w7-live-hook-readiness.v1',
    candidateSha256,
    environmentSealSha256s: structuredClone(environmentSealSha256s),
    sourceEvidence,
    reviewedCaseCount: cases.length,
    concreteCaseIds,
    explicitBlockedCaseIds,
    cases,
    effectsAttempted: false,
    repairAttempted: false,
    retryAttempted: false,
  };
  return freeze({ ...body, auditSha256: sha256(body) });
}

export function p158W7LiveHookReadinessSourceBinding() {
  return freeze({ sourcePath: SOURCE_PATH, sourceSha256: sourceDigest(SOURCE_PATH) });
}

export const P158_W7_LIVE_HOOK_AUDIT_CASE_IDS = REQUESTED_CASES;
export const P158_W7_PRODUCT_BLOCKED_CASE_IDS = PRODUCT_BLOCKERS;
