import { createHash } from 'node:crypto';

export const LOGGING_SURFACES = Object.freeze([
  'request',
  'immediate_response',
  'job',
  'event',
  'trace',
  'incident',
  'dashboard_projection',
]);

export const LOGGING_FINDING_CODES = Object.freeze([
  'missing_record',
  'duplicate_terminal',
  'conflicting_projection',
  'timestamp_inversion',
  'null_failure',
  'null_provenance',
  'one_transport_only',
  'broken_parent',
  'effect_retry_conflict',
  'capture_gap',
  'sensitive_value_leak',
]);

const INPUT_SURFACES = Object.freeze({
  requests: 'request',
  immediateResponses: 'immediate_response',
  jobs: 'job',
  events: 'event',
  traces: 'trace',
  incidents: 'incident',
  dashboardProjections: 'dashboard_projection',
});

const SCHEMA_SURFACE_ROLES = Object.freeze({
  request: 'ingress_request',
  immediate_response: 'immediate_response',
  job: 'durable_job',
  event: 'terminal_event',
  trace: 'trace_outcome',
  incident: 'incident',
  dashboard_projection: 'dashboard_projection',
});

const ROLE_TO_SURFACE = Object.freeze(
  Object.fromEntries(Object.entries(SCHEMA_SURFACE_ROLES).map(([surface, role]) => [role, surface])),
);

const ID_FIELDS = Object.freeze({
  request: ['recordId', 'id', 'requestId'],
  immediate_response: ['recordId', 'id', 'responseId'],
  job: ['recordId', 'id', 'jobId'],
  event: ['recordId', 'id', 'eventId'],
  trace: ['recordId', 'id', 'traceId'],
  incident: ['recordId', 'id', 'incidentId'],
  dashboard_projection: ['recordId', 'id', 'projectionId'],
});

const CAUSAL_LINK_FIELDS = Object.freeze([
  'requestId',
  'jobId',
  'eventId',
  'traceId',
  'incidentId',
  'attemptId',
]);

const TERMINAL_STATUSES = new Set([
  'completed',
  'failed',
  'cancelled',
  'canceled',
  'timed_out',
  'timeout',
  'rejected',
  'stopped',
  'terminal',
]);

const FAILURE_STATUSES = new Set([
  'failed',
  'cancelled',
  'canceled',
  'timed_out',
  'timeout',
  'rejected',
  'stopped',
]);

const DEFAULT_FORBIDDEN_FIELDS = Object.freeze([
  'credentialCharacters',
  'passkeyAssertions',
  'cookies',
  'bearerTokens',
  'capabilityMaterial',
  'passwordManagerVaultContent',
  'privatePageBodies',
  'rawProfilePaths',
  'providerAccountLabels',
]);

const REDACTED_VALUES = new Set([
  '[redacted]',
  '<redacted>',
  '[excluded]',
  '<excluded>',
  '[hashed]',
  '<hashed>',
]);

function clone(value) {
  return structuredClone(value);
}

function canonicalize(value, seen = new Set()) {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return value;
  if (typeof value === 'number') return Number.isFinite(value) ? value : String(value);
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { $bytes: Buffer.from(value).toString('base64') };
  if (typeof value !== 'object') return String(value);
  if (seen.has(value)) return '[circular]';
  seen.add(value);
  const normalized = Array.isArray(value)
    ? value.map((entry) => canonicalize(entry, seen))
    : Object.fromEntries(
        Object.keys(value)
          .sort()
          .filter((key) => value[key] !== undefined)
          .map((key) => [key, canonicalize(value[key], seen)]),
      );
  seen.delete(value);
  return normalized;
}

export function stableValueHash(value) {
  return createHash('sha256').update(JSON.stringify(canonicalize(value))).digest('hex');
}

function firstDefined(record, fields) {
  for (const field of fields) {
    if (record[field] !== undefined && record[field] !== null) return record[field];
  }
  return undefined;
}

function timestampValue(record) {
  const raw = firstDefined(record, [
    'timestamp',
    'wallTime',
    'completedAt',
    'createdAt',
    'updatedAt',
  ]);
  if (raw === undefined) return null;
  const parsed = typeof raw === 'number' ? raw : Date.parse(raw);
  if (!Number.isFinite(parsed)) return null;
  return parsed - Number(record.clockOffsetMilliseconds ?? 0);
}

function isTerminal(record, surface) {
  if (surface === 'trace') return true;
  if (record.terminal === true) return true;
  if (TERMINAL_STATUSES.has(String(record.status ?? record.state ?? '').toLowerCase())) return true;
  if (String(record.recordType ?? '').endsWith('_terminal')) return true;
  if (surface === 'trace' && record.outcome !== undefined) return true;
  return false;
}

function isFailed(record) {
  if (record.success === false || record.ok === false) return true;
  if (FAILURE_STATUSES.has(String(record.status ?? record.state ?? '').toLowerCase())) return true;
  if (record.resultState && record.resultState !== 'passed') return true;
  return record.failure !== undefined && record.failure !== null;
}

function recordCausalIds(record) {
  const ids = {};
  for (const field of CAUSAL_LINK_FIELDS) {
    const value = record[field] ?? record.causalIds?.[field];
    if (value !== undefined && value !== null && value !== '') ids[field] = String(value);
  }
  return ids;
}

function normalizeRecord(surface, record, index) {
  const id = String(firstDefined(record, ID_FIELDS[surface]) ?? `${surface}:${index}`);
  const parentIds = [
    ...(Array.isArray(record.parentIds) ? record.parentIds : []),
    ...[
      record.parentId,
      record.parentRecordId,
      record.previousRecordId,
      record.causedByRecordId,
    ].filter((value) => value !== undefined && value !== null),
  ].map(String);
  return {
    surface,
    id,
    index,
    nodeId: `${surface}:${id}:${index}`,
    causalIds: recordCausalIds(record),
    parentIds: [...new Set(parentIds)].sort(),
    timestamp: timestampValue(record),
    terminal: isTerminal(record, surface),
    failed: isFailed(record),
    failure: record.failure ?? record.structuredFailure ?? record.outcome?.failure ?? null,
    provenance: record.provenance ?? record.immutableProvenance ?? record.outcome?.provenance ?? null,
    effectState: record.effectState ?? record.outcome?.effectState ?? null,
    retryDisposition: record.retryDisposition ?? record.outcome?.retryDisposition ?? null,
    transport: record.transport ?? record.provenance?.transport ?? null,
    original: clone(record),
  };
}

export function normalizeAuditInput(input) {
  const source = coerceFixtureInput(input ?? {});
  const records = [];
  for (const [inputField, surface] of Object.entries(INPUT_SURFACES)) {
    const entries = source[inputField] ?? [];
    if (!Array.isArray(entries)) throw new TypeError(`${inputField} must be an array`);
    entries.forEach((record, index) => records.push(normalizeRecord(surface, record, index)));
  }
  return {
    fixtureSetId: source.fixtureSetId ?? source.id ?? null,
    runId: source.runId ?? null,
    records,
    artifacts: clone(source.artifacts ?? []),
    redactionReceipts: clone(source.redactionReceipts ?? []),
    expectations: clone(source.expectations ?? {}),
    forbiddenFields: clone(source.forbiddenFields ?? []),
    sensitiveCanaries: clone(source.sensitiveCanaries ?? []),
    canaryDefinitions: clone(source.canaryDefinitions ?? []),
  };
}

function coerceFixtureInput(input) {
  if (!Array.isArray(input.records) && !Array.isArray(input.fixtures)) return input;
  const fixtures = Array.isArray(input.fixtures) ? input.fixtures : [input];
  const result = {
    fixtureSetId: input.fixtureSetId ?? input.id ?? input.schemaVersion ?? 'p158-logging-fixtures',
    runId: input.runId ?? 'p158-logging-audit',
    requests: [],
    immediateResponses: [],
    jobs: [],
    events: [],
    traces: [],
    incidents: [],
    dashboardProjections: [],
    artifacts: clone(input.artifacts ?? []),
    redactionReceipts: clone(input.redactionReceipts ?? []),
    expectations: { byRequestId: {} },
    forbiddenFields: clone(input.forbiddenFields ?? []),
    sensitiveCanaries: (input.canaries ?? []).map((canary) => canary.value),
    canaryDefinitions: clone(input.canaries ?? []),
  };
  const inputKeyBySurface = Object.fromEntries(
    Object.entries(INPUT_SURFACES).map(([inputKey, surface]) => [surface, inputKey]),
  );
  for (const fixture of fixtures) {
    const records = fixture.records ?? [];
    const requestId =
      records.find((record) => record.surfaceRole === 'ingress_request')?.requestId ??
      records.find((record) => record.requestId)?.requestId ??
      `missing-request:${stableValueHash(fixture.fixtureId ?? fixture).slice(0, 16)}`;
    result.expectations.byRequestId[requestId] = {
      expectedSurfaces: (fixture.expectedSurfaceRoles ?? []).map((role) => ROLE_TO_SURFACE[role]),
      incidentExpected: fixture.incidentExpected ?? false,
      dashboardExpected: fixture.operatorVisible ?? false,
      operatorVisible: fixture.operatorVisible ?? false,
    };
    for (const record of records) {
      const surface = ROLE_TO_SURFACE[record.surfaceRole];
      if (!surface) throw new TypeError(`Unknown surfaceRole: ${record.surfaceRole}`);
      result[inputKeyBySurface[surface]].push({ ...record, auditFixtureId: fixture.fixtureId });
    }
  }
  return result;
}

class DisjointSet {
  constructor(size) {
    this.parents = Array.from({ length: size }, (_value, index) => index);
  }

  find(index) {
    if (this.parents[index] !== index) this.parents[index] = this.find(this.parents[index]);
    return this.parents[index];
  }

  union(left, right) {
    const leftRoot = this.find(left);
    const rightRoot = this.find(right);
    if (leftRoot === rightRoot) return;
    const low = Math.min(leftRoot, rightRoot);
    const high = Math.max(leftRoot, rightRoot);
    this.parents[high] = low;
  }
}

function buildGroups(records) {
  const sets = new DisjointSet(records.length);
  const identifiers = new Map();
  records.forEach((record, index) => {
    for (const [field, value] of Object.entries(record.causalIds)) {
      const token = `${field}:${value}`;
      if (identifiers.has(token)) sets.union(index, identifiers.get(token));
      else identifiers.set(token, index);
    }
  });
  const grouped = new Map();
  records.forEach((record, index) => {
    const root = sets.find(index);
    if (!grouped.has(root)) grouped.set(root, []);
    grouped.get(root).push(record);
  });
  return [...grouped.values()]
    .map((groupRecords) => {
      const fixtureIds = [...new Set(groupRecords.map((record) => record.original.auditFixtureId).filter(Boolean))].sort();
      const requestIds = [...new Set(groupRecords.map((record) => record.causalIds.requestId).filter(Boolean))].sort();
      const tokens = groupRecords
        .flatMap((record) => Object.entries(record.causalIds).map(([field, value]) => `${field}:${value}`))
        .sort();
      return {
        groupId:
          fixtureIds[0] ??
          requestIds[0] ??
          tokens[0] ??
          groupRecords.map((record) => record.nodeId).sort()[0],
        fixtureId: fixtureIds[0] ?? null,
        records: groupRecords.sort(
          (left, right) =>
            (left.timestamp ?? Number.MAX_SAFE_INTEGER) -
              (right.timestamp ?? Number.MAX_SAFE_INTEGER) ||
            left.surface.localeCompare(right.surface) ||
            left.id.localeCompare(right.id) ||
            left.index - right.index,
        ),
      };
    })
    .sort((left, right) => left.groupId.localeCompare(right.groupId));
}

function expectedSurfacesFor(group, globalExpectations) {
  const request = group.records.find((record) => record.surface === 'request')?.original ?? {};
  const requestId = group.records.find((record) => record.causalIds.requestId)?.causalIds.requestId;
  const specific = globalExpectations.byRequestId?.[requestId] ?? {};
  const declared = specific.expectedSurfaces ?? request.expectedSurfaces;
  const expected = Object.fromEntries(LOGGING_SURFACES.map((surface) => [surface, 0]));
  const base = declared ?? ['request', 'immediate_response', 'job', 'event', 'trace'];
  for (const surface of base) expected[surface] = 1;
  if (specific.incidentExpected ?? request.incidentExpected ?? request.expectedIncident ?? false) {
    expected.incident = 1;
  }
  if (specific.dashboardExpected ?? request.dashboardExpected ?? request.operatorVisible ?? false) {
    expected.dashboard_projection = 1;
  }
  return { expected, request, specific };
}

function relevantSurfaceRecords(group, surface) {
  const records = group.records.filter((record) => record.surface === surface);
  return records;
}

function normalizeFieldName(value) {
  return String(value).replaceAll(/[^a-zA-Z0-9]/g, '').toLowerCase();
}

function isRedactedValue(value) {
  return typeof value === 'string' && REDACTED_VALUES.has(value.trim().toLowerCase());
}

function scanSensitive(value, { forbiddenNames, canaries }, path = '$', seen = new Set()) {
  const findings = [];
  if (typeof value === 'string') {
    for (const canary of canaries) {
      if (canary.value !== '' && value.includes(canary.value)) {
        findings.push({
          kind: 'canary',
          path,
          canaryHash: stableValueHash(canary.value),
          syntheticCanaryId: canary.canaryId,
        });
      }
    }
    return findings;
  }
  if (value === null || typeof value !== 'object') return findings;
  if (seen.has(value)) return findings;
  seen.add(value);
  if (value instanceof Uint8Array) {
    const text = Buffer.from(value).toString('utf8');
    findings.push(...scanSensitive(text, { forbiddenNames, canaries }, path, seen));
  } else if (Array.isArray(value)) {
    value.forEach((entry, index) => findings.push(...scanSensitive(entry, { forbiddenNames, canaries }, `${path}[${index}]`, seen)));
  } else {
    for (const [key, entry] of Object.entries(value)) {
      const childPath = `${path}.${key}`;
      if (key === 'value' && value.redacted === true) continue;
      if (
        forbiddenNames.has(normalizeFieldName(key)) &&
        entry !== null &&
        entry !== undefined &&
        !isRedactedValue(entry)
      ) {
        findings.push({ kind: 'forbidden_field', path: childPath, fieldClass: key });
      }
      findings.push(...scanSensitive(entry, { forbiddenNames, canaries }, childPath, seen));
    }
  }
  seen.delete(value);
  return findings;
}

function findingSort(left, right) {
  return (
    left.code.localeCompare(right.code) ||
    String(left.groupId ?? '').localeCompare(String(right.groupId ?? '')) ||
    String(left.surface ?? '').localeCompare(String(right.surface ?? '')) ||
    String(left.recordId ?? '').localeCompare(String(right.recordId ?? '')) ||
    stableValueHash(left).localeCompare(stableValueHash(right))
  );
}

function addFinding(findings, finding) {
  const normalized = {
    code: finding.code,
    severity: finding.severity ?? 'error',
    groupId: finding.groupId ?? null,
    surface: finding.surface ?? null,
    recordId: finding.recordId ?? null,
    details: clone(finding.details ?? {}),
  };
  const key = `${normalized.code}\0${normalized.groupId ?? 'global'}`;
  if (!findings.some((entry) => entry.key === key)) findings.push({ key, ...normalized });
}

export function auditLoggingCompleteness(input, options = {}) {
  const inputSha256 = stableValueHash(input);
  const normalized = normalizeAuditInput(input);
  const groups = buildGroups(normalized.records);
  const findings = [];
  const surfaceCounts = Object.fromEntries(
    LOGGING_SURFACES.map((surface) => [surface, { expected: 0, observed: 0, missing: 0, duplicate: 0, conflicting: 0 }]),
  );
  const idIndex = new Map();
  for (const record of normalized.records) {
    for (const id of new Set([record.id, ...Object.values(record.causalIds)])) {
      if (!idIndex.has(id)) idIndex.set(id, []);
      idIndex.get(id).push(record);
    }
  }

  for (const group of groups) {
    const { expected, request, specific } = expectedSurfacesFor(group, normalized.expectations);
    const groupFindingStart = findings.length;
    for (const surface of LOGGING_SURFACES) {
      const observedRecords = relevantSurfaceRecords(group, surface);
      const observed = observedRecords.length;
      const expectedCount = expected[surface];
      const missing = Math.max(0, expectedCount - observed);
      const duplicate = Math.max(0, observed - expectedCount);
      surfaceCounts[surface].expected += expectedCount;
      surfaceCounts[surface].observed += observed;
      surfaceCounts[surface].missing += missing;
      surfaceCounts[surface].duplicate += duplicate;
      if (missing > 0) {
        addFinding(findings, {
          code: 'missing_record',
          groupId: group.groupId,
          surface,
          details: { expected: expectedCount, observed, missing },
        });
      }
      const duplicateTerminal = Math.max(
        0,
        observedRecords.filter((record) => record.terminal).length - 1,
      );
      if (duplicateTerminal > 0) {
        addFinding(findings, {
          code: 'duplicate_terminal',
          groupId: group.groupId,
          surface,
          details: {
            expected: 1,
            observed: duplicateTerminal + 1,
            duplicate: duplicateTerminal,
            recordIds: observedRecords.filter((record) => record.terminal).map((record) => record.id).sort(),
          },
        });
      }
    }

    const requestIds = [...new Set(group.records.map((record) => record.causalIds.requestId).filter(Boolean))].sort();
    if (requestIds.length > 1) {
      addFinding(findings, {
        code: 'conflicting_projection',
        groupId: group.groupId,
        surface: 'request',
        details: { field: 'requestId', values: requestIds },
      });
      surfaceCounts.request.conflicting += 1;
    }

    for (const field of ['failure', 'provenance']) {
      const projections = group.records
        .filter((record) => record.terminal && record[field] !== null)
        .map((record) => ({ surface: record.surface, recordId: record.id, hash: stableValueHash(record[field]) }));
      const hashes = [...new Set(projections.map((entry) => entry.hash))];
      if (hashes.length > 1) {
        addFinding(findings, {
          code: 'conflicting_projection',
          groupId: group.groupId,
          details: { field, projections },
        });
        for (const surface of new Set(projections.map((entry) => entry.surface))) surfaceCounts[surface].conflicting += 1;
      }
    }

    const outcomeRecords = group.records.filter(
      (record) =>
        record.surface === 'event' ||
        record.surface === 'trace' ||
        (record.surface === 'job' && record.terminal),
    );
    const effects = [...new Set(outcomeRecords.map((record) => record.effectState).filter(Boolean))].sort();
    const retries = [...new Set(outcomeRecords.map((record) => record.retryDisposition).filter(Boolean))].sort();
    const unsafeRetry = outcomeRecords.some(
      (record) =>
        record.effectState === 'effect_uncertain' && record.retryDisposition === 'retry_same_request',
    );
    if (effects.length > 1 || retries.length > 1 || unsafeRetry) {
      addFinding(findings, {
        code: 'effect_retry_conflict',
        groupId: group.groupId,
        details: { effectStates: effects, retryDispositions: retries, unsafeRetry },
      });
    }

    const nullFailureRecords = outcomeRecords.filter((record) => record.failed && record.failure === null);
    if (nullFailureRecords.length > 0) {
      addFinding(findings, {
        code: 'null_failure',
        groupId: group.groupId,
        surface: nullFailureRecords[0].surface,
        recordId: nullFailureRecords[0].id,
        details: { recordIds: nullFailureRecords.map((record) => record.id).sort() },
      });
    }
    const nullProvenanceRecords = outcomeRecords.filter((record) => record.provenance === null);
    if (nullProvenanceRecords.length > 0) {
      addFinding(findings, {
        code: 'null_provenance',
        groupId: group.groupId,
        surface: nullProvenanceRecords[0].surface,
        recordId: nullProvenanceRecords[0].id,
        details: { recordIds: nullProvenanceRecords.map((record) => record.id).sort() },
      });
    }
    for (const record of group.records) {
      for (const parentId of record.parentIds) {
        const parents = idIndex.get(parentId) ?? [];
        if (parents.length === 0) {
          addFinding(findings, {
            code: 'broken_parent',
            groupId: group.groupId,
            surface: record.surface,
            recordId: record.id,
            details: { parentId },
          });
          continue;
        }
        for (const parent of parents) {
          if (record.timestamp !== null && parent.timestamp !== null && record.timestamp < parent.timestamp) {
            addFinding(findings, {
              code: 'timestamp_inversion',
              groupId: group.groupId,
              surface: record.surface,
              recordId: record.id,
              details: { parentId, parentTimestamp: parent.timestamp, childTimestamp: record.timestamp },
            });
          }
        }
      }
    }

    let previous = null;
    for (const surface of LOGGING_SURFACES) {
      const timestamps = relevantSurfaceRecords(group, surface)
        .map((record) => record.timestamp)
        .filter((value) => value !== null);
      if (timestamps.length === 0) continue;
      const earliest = Math.min(...timestamps);
      if (previous && earliest < previous.timestamp) {
        addFinding(findings, {
          code: 'timestamp_inversion',
          groupId: group.groupId,
          surface,
          details: { earlierSurface: previous.surface, earlierTimestamp: previous.timestamp, timestamp: earliest },
        });
      }
      previous = { surface, timestamp: earliest };
    }

    const expectedTransports = [
      ...new Set(
        specific.expectedOutcomeTransports ??
          request.expectedOutcomeTransports ??
          normalized.expectations.expectedOutcomeTransports ??
          [],
      ),
    ].sort();
    const outcomeTransports = [
      ...new Set(group.records.filter((record) => record.transport).map((record) => record.transport)),
    ].sort();
    if (
      (expectedTransports.length > 1 && outcomeTransports.length <= 1) ||
      (expectedTransports.length === 0 && group.records.filter((record) => record.terminal).length > 1 && outcomeTransports.length === 1)
    ) {
      addFinding(findings, {
        code: 'one_transport_only',
        groupId: group.groupId,
        details: { expectedTransports, observedTransports: outcomeTransports },
      });
    }
    group.findingCount = findings.length - groupFindingStart;
    group.expected = expected;
    group.outcomeTransports = outcomeTransports;
    group.operatorVisible = Boolean(
      specific.operatorVisible ?? specific.dashboardExpected ?? request.operatorVisible ?? false,
    );
    group.incidentExpected = Boolean(
      specific.incidentExpected ?? request.incidentExpected ?? request.expectedIncident ?? false,
    );
  }

  for (const artifact of normalized.artifacts) {
    const artifactGroupId =
      groups.find((group) =>
        group.records.some(
          (record) =>
            record.causalIds.requestId &&
            record.causalIds.requestId === (artifact.requestId ?? artifact.causalIds?.requestId),
        ),
      )?.groupId ?? groups[0]?.groupId;
    if (artifact.captureState === 'partial' || artifact.captureState === 'missing' || artifact.captureGap) {
      addFinding(findings, {
        code: 'capture_gap',
        groupId: artifactGroupId,
        recordId: artifact.artifactId ?? artifact.id ?? null,
        details: { captureState: artifact.captureState ?? null, captureGap: artifact.captureGap ?? null },
      });
    }
    if (artifact.captureState === 'redacted') {
      const receipts = normalized.redactionReceipts.filter(
        (receipt) => receipt.artifactId === (artifact.artifactId ?? artifact.id),
      );
      if (receipts.length === 0 && !(artifact.redactions?.length > 0)) {
        addFinding(findings, {
          code: 'capture_gap',
          groupId: artifactGroupId,
          recordId: artifact.artifactId ?? artifact.id ?? null,
          details: { captureState: 'redacted', captureGap: 'missing_redaction_receipt' },
        });
      }
    }
  }

  for (const record of normalized.records) {
    if (
      record.original.captureState === 'partial' ||
      record.original.captureState === 'missing' ||
      record.original.captureGap
    ) {
      addFinding(findings, {
        code: 'capture_gap',
        groupId: groups.find((group) => group.records.includes(record))?.groupId,
        surface: record.surface,
        recordId: record.id,
        details: {
          captureState: record.original.captureState ?? null,
          captureGap: record.original.captureGap ?? null,
        },
      });
    }
  }

  const forbiddenNames = new Set(
    [...DEFAULT_FORBIDDEN_FIELDS, ...normalized.forbiddenFields, ...(options.forbiddenFields ?? [])]
      .map(normalizeFieldName),
  );
  const canaryDefinitions = [
    ...normalized.canaryDefinitions,
    ...normalized.sensitiveCanaries.map((value) => ({
      canaryId: `canary:${stableValueHash(value).slice(0, 16)}`,
      value,
    })),
    ...(options.sensitiveCanaries ?? []).map((value) =>
      typeof value === 'string'
        ? { canaryId: `canary:${stableValueHash(value).slice(0, 16)}`, value }
        : value,
    ),
  ].filter((canary) => typeof canary?.value === 'string' && canary.value.length > 0);
  const canaries = [...new Map(canaryDefinitions.map((canary) => [canary.value, canary])).values()];
  const sensitiveSources = [
    ...normalized.records.map((record) => ({
      id: record.id,
      surface: record.surface,
      value: record.original,
      groupId: groups.find((group) => group.records.includes(record))?.groupId,
    })),
    ...normalized.artifacts.map((artifact, index) => ({
      id: artifact.artifactId ?? artifact.id ?? `artifact:${index}`,
      surface: 'artifact',
      value: artifact,
      groupId:
        groups.find((group) =>
          group.records.some(
            (record) =>
              record.causalIds.requestId &&
              record.causalIds.requestId === (artifact.requestId ?? artifact.causalIds?.requestId),
          ),
        )?.groupId ?? groups[0]?.groupId,
    })),
  ];
  for (const source of sensitiveSources) {
    for (const leak of scanSensitive(source.value, { forbiddenNames, canaries })) {
      addFinding(findings, {
        code: 'sensitive_value_leak',
        groupId: source.groupId,
        surface: source.surface,
        recordId: source.id,
        details: leak,
      });
    }
  }

  findings.sort(findingSort);
  const totals = Object.values(surfaceCounts).reduce(
    (sum, counts) => {
      for (const field of Object.keys(sum)) sum[field] += counts[field];
      return sum;
    },
    { expected: 0, observed: 0, missing: 0, duplicate: 0, conflicting: 0 },
  );
  const envelopeIdentities = new Map(
    groups.map((group) => {
      const requestId =
        [...new Set(group.records.map((record) => record.causalIds.requestId).filter(Boolean))].sort()[0] ??
        `missing-request:${stableValueHash(group.groupId).slice(0, 16)}`;
      return [
        group.groupId,
        {
          envelopeId:
            group.fixtureId ?? `p158-envelope:${stableValueHash(group.groupId).slice(0, 20)}`,
          requestId,
        },
      ];
    }),
  );
  const fallbackIdentity =
    envelopeIdentities.values().next().value ?? {
      envelopeId: `p158-envelope:${inputSha256.slice(0, 20)}`,
      requestId: `missing-request:${inputSha256.slice(0, 16)}`,
    };
  const severityFor = (code) => {
    if (code === 'capture_gap' || code === 'one_transport_only') return 'needs_evidence';
    return 'blocking';
  };
  const messageFor = (finding) => {
    const labels = {
      missing_record: 'A required causal-envelope record is missing.',
      duplicate_terminal: 'The causal envelope contains duplicate terminal records.',
      conflicting_projection: 'Structured outcome projections conflict across surfaces.',
      timestamp_inversion: 'A causal child or later surface predates its parent.',
      null_failure: 'A failed terminal outcome has no structured failure.',
      null_provenance: 'A terminal outcome has no immutable provenance.',
      one_transport_only: 'The terminal outcome is observable through only one transport.',
      broken_parent: 'A declared parent record cannot be resolved.',
      effect_retry_conflict: 'Effect state or retry disposition conflicts across terminal projections.',
      capture_gap: 'Evidence capture is partial, missing, or lacks its redaction receipt.',
      sensitive_value_leak: 'A forbidden field or synthetic sensitive canary is present in captured evidence.',
    };
    return labels[finding.code];
  };
  const finalizedFindings = findings.map(({ key: _key, ...finding }, index) => {
    const identity = envelopeIdentities.get(finding.groupId) ?? fallbackIdentity;
    const surfaceRoles = finding.surface && SCHEMA_SURFACE_ROLES[finding.surface]
      ? [SCHEMA_SURFACE_ROLES[finding.surface]]
      : [];
    const result = {
      findingId: `logging-finding-${String(index + 1).padStart(6, '0')}`,
      code: finding.code,
      severity: severityFor(finding.code),
      envelopeId: identity.envelopeId,
      requestId: identity.requestId,
      surfaceRoles,
      recordIds: finding.recordId ? [finding.recordId] : [],
      message: messageFor(finding),
      expected: finding.details.expected ?? finding.details.parentId ?? 'causal contract satisfied',
      observed: finding.details.observed ?? finding.details,
      repairAttempted: false,
    };
    if (finding.details.syntheticCanaryId) result.syntheticCanaryId = finding.details.syntheticCanaryId;
    return result;
  });
  const findingCounts = Object.fromEntries(
    LOGGING_FINDING_CODES.map((code) => [
      code,
      finalizedFindings.filter((finding) => finding.code === code).length,
    ]),
  );
  const envelopeReports = groups.map((group) => {
    const identity = envelopeIdentities.get(group.groupId);
    const groupFindings = finalizedFindings.filter(
      (finding) => finding.envelopeId === identity.envelopeId,
    );
    const codes = new Set(groupFindings.map((finding) => finding.code));
    let state = 'complete';
    if (codes.has('sensitive_value_leak')) state = 'leaking';
    else if (codes.has('conflicting_projection') || codes.has('effect_retry_conflict')) state = 'conflicting';
    else if (groupFindings.length > 0) state = 'incomplete';
    return {
      envelopeId: identity.envelopeId,
      fixtureId: group.fixtureId,
      requestId: identity.requestId,
      operatorVisible: group.operatorVisible,
      incidentExpected: group.incidentExpected,
      expectedSurfaceRoles: LOGGING_SURFACES
        .filter((surface) => group.expected[surface] > 0)
        .map((surface) => SCHEMA_SURFACE_ROLES[surface]),
      observedSurfaceRoles: LOGGING_SURFACES
        .filter((surface) => relevantSurfaceRecords(group, surface).length > 0)
        .map((surface) => SCHEMA_SURFACE_ROLES[surface]),
      sourceRecordCount: group.records.length,
      state,
      findingIds: groupFindings.map((finding) => finding.findingId),
    };
  });
  const auditedAt =
    options.auditedAt ??
    (normalized.records.some((record) => record.timestamp !== null)
      ? new Date(Math.max(...normalized.records.map((record) => record.timestamp ?? 0))).toISOString()
      : '1970-01-01T00:00:00.000Z');
  const report = {
    schemaVersion: 'agent-browser.p158-logging-audit-report.v1',
    planId: 'P158',
    auditId: options.auditId ?? `p158-audit:${inputSha256.slice(0, 24)}`,
    runId: options.runId ?? normalized.runId ?? `p158-run:${inputSha256.slice(0, 16)}`,
    inputSha256,
    auditedAt,
    repairAttempted: false,
    summary: {
      envelopeCount: envelopeReports.length,
      completeEnvelopeCount: envelopeReports.filter((envelope) => envelope.state === 'complete').length,
      incompleteEnvelopeCount: envelopeReports.filter((envelope) => envelope.state !== 'complete').length,
      expectedRecordCount: totals.expected,
      observedRecordCount: totals.observed,
      missingRecordCount: totals.missing,
      duplicateTerminalCount: findingCounts.duplicate_terminal,
      conflictingProjectionCount: findingCounts.conflicting_projection,
      timestampInversionCount: findingCounts.timestamp_inversion,
      nullFailureCount: findingCounts.null_failure,
      nullProvenanceCount: findingCounts.null_provenance,
      oneTransportOnlyCount: findingCounts.one_transport_only,
      brokenParentCount: findingCounts.broken_parent,
      effectRetryConflictCount: findingCounts.effect_retry_conflict,
      sensitiveValueLeakCount: findingCounts.sensitive_value_leak,
      captureGapCount: findingCounts.capture_gap,
    },
    envelopes: envelopeReports,
    findings: finalizedFindings,
  };
  return clone(report);
}

export function auditCausalEnvelopes({ fixtureSet, options = {} }) {
  if (!fixtureSet || typeof fixtureSet !== 'object') throw new TypeError('fixtureSet is required');
  return auditLoggingCompleteness(fixtureSet, options);
}
