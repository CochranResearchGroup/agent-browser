import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { readdir } from 'node:fs/promises';
import { isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, createFileArtifactStore, sha256 } from './p158-campaign-controller.js';
import {
  P158_FINAL_ANALYSIS_RUNNER_SOURCE_PATH,
} from './p158-final-analysis-runner.js';

export const P158_FINAL_ANALYSIS_DESCRIPTOR_SOURCE_PATH =
  'scripts/lib/p158-final-analysis-descriptor.js';
export const P158_FINAL_ANALYSIS_DESCRIPTOR_PATH = 'p158-final-analysis-descriptor.json';

const REPO_ROOT = resolve(new URL('../..', import.meta.url).pathname);
const SHA256 = /^[a-f0-9]{64}$/u;
const STRUCTURED_ROLES = new Set([
  'logging_evidence', 'dashboard_fixture', 'external_handoff_session', 'pressure_samples',
  'logging_operation_gaps', 'analysis_role_assignments', 'evidence_manifest',
]);

export class P158FinalAnalysisDescriptorError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158FinalAnalysisDescriptorError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158FinalAnalysisDescriptorError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function without(value, field) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== field));
}

function sourceSha256() {
  return createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
}

function sourceBindings() {
  return [
    { hookId: 'p158.final_analysis_descriptor',
      sourcePath: P158_FINAL_ANALYSIS_DESCRIPTOR_SOURCE_PATH, sourceSha256: sourceSha256() },
    { hookId: 'p158.final_analysis_runner', sourcePath: P158_FINAL_ANALYSIS_RUNNER_SOURCE_PATH,
      sourceSha256: sha256(readFileSync(resolve(REPO_ROOT, P158_FINAL_ANALYSIS_RUNNER_SOURCE_PATH))) },
  ];
}

async function readOptional(store, path) {
  try { return await store.read(path); } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}

async function writeExact(store, path, value) {
  const bytes = Buffer.from(canonicalJson(value));
  const prior = await readOptional(store, path);
  if (prior !== null && prior !== undefined) {
    if (sha256(prior) !== sha256(bytes)) fail('analysis_descriptor_checkpoint_changed', path);
    return { resumed: true, sha256: sha256(bytes), byteCount: bytes.byteLength };
  }
  await store.writeOnce(path, bytes);
  return { resumed: false, sha256: sha256(bytes), byteCount: bytes.byteLength };
}

function validateGap(gap, runId) {
  if (!['A08', 'A13'].includes(gap?.caseId) || gap.phaseId !== 'W7' || gap.productRequestId !== null ||
      gap.correlationState !== 'product_request_id_unavailable' ||
      gap.loggingGap?.code !== 'product_request_id_not_preserved' ||
      typeof gap.operationCorrelationId !== 'string' || !gap.operationCorrelationId.startsWith(`${runId}:`) ||
      typeof gap.attemptId !== 'string' || typeof gap.actionId !== 'string') {
    fail('logging_operation_gap_invalid', 'Logging operation gaps must preserve exact non-product correlations');
  }
}

function receiptProjection(receipt, analysisRole) {
  if (!receipt?.artifactId || !receipt.relativePath || !SHA256.test(receipt.sha256 ?? '') ||
      !Number.isInteger(receipt.byteCount)) fail('analysis_artifact_receipt_invalid', receipt?.artifactId);
  if (STRUCTURED_ROLES.has(analysisRole) && receipt.mediaType !== 'application/json') {
    fail('analysis_artifact_role_invalid', `${analysisRole} requires application/json`);
  }
  return {
    artifactId: receipt.artifactId, relativePath: receipt.relativePath,
    sha256: receipt.sha256, byteCount: receipt.byteCount,
    mediaType: receipt.mediaType, analysisRole,
    captureState: receipt.captureState, captureGap: receipt.captureGap,
    redactions: clone(receipt.redactions ?? []),
    parentArtifactSha256s: clone(receipt.parentArtifactSha256s ?? []),
  };
}

function validateAssignments(artifacts, assignments) {
  const byId = new Map(assignments.map((entry) => [entry.artifactId, entry.analysisRole]));
  if (byId.size !== assignments.length || artifacts.some((entry) => !byId.has(entry.artifactId)) ||
      assignments.some((entry) => typeof entry.analysisRole !== 'string' || entry.analysisRole.length === 0 ||
        !artifacts.some((artifact) => artifact.artifactId === entry.artifactId))) {
    fail('analysis_artifact_role_inventory_invalid', 'Every pre-analysis artifact requires exactly one role');
  }
  for (const role of ['logging_evidence', 'dashboard_fixture']) {
    if (![...byId.values()].includes(role)) fail('analysis_artifact_role_missing', role);
  }
  return byId;
}

async function walkFiles(root, prefix = '') {
  const result = [];
  for (const entry of await readdir(resolve(root, prefix), { withFileTypes: true })) {
    const relativePath = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) result.push(...await walkFiles(root, relativePath));
    else if (entry.isFile()) result.push(relativePath);
  }
  return result.sort();
}

async function storedPaths(store, runRoot) {
  if (typeof store.paths === 'function') return [...store.paths()].sort();
  return walkFiles(runRoot);
}

function isControlPlanePath(path, authorityPaths) {
  return authorityPaths.has(path) || path.startsWith('ledger/') ||
    path.startsWith('campaign-phases/') || path.startsWith('live-campaign-entrypoint/') ||
    path.startsWith('live-bundle-assembly/') || path.startsWith('w9/checkpoints/');
}

function validateRawInventoryEntry(entry) {
  if (!entry?.artifactId || typeof entry.relativePath !== 'string' ||
      !entry.relativePath.startsWith('artifacts/') || !SHA256.test(entry.sha256 ?? '') ||
      !Number.isInteger(entry.byteCount) || entry.byteCount < 0 ||
      typeof entry.mediaType !== 'string' || typeof entry.analysisRole !== 'string' ||
      !['complete', 'capture_gap'].includes(entry.captureState)) {
    fail('analysis_raw_artifact_inventory_invalid', entry?.artifactId ?? 'missing-artifact-id');
  }
  if (entry.captureState === 'capture_gap' && !entry.captureGap) {
    fail('analysis_raw_artifact_inventory_invalid', `${entry.artifactId} lacks capture gap detail`);
  }
  if (STRUCTURED_ROLES.has(entry.analysisRole) && entry.mediaType !== 'application/json') {
    fail('analysis_artifact_role_invalid', `${entry.analysisRole} requires application/json`);
  }
}

async function registerRawArtifact({ controller, store, entry, registered }) {
  validateRawInventoryEntry(entry);
  const bytes = await readOptional(store, entry.relativePath);
  if (!bytes || sha256(bytes) !== entry.sha256 || bytes.byteLength !== entry.byteCount) {
    fail('analysis_raw_artifact_binding_mismatch', entry.artifactId);
  }
  const prior = registered.get(entry.artifactId);
  if (prior) {
    if (prior.relativePath !== entry.relativePath || prior.sha256 !== entry.sha256 ||
        prior.byteCount !== entry.byteCount) fail('analysis_raw_artifact_binding_mismatch', entry.artifactId);
    return prior;
  }
  const receipt = await controller.writeArtifact({ artifactId: entry.artifactId,
    relativePath: entry.relativePath, content: bytes, metadata: {
      mediaType: entry.mediaType, capturePurpose: entry.analysisRole,
      captureState: entry.captureState, captureGap: clone(entry.captureGap ?? null),
      redactions: clone(entry.redactions ?? []),
      parentArtifactSha256s: clone(entry.parentArtifactSha256s ?? []), adoptExisting: true,
    } });
  if (receipt.relativePath !== entry.relativePath || receipt.sha256 !== entry.sha256 ||
      receipt.byteCount !== entry.byteCount) fail('analysis_raw_artifact_registration_mismatch', entry.artifactId);
  return receipt;
}

function assertExternalRunRoot(runRoot) {
  if (!isAbsolute(runRoot ?? '')) fail('analysis_run_root_invalid', 'Run root must be absolute');
  const fromRepo = relative(REPO_ROOT, resolve(runRoot));
  if (fromRepo === '' || (!fromRepo.startsWith('..') && !isAbsolute(fromRepo))) {
    fail('analysis_run_root_inside_repository', 'Analysis campaign state must remain outside the repository');
  }
}

function verifyBindingBytes(store, binding, label) {
  if (!binding?.relativePath || !SHA256.test(binding.sha256 ?? '') ||
      !Number.isInteger(binding.byteCount)) fail('analysis_authority_binding_invalid', label);
  return Promise.resolve(store.read(binding.relativePath)).then((bytes) => {
    if (!bytes || sha256(bytes) !== binding.sha256 || bytes.byteLength !== binding.byteCount) {
      fail('analysis_authority_binding_mismatch', label);
    }
    return bytes;
  });
}

function normalizedAuthorityBinding(runRoot, binding, label) {
  const supplied = binding?.relativePath ?? binding?.path;
  if (typeof supplied !== 'string') fail('analysis_authority_binding_invalid', label);
  const relativePath = isAbsolute(supplied) ? relative(resolve(runRoot), resolve(supplied)) : supplied;
  if (relativePath === '..' || relativePath.startsWith('../') || isAbsolute(relativePath)) {
    fail('analysis_authority_binding_invalid', `${label} is outside the run root`);
  }
  return { relativePath, sha256: binding.sha256, byteCount: binding.byteCount };
}

export function createP158FinalAnalysisDescriptorHook({ runRoot, controller, artifactStore = null,
  authorities, loggingExpectations = [], architectureCriteria = [], p157Criteria = [],
  loggingOperationGapsSha256, loggingOperationGapCount }) {
  assertExternalRunRoot(runRoot);
  if (typeof controller?.writeArtifact !== 'function' || typeof controller?.snapshot !== 'function') {
    fail('analysis_controller_invalid', 'A ledger-capable controller with writeArtifact and snapshot is required');
  }
  const store = artifactStore ?? createFileArtifactStore(runRoot);
  const frozenSources = sourceBindings();
  return Object.freeze({
    sourcePath: P158_FINAL_ANALYSIS_DESCRIPTOR_SOURCE_PATH,
    sourceSha256: frozenSources[0].sourceSha256,
    async prepareBeforeSeal({ rawArtifactInventory, loggingOperationGaps }) {
      const snapshot = controller.snapshot();
      if (snapshot.state !== 'execution_terminal') {
        fail('analysis_preparation_order_invalid', 'Descriptor preparation runs after execution terminal and before seal');
      }
      if (!Array.isArray(loggingOperationGaps) || loggingOperationGaps.length === 0) {
        fail('logging_operation_gaps_missing', 'The product-correlation gaps artifact is required');
      }
      if (!Array.isArray(rawArtifactInventory) ||
          sha256(loggingOperationGaps) !== loggingOperationGapsSha256 ||
          loggingOperationGaps.length !== loggingOperationGapCount) {
        fail('analysis_preparation_input_changed', 'A13 gaps differ from frozen assembly digests');
      }
      for (const gap of loggingOperationGaps) validateGap(gap, snapshot.runId);
      const artifactIds = new Set();
      const artifactPaths = new Set();
      for (const entry of rawArtifactInventory) {
        validateRawInventoryEntry(entry);
        if (artifactIds.has(entry.artifactId) || artifactPaths.has(entry.relativePath)) {
          fail('analysis_raw_artifact_inventory_invalid', 'Artifact IDs and paths must be unique');
        }
        artifactIds.add(entry.artifactId);
        artifactPaths.add(entry.relativePath);
      }
      const normalizedAuthorities = Object.fromEntries(Object.entries(authorities ?? {}).map(([label, binding]) =>
        [label, normalizedAuthorityBinding(runRoot, binding, label)]));
      const authorityPaths = new Set(Object.values(normalizedAuthorities).map((entry) => entry.relativePath));
      const unclassifiedPaths = (await storedPaths(store, runRoot)).filter((path) =>
        !artifactPaths.has(path) && !isControlPlanePath(path, authorityPaths));
      if (unclassifiedPaths.length > 0) {
        fail('analysis_unclassified_raw_artifact', 'Run root contains unclassified pre-seal files',
          { relativePaths: unclassifiedPaths });
      }
      const registered = new Map((snapshot.evidence?.artifacts ?? []).map((entry) =>
        [entry.artifactId, entry]));
      for (const entry of rawArtifactInventory) {
        await registerRawArtifact({ controller, store, entry, registered });
        registered.set(entry.artifactId, entry);
      }
      const gapEntries = rawArtifactInventory.filter((entry) => entry.analysisRole === 'logging_operation_gaps');
      if (gapEntries.length !== 1) {
        fail('logging_operation_gaps_missing', 'Exactly one harvested operation-gap artifact is required');
      }
      const gapArtifact = controller.snapshot().evidence.artifacts.find((entry) =>
        entry.artifactId === gapEntries[0].artifactId);
      const gapBody = JSON.parse((await store.read(gapArtifact.relativePath)).toString('utf8'));
      if (gapBody.runId !== snapshot.runId || gapBody.operationGapCount !== loggingOperationGaps.length ||
          gapBody.loggingOperationGapsSha256 !== sha256(loggingOperationGaps) ||
          sha256(gapBody.operations) !== sha256(loggingOperationGaps)) {
        fail('logging_operation_gap_invalid', 'Harvested operation-gap artifact differs from the frozen descriptors');
      }
      const artifacts = controller.snapshot().evidence?.artifacts ?? [];
      const assignments = rawArtifactInventory.map((entry) => ({ artifactId: entry.artifactId,
        analysisRole: entry.analysisRole }));
      const byId = validateAssignments(artifacts, assignments);
      const roleBody = {
        schemaVersion: 'agent-browser.p158-analysis-artifact-roles.v1', planId: 'P158',
        runId: snapshot.runId, sourceBindings: frozenSources,
        assignments: artifacts.map((artifact) => receiptProjection(artifact, byId.get(artifact.artifactId))),
        assignmentSetSha256: sha256(artifacts.map((artifact) => ({ artifactId: artifact.artifactId,
          analysisRole: byId.get(artifact.artifactId) }))), effectsAttempted: false, repairAttempted: false,
      };
      const roleArtifact = await controller.writeArtifact({
        artifactId: `${snapshot.runId}:analysis-artifact-roles`,
        relativePath: 'analysis/artifact-roles.json', content: canonicalJson(roleBody),
        metadata: { mediaType: 'application/json', capturePurpose: 'analysis_artifact_roles',
          captureState: 'complete' },
      });
      const body = { schemaVersion: 'agent-browser.p158-analysis-descriptor-preparation.v1',
        runId: snapshot.runId, gapArtifactId: gapArtifact.artifactId,
        gapArtifactSha256: gapArtifact.sha256,
        roleArtifactSha256: roleArtifact.sha256, sourceBindings: frozenSources,
        rawArtifactInventorySha256: sha256(rawArtifactInventory),
        rawArtifactInventoryCount: rawArtifactInventory.length,
        effectsAttempted: false, repairAttempted: false };
      return Object.freeze({ ...body, preparationSha256: sha256(body) });
    },
    async finalizeAfterSeal({ preparation }) {
      const snapshot = controller.snapshot();
      if (snapshot.state !== 'evidence_sealed' || !snapshot.seal ||
          snapshot.evidence?.events?.at(-1)?.recordType !== 'evidence_seal' ||
          snapshot.schedule?.some((attempt) => !attempt.resultState) ||
          !snapshot.scheduledTeardown?.resultState) {
        fail('analysis_finalize_order_invalid', 'Descriptor finalization requires the canonical evidence seal');
      }
      if (preparation?.preparationSha256 !== sha256(without(preparation, 'preparationSha256')) ||
          preparation.runId !== snapshot.runId) fail('analysis_preparation_changed', 'Preparation receipt changed');
      const roleReceipt = snapshot.evidence.artifacts.find((entry) =>
        entry.artifactId === `${snapshot.runId}:analysis-artifact-roles`);
      const gapReceipt = snapshot.evidence.artifacts.find((entry) =>
        entry.artifactId === preparation.gapArtifactId);
      if (roleReceipt?.sha256 !== preparation.roleArtifactSha256 ||
          gapReceipt?.sha256 !== preparation.gapArtifactSha256) {
        fail('analysis_preparation_artifact_missing', 'Prepared analysis artifacts are absent from the seal');
      }
      const roleBytes = await verifyBindingBytes(store, roleReceipt, 'analysis roles');
      await verifyBindingBytes(store, gapReceipt, 'A13 operation gaps');
      const authorityEntries = Object.entries(authorities ?? {}).map(([label, binding]) =>
        [label, normalizedAuthorityBinding(runRoot, binding, label)]);
      if (authorityEntries.map(([label]) => label).sort().join(',') !== 'freeze,manifest,registry,schedule') {
        fail('analysis_authority_binding_invalid', 'Exactly manifest, freeze, schedule, and registry are required');
      }
      const authorityBytes = Object.fromEntries(await Promise.all(authorityEntries.map(async ([label, binding]) =>
        [label, await verifyBindingBytes(store, binding, label)])));
      const authorityBindings = Object.fromEntries(authorityEntries);
      const eventBytes = await Promise.all(snapshot.evidence.events.map((event) => verifyBindingBytes(store, {
          relativePath: `ledger/${String(event.sequence).padStart(8, '0')}-${event.recordType}.json`,
          sha256: event.sha256, byteCount: event.byteCount,
        }, event.recordId)));
      await Promise.all(snapshot.evidence.artifacts.map((artifact) =>
        verifyBindingBytes(store, artifact, artifact.artifactId)));
      const manifest = JSON.parse(authorityBytes.manifest.toString('utf8'));
      const freezeReceipt = JSON.parse(authorityBytes.freeze.toString('utf8'));
      const schedule = JSON.parse(authorityBytes.schedule.toString('utf8'));
      const registry = JSON.parse(authorityBytes.registry.toString('utf8'));
      if (manifest.runId !== snapshot.runId || snapshot.manifestSha256 !== authorities.manifest.sha256 ||
          freezeReceipt.manifestSha256 !== authorities.manifest.sha256 ||
          schedule.registrySha256 !== sha256(registry) || manifest.registrySha256 !== sha256(registry) ||
          schedule.attempts.length !== snapshot.schedule.length) {
        fail('analysis_sealed_authority_mismatch', 'Sealed authorities differ from the controller snapshot');
      }
      let prior = null;
      for (const [index, bytes] of eventBytes.entries()) {
        const record = JSON.parse(bytes.toString('utf8'));
        if (record.sequence !== index || record.previousRecordSha256 !== prior ||
            record.manifestSha256 !== authorities.manifest.sha256) {
          fail('analysis_sealed_ledger_invalid', `Ledger record ${index} is not canonical`);
        }
        prior = sha256(bytes);
      }
      const evidenceManifest = JSON.parse((await store.read(snapshot.seal.relativePath)).toString('utf8'));
      const sealedInventory = [...(evidenceManifest.artifacts ?? []), snapshot.seal];
      const snapshotInventory = [...snapshot.evidence.artifacts, snapshot.seal];
      if (evidenceManifest.runId !== snapshot.runId || sealedInventory.length !== snapshotInventory.length ||
          sealedInventory.some((sealed) => !snapshotInventory.some((artifact) =>
            artifact.artifactId === sealed.artifactId && artifact.sha256 === sealed.sha256 &&
            artifact.byteCount === sealed.byteCount && artifact.relativePath === sealed.relativePath))) {
        fail('analysis_sealed_artifact_inventory_invalid', 'Evidence manifest and controller inventory differ');
      }
      const roleManifest = JSON.parse(roleBytes.toString('utf8'));
      const assignmentDigest = sha256(roleManifest.assignments.map((entry) => ({
        artifactId: entry.artifactId, analysisRole: entry.analysisRole,
      })));
      const normalizedRoleAssignments = roleManifest.assignments.map((entry) =>
        receiptProjection(entry, entry.analysisRole));
      if (roleManifest.runId !== snapshot.runId ||
          roleManifest.assignmentSetSha256 !== assignmentDigest ||
          sha256(normalizedRoleAssignments) !== sha256(roleManifest.assignments) ||
          sha256(roleManifest.sourceBindings) !== sha256(frozenSources)) {
        fail('analysis_artifact_role_inventory_invalid', 'The sealed role assignment artifact changed');
      }
      const roleById = new Map(roleManifest.assignments.map((entry) => [entry.artifactId, entry.analysisRole]));
      const artifactBindings = snapshotInventory.map((artifact) => receiptProjection(artifact,
        artifact.artifactId === roleReceipt.artifactId ? 'analysis_role_assignments' :
          artifact.artifactId === snapshot.seal.artifactId ? 'evidence_manifest' : roleById.get(artifact.artifactId)));
      if (artifactBindings.some((entry) => !entry.analysisRole)) {
        fail('analysis_artifact_role_inventory_invalid', 'The seal contains an unassigned artifact');
      }
      const finalAllowed = new Set([...authorityEntries.map(([, binding]) => binding.relativePath),
        ...snapshotInventory.map((entry) => entry.relativePath)]);
      const finalUnclassified = (await storedPaths(store, runRoot)).filter((path) =>
        !finalAllowed.has(path) && !isControlPlanePath(path, finalAllowed) &&
        ![P158_FINAL_ANALYSIS_DESCRIPTOR_PATH, 'analysis/p158-final-analysis.json',
          'analysis/p158-redacted-review-candidate.json'].includes(path));
      if (finalUnclassified.length > 0) {
        fail('analysis_unclassified_raw_artifact', 'Run root gained unclassified files before finalization',
          { relativePaths: finalUnclassified });
      }
      const descriptor = {
        schemaVersion: 'agent-browser.p158-final-analysis-runner.v1', planId: 'P158', runRoot,
        sourceBindings: frozenSources,
        files: {
          manifest: clone(authorityBindings.manifest), freeze: clone(authorityBindings.freeze),
          schedule: clone(authorityBindings.schedule), registry: clone(authorityBindings.registry),
          evidenceManifest: receiptProjection(snapshot.seal, 'evidence_manifest'),
          ledger: snapshot.evidence.events.map((event) => ({
            relativePath: `ledger/${String(event.sequence).padStart(8, '0')}-${event.recordType}.json`,
            sha256: event.sha256, byteCount: event.byteCount,
          })),
          artifacts: artifactBindings,
        },
        loggingExpectations: clone(loggingExpectations), architectureCriteria: clone(architectureCriteria),
        p157Criteria: clone(p157Criteria),
        output: { analysis: 'analysis/p158-final-analysis.json',
          reviewCandidate: 'analysis/p158-redacted-review-candidate.json' },
      };
      const write = await writeExact(store, P158_FINAL_ANALYSIS_DESCRIPTOR_PATH, descriptor);
      return Object.freeze({ descriptor: clone(descriptor), descriptorSha256: write.sha256,
        descriptorPath: resolve(runRoot, P158_FINAL_ANALYSIS_DESCRIPTOR_PATH), resumed: write.resumed,
        effectsAttempted: false, repairAttempted: false });
    },
  });
}

export function p158FinalAnalysisDescriptorSourceBinding() {
  return Object.freeze(sourceBindings()[0]);
}
