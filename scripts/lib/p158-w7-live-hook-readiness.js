import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { sha256 } from './p158-campaign-controller.js';

const SOURCE_PATH = 'scripts/lib/p158-w7-live-hook-readiness.js';
const REQUESTED_CASES = Object.freeze([
  'A01', 'A02', 'A03', 'A04', 'A05', 'A06', 'A08', 'A09', 'A10', 'A15',
  'X01', 'X02', 'X03', 'X04', 'X05', 'X07', 'X08', 'X09', 'X10',
]);
const PRODUCT_BLOCKERS = Object.freeze(['A11', 'A12', 'A14']);

const FINDINGS = Object.freeze({
  A01: ['distinct_client_transport_identity_unproven', 'resource_ownership_postcondition_missing'],
  A02: ['shared_browser_barrier_driver_missing', 'per_client_tab_ownership_postcondition_missing'],
  A03: ['distinct_connection_transport_missing', 'same_label_connection_oracle_missing'],
  A04: ['acl_fixture_materializer_missing', 'acl_decision_oracle_missing'],
  A05: ['revisioned_policy_barrier_harness_missing', 'effect_time_profile_ownership_unproven'],
  A06: ['revocation_barrier_harness_missing', 'exact_tab_eviction_postcondition_missing'],
  A08: ['unsafe_identity_fixture_materializer_missing', 'identity_action_oracle_missing'],
  A09: ['seven_target_pathology_materializers_missing', 'target_identity_oracle_incomplete'],
  A10: ['owned_foreign_process_fixture_missing', 'effect_time_process_ownership_unproven'],
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
  A11: ['scheduler_terminal_fault_product_seam_missing'],
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
]);

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

export function auditP158W7LiveHookReadiness({ candidateSha256, environmentSealSha256s }) {
  if (!/^[a-f0-9]{64}$/u.test(candidateSha256 ?? '') ||
      !environmentSealSha256s || Object.values(environmentSealSha256s)
        .some((digest) => !/^[a-f0-9]{64}$/u.test(digest))) {
    throw Object.assign(new Error('W7 readiness requires frozen candidate and environment seals'), {
      code: 'w7_readiness_seal_missing',
    });
  }
  const sourceEvidence = REVIEWED_SOURCES.map((sourcePath) => ({
    sourcePath,
    sourceSha256: sourceDigest(sourcePath),
  }));
  const cases = [...REQUESTED_CASES, ...PRODUCT_BLOCKERS].map((caseId) => ({
    caseId,
    requestedMode: 'concrete_live',
    implementationKind: 'explicit_blocked',
    blockerKind: PRODUCT_BLOCKERS.includes(caseId) ? 'product_source' : 'campaign_harness',
    findingCodes: [...FINDINGS[caseId]],
    effectsAllowed: false,
    implementedActionCount: 0,
    ownershipReceiptState: 'missing_or_not_effect_time_revalidated',
  }));
  const body = {
    schemaVersion: 'agent-browser.p158-w7-live-hook-readiness.v1',
    candidateSha256,
    environmentSealSha256s: structuredClone(environmentSealSha256s),
    sourceEvidence,
    reviewedCaseCount: cases.length,
    concreteCaseIds: [],
    explicitBlockedCaseIds: cases.map((entry) => entry.caseId),
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
