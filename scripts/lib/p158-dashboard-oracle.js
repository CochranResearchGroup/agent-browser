import { createHash } from 'node:crypto';

import { classifyOperatorUrl } from './p158-external-handoff-oracle.js';

const ABORTED_REQUEST_FAILURE_SHA256 =
  createHash('sha256').update('net::ERR_ABORTED').digest('hex');
const GUACAMOLE_ACTIVE_SHARING_PROFILES =
  /\/guacamole\/api\/session\/tunnels\/[^/]+\/activeConnection\/connection\/sharingProfiles$/i;
const GUACAMOLE_TUNNEL_PROTOCOL =
  /\/guacamole\/api\/session\/tunnels\/[^/]+\/protocol$/i;

export const DASHBOARD_FINDING_CODES = Object.freeze([
  'missing_rail_row',
  'duplicate_rail_row',
  'stale_rail_row',
  'wrong_rail_row',
  'unstable_same_label_identity',
  'rail_order_mismatch',
  'rail_label_mismatch',
  'rail_badge_mismatch',
  'rail_count_mismatch',
  'selected_record_mismatch',
  'inspector_record_mismatch',
  'selection_recovery_invalid',
  'deep_link_recovery_invalid',
  'multi_client_selection_leakage',
  'wrong_action_target',
  'ineligible_action_exposed',
  'action_eligibility_mismatch',
  'warning_axis_conflated',
  'warning_axis_mismatch',
  'convergence_action_count_invalid',
  'internal_handoff_url',
  'stale_stream_ready',
  'stream_not_converged',
  'accessibility_defect',
  'focus_defect',
  'keyboard_defect',
  'native_modal_used',
  'overflow_defect',
  'reduced_motion_defect',
  'viewport_defect',
  'console_error',
  'network_failure',
  'latency_budget_exceeded',
  'heap_growth_exceeded',
  'dom_growth_exceeded',
  'listener_growth_exceeded',
  'cpu_growth_exceeded',
  'network_growth_exceeded',
  'long_task_growth_exceeded',
  'browser_process_growth_exceeded',
  'xvfb_process_growth_exceeded',
  'route_growth_exceeded',
  'profile_lease_growth_exceeded',
  'retained_session_growth_exceeded',
  'unresolved_job_growth_exceeded',
  'capture_gap',
]);

const UI_FINDING = Object.freeze({
  accessibility: 'accessibility_defect',
  focus: 'focus_defect',
  keyboard: 'keyboard_defect',
  modal: 'native_modal_used',
  overflow: 'overflow_defect',
  reduced_motion: 'reduced_motion_defect',
  viewport: 'viewport_defect',
});

const RESOURCE_FIELDS = Object.freeze({
  heapBytes: ['heapBytesPerMinute', 'heap_growth_exceeded'],
  domNodeCount: ['domNodesPerMinute', 'dom_growth_exceeded'],
  listenerCount: ['listenersPerMinute', 'listener_growth_exceeded'],
  cpuMilliseconds: ['cpuMillisecondsPerMinute', 'cpu_growth_exceeded'],
  networkBytes: ['networkBytesPerMinute', 'network_growth_exceeded'],
  longTaskCount: ['longTasksPerMinute', 'long_task_growth_exceeded'],
  browserProcessCount: ['browserProcessesPerMinute', 'browser_process_growth_exceeded'],
  xvfbProcessCount: ['xvfbProcessesPerMinute', 'xvfb_process_growth_exceeded'],
  routeAllocationCount: ['routeAllocationsPerMinute', 'route_growth_exceeded'],
  profileLeaseCount: ['profileLeasesPerMinute', 'profile_lease_growth_exceeded'],
  retainedSessionCount: ['retainedSessionsPerMinute', 'retained_session_growth_exceeded'],
  unresolvedJobCount: ['unresolvedJobsPerMinute', 'unresolved_job_growth_exceeded'],
});

const DEFAULT_SLOPE_BUDGETS = Object.freeze(
  {
    heapBytesPerMinute: 1_000_000,
    domNodesPerMinute: 100,
    listenersPerMinute: 10,
    cpuMillisecondsPerMinute: 10_000,
    networkBytesPerMinute: 1_000_000,
    longTasksPerMinute: 10,
    browserProcessesPerMinute: 0,
    xvfbProcessesPerMinute: 0,
    routeAllocationsPerMinute: 0,
    profileLeasesPerMinute: 0,
    retainedSessionsPerMinute: 0,
    unresolvedJobsPerMinute: 0,
  },
);

function canonicalize(value, seen = new Set()) {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return value;
  if (typeof value === 'number') return Number.isFinite(value) ? value : String(value);
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { $bytes: Buffer.from(value).toString('base64') };
  if (typeof value !== 'object') return String(value);
  if (seen.has(value)) return '[circular]';
  seen.add(value);
  const result = Array.isArray(value)
    ? value.map((entry) => canonicalize(entry, seen))
    : Object.fromEntries(
        Object.keys(value)
          .sort()
          .filter((key) => value[key] !== undefined)
          .map((key) => [key, canonicalize(value[key], seen)]),
      );
  seen.delete(value);
  return result;
}

export function stableDashboardHash(value) {
  return createHash('sha256').update(JSON.stringify(canonicalize(value))).digest('hex');
}

function clone(value) {
  return structuredClone(value);
}

function same(left, right) {
  return stableDashboardHash(left) === stableDashboardHash(right);
}

function finite(value, fallback = 0) {
  const number = Number(value);
  return Number.isFinite(number) ? number : fallback;
}

function nearestRank(sorted, percentile) {
  if (sorted.length === 0) return 0;
  return sorted[Math.max(0, Math.ceil(percentile * sorted.length) - 1)];
}

export function calculateTimingDistribution(samples, p95BudgetMs = Infinity) {
  const sorted = samples.map(Number).filter(Number.isFinite).sort((a, b) => a - b);
  const p50Ms = nearestRank(sorted, 0.5);
  const p95Ms = nearestRank(sorted, 0.95);
  const p99Ms = nearestRank(sorted, 0.99);
  const worstMs = sorted.at(-1) ?? 0;
  return {
    p50Ms,
    p95Ms,
    p99Ms,
    worstMs,
    p95BudgetMs,
    budgetMiss: p95Ms > p95BudgetMs,
  };
}

function linearSlopePerMinute(samples, field) {
  const points = samples
    .map((sample) => [finite(sample.elapsedMs, NaN) / 60_000, finite(sample[field], NaN)])
    .filter(([x, y]) => Number.isFinite(x) && Number.isFinite(y));
  if (points.length < 2) return 0;
  const meanX = points.reduce((sum, point) => sum + point[0], 0) / points.length;
  const meanY = points.reduce((sum, point) => sum + point[1], 0) / points.length;
  const denominator = points.reduce((sum, point) => sum + (point[0] - meanX) ** 2, 0);
  if (denominator === 0) return 0;
  const numerator = points.reduce(
    (sum, point) => sum + (point[0] - meanX) * (point[1] - meanY),
    0,
  );
  const slope = numerator / denominator;
  return Object.is(slope, -0) ? 0 : slope;
}

export function calculateResourceSlopes(samples) {
  return Object.fromEntries(
    Object.entries(RESOURCE_FIELDS).map(([field, [outputField]]) => [
      outputField,
      linearSlopePerMinute(samples, field),
    ]),
  );
}

function addFinding(findings, finding) {
  const normalized = {
    code: finding.code,
    severity: finding.severity ?? 'blocking',
    field: finding.field ?? null,
    recordIds: [...new Set((finding.recordIds ?? []).filter(Boolean).map(String))].sort(),
    message: finding.message,
    expected: clone(finding.expected ?? null),
    observed: clone(finding.observed ?? null),
  };
  const identity = stableDashboardHash(normalized);
  if (!findings.some((entry) => entry.identity === identity)) findings.push({ identity, ...normalized });
}

function healthAxes(warnings) {
  const mapping = [
    ['runtimeHealth', 'runtime'],
    ['convergenceHealth', 'convergence'],
    ['accessHealth', 'access'],
    ['acquisitionHealth', 'acquisition'],
  ];
  return mapping.filter(([field]) => warnings[field] !== 'healthy').map(([, axis]) => axis).sort();
}

function resourceOrder(resource, index) {
  return resource.orderKey ?? resource.order ?? resource.orderIndex ?? resource.sortIndex ?? resource.sortKey ?? index;
}

function rowProjection(record) {
  return {
    resourceId: record.resourceId,
    resourceType: record.resourceType,
    label: record.label,
    state: record.state,
    badge: record.badge ?? null,
    count: record.count ?? 0,
  };
}

function timingDistributions(fixture) {
  return (fixture.timings ?? []).map((timing) => {
    const calculated = Array.isArray(timing.samplesMs)
      ? calculateTimingDistribution(timing.samplesMs, timing.p95BudgetMs)
      : {
          p50Ms: finite(timing.p50Ms),
          p95Ms: finite(timing.p95Ms),
          p99Ms: finite(timing.p99Ms),
          worstMs: finite(timing.worstMs),
          p95BudgetMs: finite(timing.p95BudgetMs),
          budgetMiss: finite(timing.p95Ms) > finite(timing.p95BudgetMs),
        };
    return { interaction: timing.interaction, ...calculated };
  });
}

function auditRail(fixture, findings) {
  const truth = fixture.truth ?? {};
  const resources = truth.resources ?? [];
  const expected = resources.filter((resource) => resource.rowExpected);
  const rows = fixture.railRows ?? [];
  const truthById = new Map(resources.map((resource) => [resource.resourceId, resource]));
  const expectedById = new Map(expected.map((resource) => [resource.resourceId, resource]));
  const rowsByResource = new Map();
  for (const row of rows) {
    const entries = rowsByResource.get(row.resourceId) ?? [];
    entries.push(row);
    rowsByResource.set(row.resourceId, entries);
  }

  for (const resource of expected) {
    const matches = rowsByResource.get(resource.resourceId) ?? [];
    if (matches.length === 0) {
      addFinding(findings, {
        code: 'missing_rail_row', field: 'railRows', recordIds: [resource.resourceId],
        message: 'An actionable authoritative resource has no rendered rail row.', expected: 1, observed: 0,
      });
    }
    if (matches.length > 1) {
      addFinding(findings, {
        code: 'duplicate_rail_row', field: 'railRows', recordIds: matches.map((row) => row.rowId),
        message: 'An authoritative resource has more than one rendered rail row.', expected: 1, observed: matches.length,
      });
    }
  }

  for (const row of rows) {
    const resource = truthById.get(row.resourceId);
    if (!resource || !expectedById.has(row.resourceId)) {
      addFinding(findings, {
        code: 'wrong_rail_row', field: 'railRows', recordIds: [row.rowId],
        message: 'A rendered rail row does not map to one current actionable resource.',
        expected: null, observed: rowProjection(row),
      });
      continue;
    }
    if (row.snapshotRevision !== truth.snapshotRevision) {
      addFinding(findings, {
        code: 'stale_rail_row', field: 'snapshotRevision', recordIds: [row.rowId],
        message: 'A rail row was rendered from a different snapshot barrier.',
        expected: truth.snapshotRevision, observed: row.snapshotRevision,
      });
    }
    if (row.resourceType !== resource.resourceType || row.state !== resource.state) {
      addFinding(findings, {
        code: 'wrong_rail_row', field: 'railRows', recordIds: [row.rowId, row.resourceId],
        message: 'A rail row identifies the right resource ID but projects the wrong type or state.',
        expected: { resourceType: resource.resourceType, state: resource.state },
        observed: { resourceType: row.resourceType, state: row.state },
      });
    }
    if (row.label !== resource.label) {
      addFinding(findings, {
        code: 'rail_label_mismatch', field: 'label', recordIds: [row.rowId, row.resourceId],
        message: 'A rendered rail label differs from the authoritative label.', expected: resource.label, observed: row.label,
      });
    }
    if (!same(row.badge ?? null, resource.badge ?? null)) {
      addFinding(findings, {
        code: 'rail_badge_mismatch', field: 'badges', recordIds: [row.rowId, row.resourceId],
        message: 'Rendered rail badge differs from the authoritative badge.', expected: resource.badge ?? null, observed: row.badge ?? null,
      });
    }
    if (!same(row.count ?? 0, resource.count ?? 0)) {
      addFinding(findings, {
        code: 'rail_count_mismatch', field: 'counts', recordIds: [row.rowId, row.resourceId],
        message: 'Rendered rail count differs from the authoritative count.', expected: resource.count ?? 0, observed: row.count ?? 0,
      });
    }
    const expectedRowId = resource.rowId ?? resource.stableRowId ?? `row-${resource.resourceId}`;
    if ((rowsByResource.get(row.resourceId) ?? []).length === 1 && expectedRowId && row.rowId !== expectedRowId) {
      addFinding(findings, {
        code: 'unstable_same_label_identity', field: 'rowId', recordIds: [row.rowId, row.resourceId],
        message: 'A rail row did not retain its stable identity.', expected: expectedRowId, observed: row.rowId,
      });
    }
  }

  const expectedOrder = expected
    .map((resource, index) => ({ resourceId: resource.resourceId, order: resourceOrder(resource, index) }))
    .sort((a, b) => String(a.order).localeCompare(String(b.order), undefined, { numeric: true }))
    .map((entry) => entry.resourceId);
  const observedOrder = rows.filter((row) => expectedById.has(row.resourceId)).map((row) => row.resourceId);
  const exactBijection =
    rows.length === expected.length &&
    expected.every((resource) => (rowsByResource.get(resource.resourceId) ?? []).length === 1);
  if (exactBijection && !same(observedOrder, expectedOrder)) {
    addFinding(findings, {
      code: 'rail_order_mismatch', field: 'railRows', recordIds: rows.map((row) => row.rowId),
      message: 'Rendered rail order differs from the authoritative order.', expected: expectedOrder, observed: observedOrder,
    });
  }
  return { expected, rows };
}

function auditSelectionAndActions(fixture, findings) {
  const selection = fixture.selection ?? {};
  const resourceIds = new Set((fixture.truth?.resources ?? []).map((resource) => resource.resourceId));
  const renderedSelected = selection.selectedResourceId;
  const selectedActuallyExists = renderedSelected !== null && resourceIds.has(renderedSelected);
  if (selection.selectedExists !== selectedActuallyExists) {
    addFinding(findings, {
      code: 'selected_record_mismatch', field: 'selection.selectedResourceId',
      recordIds: [renderedSelected], message: 'Rendered selection existence differs from the authoritative snapshot.',
      expected: selectedActuallyExists, observed: selection.selectedExists,
    });
  }
  const inspector = selection.inspectorResourceId;
  if (inspector !== renderedSelected || (selection.inspectorRecord && selection.expectedInspectorRecord && !same(selection.inspectorRecord, selection.expectedInspectorRecord))) {
    addFinding(findings, {
      code: 'inspector_record_mismatch', field: 'selection.inspectorResourceId',
      recordIds: [inspector, renderedSelected], message: 'Inspector readback does not describe the selected resource.',
      expected: selection.expectedInspectorRecord ?? renderedSelected ?? null,
      observed: selection.inspectorRecord ?? inspector ?? null,
    });
  }
  if (selection.recoveryActionCount !== 1) {
    addFinding(findings, {
      code: 'selection_recovery_invalid', field: 'selection.recoveryActionCount', recordIds: [renderedSelected],
      message: 'Selection recovery did not yield exactly one deterministic action.',
      expected: 1, observed: selection.recoveryActionCount,
    });
  }
  if (selection.deepLinkRequestedId !== null) {
    const requestedExists = resourceIds.has(selection.deepLinkRequestedId);
    const resolvedExists = selection.deepLinkResolvedId !== null && resourceIds.has(selection.deepLinkResolvedId);
    const exactCurrent = requestedExists && selection.deepLinkResolvedId === selection.deepLinkRequestedId;
    if (!resolvedExists || (requestedExists && !exactCurrent)) {
      addFinding(findings, {
        code: 'deep_link_recovery_invalid', field: 'selection.deepLinkResolvedId',
        recordIds: [selection.deepLinkRequestedId, selection.deepLinkResolvedId],
        message: 'Deep-link recovery did not resolve to the requested current resource or a valid current fallback.',
        expected: requestedExists ? selection.deepLinkRequestedId : 'a current resource', observed: selection.deepLinkResolvedId,
      });
    }
  }
  for (const client of fixture.clientSelections ?? []) {
    const selectionLeak = client.observedSelectedResourceId !== client.expectedSelectedResourceId;
    const inspectorLeak = client.observedInspectorResourceId !== client.expectedInspectorResourceId;
    if (selectionLeak || inspectorLeak) {
      addFinding(findings, {
        code: 'multi_client_selection_leakage', field: 'clientSelections',
        recordIds: [client.clientId, client.expectedSelectedResourceId, client.observedSelectedResourceId],
        message: 'A dashboard client observed another client\'s selection or inspector state.',
        expected: {
          selectedResourceId: client.expectedSelectedResourceId,
          inspectorResourceId: client.expectedInspectorResourceId,
        },
        observed: {
          selectedResourceId: client.observedSelectedResourceId,
          inspectorResourceId: client.observedInspectorResourceId,
        },
      });
    }
  }

  for (const action of fixture.actions ?? []) {
    if (action.rendered && !action.eligible) {
      addFinding(findings, {
        code: 'ineligible_action_exposed', field: 'actions.eligible', recordIds: [action.actionId],
        message: 'An ineligible action is exposed as executable.', expected: false, observed: true,
      });
    }
    if (action.eligible && (!action.rendered || !action.displayedEligible)) {
      addFinding(findings, {
        code: 'action_eligibility_mismatch', field: 'actions.rendered', recordIds: [action.actionId],
        message: 'An eligible authoritative action is absent or displayed ineligible.',
        expected: { rendered: true, displayedEligible: true },
        observed: { rendered: action.rendered, displayedEligible: action.displayedEligible },
      });
    }
    if (action.invokedTargetResourceId !== null && action.invokedTargetResourceId !== action.declaredTargetResourceId) {
      addFinding(findings, {
        code: 'wrong_action_target', field: 'actions.invokedTargetResourceId', recordIds: [action.actionId],
        message: 'A dashboard action invoked a resource other than its declared target.',
        expected: action.declaredTargetResourceId, observed: action.invokedTargetResourceId,
      });
    }
  }
}

function auditWarnings(fixture, findings) {
  const warnings = fixture.warnings ?? {};
  const expectedAxes = warnings.expectedAxes ?? healthAxes(warnings);
  const observedAxes = [...(warnings.displayedAxes ?? [])].sort();
  if (!same(expectedAxes, observedAxes)) {
    const conflated = observedAxes.some((axis) => !expectedAxes.includes(axis));
    addFinding(findings, {
      code: conflated ? 'warning_axis_conflated' : 'warning_axis_mismatch', field: 'warnings.displayedAxes',
      recordIds: warnings.warningIds ?? [], message: 'Displayed warning axes do not match independent health axes.',
      expected: expectedAxes, observed: observedAxes,
    });
  }
  const convergenceFailed = warnings.convergenceHealth === 'failed';
  const actionIds = warnings.convergenceActionIds ?? [];
  const eligibleIds = new Set(
    (fixture.actions ?? []).filter((action) => action.rendered && action.eligible).map((action) => action.actionId),
  );
  const executable = actionIds.filter((id) => eligibleIds.has(id));
  const valid = convergenceFailed ? actionIds.length === 1 && executable.length === 1 : actionIds.length === 0;
  if (!valid) {
    addFinding(findings, {
      code: 'convergence_action_count_invalid', field: 'warnings.convergenceActionIds', recordIds: actionIds,
      message: 'Runtime convergence warning does not expose exactly one executable convergence action.',
      expected: convergenceFailed ? 1 : 0, observed: { actionCount: actionIds.length, executableCount: executable.length },
    });
  }
}

function auditHandoffUrls(fixture, findings) {
  for (const [index, observation] of (fixture.handoffUrls ?? []).entries()) {
    const url = typeof observation === 'string' ? observation : observation.url;
    const role = typeof observation === 'string' ? 'copied_action' : observation.role ?? 'copied_action';
    const classification = classifyOperatorUrl(url, {
      role: typeof observation === 'string' ? 'starting_handoff' : role,
      baseUrl: observation?.baseUrl,
      resolvedAddresses: observation?.resolvedAddresses ?? [],
    });
    if (!classification.valid) {
      addFinding(findings, {
        code: 'internal_handoff_url', field: `handoffUrls.${index}`, recordIds: [observation?.observationId],
        message: 'A dashboard copy or open action exposed a non-durable or internal URL.',
        expected: { valid: true, role: 'copied_action' }, observed: classification,
      });
    }
  }
}

function exactRecoveryIds(entry) {
  const recoveryIds = entry.classification?.recoveryEvidenceEntryIds;
  if (!Array.isArray(recoveryIds) || recoveryIds.length === 0 ||
      new Set(recoveryIds).size !== recoveryIds.length) {
    return null;
  }
  return recoveryIds.every((recoveryId) => typeof recoveryId === 'string' && recoveryId.length > 0)
    ? recoveryIds
    : null;
}

function hasLaterSuccessfulNetworkRecovery(entry, networkEntries) {
  const recoveryIds = exactRecoveryIds(entry);
  const failedAt = Date.parse(entry.completedAt ?? entry.startedAt ?? '');
  if (!recoveryIds || !Number.isFinite(failedAt) ||
      !/^[a-f0-9]{64}$/.test(String(entry.urlSha256 ?? '')) ||
      typeof entry.method !== 'string' || entry.method.length === 0) {
    return false;
  }
  return recoveryIds.every((recoveryId) => {
    const recovery = networkEntries.find((candidate) => candidate.entryId === recoveryId);
    const recoveredAt = Date.parse(recovery?.startedAt ?? recovery?.completedAt ?? '');
    return recovery?.urlSha256 === entry.urlSha256 && recovery?.method === entry.method &&
      Number.isFinite(recoveredAt) && recoveredAt > failedAt &&
      Number.isInteger(recovery.status) && recovery.status >= 200 && recovery.status < 400 &&
      recovery.error === null;
  });
}

function hasExactExpectedLifecycleRecovery(entry, networkEntries) {
  const recoveryIds = exactRecoveryIds(entry);
  return recoveryIds !== null && recoveryIds.every((recoveryId) => {
    const recovery = networkEntries.find((candidate) => candidate.entryId === recoveryId);
    return recovery !== undefined && isExpectedLifecycleNoise(recovery, 'network', networkEntries);
  });
}

function isExpectedLifecycleNoise(entry, evidenceType, networkEntries = []) {
  if (entry.classification?.disposition !== 'expected_lifecycle_noise') return false;
  if (evidenceType === 'console') {
    return entry.classification.code ===
        'console_resource_failure_matches_expected_network_lifecycle' &&
      entry.messageClass === 'resource_load_failed' &&
      entry.locationPathClass === 'guacamole_transport' &&
      hasExactExpectedLifecycleRecovery(entry, networkEntries);
  }
  if (evidenceType !== 'network') return false;
  if (entry.classification.code === 'guacamole_active_connection_observation_absent') {
    return entry.status === 404 &&
      GUACAMOLE_ACTIVE_SHARING_PROFILES.test(String(entry.url ?? ''));
  }
  if (entry.classification.code === 'guacamole_token_refresh_recovered') {
    return entry.status === 403 &&
      /\/guacamole\/api\/tokens$/i.test(String(entry.url ?? '')) &&
      entry.classification.recoveryEvidenceEntryIds?.length > 0;
  }
  if (entry.classification.code === 'page_or_reconnect_request_cancelled') {
    return entry.status === null && typeof entry.error === 'string' &&
      (/\/guacamole\/tunnel$/i.test(String(entry.url ?? '')) ||
        entry.pathClass === 'dashboard_auth_status' ||
        ((GUACAMOLE_ACTIVE_SHARING_PROFILES.test(String(entry.url ?? '')) ||
          GUACAMOLE_TUNNEL_PROTOCOL.test(String(entry.url ?? ''))) &&
          entry.error === `request-failure-sha256:${ABORTED_REQUEST_FAILURE_SHA256}`) ||
        (/\/guacamole\/api\/tokens$/i.test(String(entry.url ?? '')) &&
          entry.classification.recoveryEvidenceEntryIds?.length > 0) ||
        (entry.pathClass === 'guacamole_transport' &&
          hasLaterSuccessfulNetworkRecovery(entry, networkEntries)));
  }
  return false;
}

function auditStreamAndUi(fixture, findings) {
  const stream = fixture.stream ?? {};
  const stale = stream.streamRevision < stream.snapshotRevision || stream.authoritativeReady !== true;
  if (stale && stream.displayedReady) {
    addFinding(findings, {
      code: 'stale_stream_ready', field: 'stream.displayedReady', recordIds: [stream.streamId],
      message: 'The dashboard displayed ready from stale or non-ready stream evidence.', expected: false, observed: true,
    });
  }
  if (stream.streamRevision !== stream.snapshotRevision) {
    addFinding(findings, {
      code: 'stream_not_converged', field: 'stream.streamRevision', recordIds: [stream.streamId],
      message: 'Dashboard stream revision did not converge to its snapshot barrier.',
      expected: stream.snapshotRevision, observed: stream.streamRevision,
    });
  }
  for (const check of fixture.uiChecks ?? []) {
    if (check.state === 'failed') {
      addFinding(findings, {
        code: UI_FINDING[check.kind] ?? 'capture_gap', field: `uiChecks.${check.kind}`, recordIds: [check.checkId],
        message: `Dashboard ${check.kind} evidence failed.`, expected: 'passed', observed: check.detail ?? 'failed',
      });
    }
  }
  for (const entry of fixture.consoleEntries ?? []) {
    const expectedLifecycleNoise = isExpectedLifecycleNoise(
      entry,
      'console',
      fixture.networkEntries ?? [],
    );
    if (!expectedLifecycleNoise && ['error', 'exception'].includes(String(entry.level).toLowerCase())) {
      addFinding(findings, {
        code: 'console_error', field: 'consoleEntries', recordIds: [entry.entryId],
        message: 'Dashboard console capture contains an error.', expected: 'no errors', observed: entry.message ?? entry.level,
      });
    }
  }
  for (const entry of fixture.networkEntries ?? []) {
    const expectedLifecycleNoise = isExpectedLifecycleNoise(
      entry,
      'network',
      fixture.networkEntries ?? [],
    );
    if (!expectedLifecycleNoise &&
        (entry.success === false || entry.status === null || finite(entry.status, 200) >= 400)) {
      addFinding(findings, {
        code: 'network_failure', field: 'networkEntries', recordIds: [entry.entryId],
        message: 'Dashboard network capture contains a failed request.', expected: 'successful request', observed: entry.status ?? entry.error ?? false,
      });
    }
  }
}

function auditPerformance(fixture, findings, options) {
  const timings = timingDistributions(fixture);
  for (const timing of timings) {
    if (timing.budgetMiss) {
      addFinding(findings, {
        code: 'latency_budget_exceeded', field: `timings.${timing.interaction}`, recordIds: [timing.interaction],
        message: 'A dashboard interaction exceeded its frozen p95 latency budget.',
        expected: { p95MsAtMost: timing.p95BudgetMs }, observed: timing,
      });
    }
  }
  const samples = fixture.resourceSamples ?? [];
  const slopes = calculateResourceSlopes(samples);
  const budgets = { ...DEFAULT_SLOPE_BUDGETS, ...(fixture.resourceSlopeBudgets ?? {}), ...(options.resourceSlopeBudgets ?? {}) };
  if (samples.length >= 2) {
    for (const [inputField, [outputField, code]] of Object.entries(RESOURCE_FIELDS)) {
      if (!samples.some((sample) => sample[inputField] !== undefined)) continue;
      const slope = slopes[outputField];
      const first = samples[0]?.[inputField];
      const last = samples.at(-1)?.[inputField];
      const budget = finite(budgets[outputField], 0);
      if (slope > budget && finite(last) > finite(first)) {
        addFinding(findings, {
          code, field: `resourceSamples.${inputField}`, recordIds: [],
          message: `Dashboard ${inputField} has an unbounded upward trend beyond its frozen slope budget.`,
          expected: { slopePerMinuteAtMost: budget }, observed: { slopePerMinute: slope, first, last },
        });
      }
    }
  }
  return { timings, slopes };
}

function finalizeFindings(findings, fixtureId) {
  return findings
    .sort((a, b) => a.code.localeCompare(b.code) || a.identity.localeCompare(b.identity))
    .map(({ identity, ...finding }, index) => ({
      findingId: `dashboard-finding-${String(index + 1).padStart(4, '0')}-${identity.slice(0, 12)}`,
      ...finding,
      repairAttempted: false,
    }));
}

export function auditDashboardFixture({ fixture, options = {} }) {
  const input = clone(fixture);
  const findings = [];
  const { expected, rows } = auditRail(input, findings);
  auditSelectionAndActions(input, findings);
  auditWarnings(input, findings);
  auditHandoffUrls(input, findings);
  auditStreamAndUi(input, findings);
  const { timings, slopes } = auditPerformance(input, findings, options);
  for (const gap of input.captureGaps ?? []) {
    addFinding(findings, {
      code: 'capture_gap', severity: 'needs_evidence', field: gap.evidenceClass, recordIds: [gap.gapId],
      message: 'Required dashboard evidence was not captured.', expected: 'complete capture', observed: gap.detail,
    });
  }
  const finalized = finalizeFindings(findings, input.fixtureId);
  const findingCounts = Object.fromEntries(
    DASHBOARD_FINDING_CODES.map((code) => [code, finalized.filter((finding) => finding.code === code).length]),
  );
  const count = (code) => findingCounts[code] ?? 0;
  const report = {
    schemaVersion: 'agent-browser.p158-dashboard-oracle-report.v1',
    planId: 'P158',
    auditId: `dashboard-audit-${stableDashboardHash({ fixture: input, options }).slice(0, 20)}`,
    fixtureId: input.fixtureId,
    inputSha256: stableDashboardHash(input),
    auditedAt: options.auditedAt ?? input.auditedAt ?? '1970-01-01T00:00:00.000Z',
    repairAttempted: false,
    passed: finalized.length === 0,
    summary: {
      expectedRailRowCount: expected.length,
      observedRailRowCount: rows.length,
      missingRailRowCount: count('missing_rail_row'),
      duplicateRailRowCount: count('duplicate_rail_row'),
      staleRailRowCount: count('stale_rail_row'),
      wrongRailRowCount: count('wrong_rail_row'),
      findingCount: finalized.length,
      findingCounts,
    },
    timingDistributions: timings,
    resourceSlopes: slopes,
    handoffUrlHygienePassed: count('internal_handoff_url') === 0,
    findings: finalized,
  };
  return report;
}

function setFixtureIdentity(fixture, descriptor) {
  fixture.fixtureId = descriptor.fixtureId;
  fixture.description = descriptor.description;
  fixture.density = descriptor.density;
  fixture.expectedFindingCodes = clone(descriptor.expectedFindingCodes);
  return fixture;
}

function mutateResourceSlope(fixture, field) {
  const samples = fixture.resourceSamples;
  const first = finite(samples[0][field]);
  const outputField = RESOURCE_FIELDS[field][0];
  const budget = finite(fixture.resourceSlopeBudgets?.[outputField] ?? DEFAULT_SLOPE_BUDGETS[outputField]);
  samples.at(-1)[field] = Math.ceil(first + Math.max(1, budget + 1) * 2);
}

export function materializeDashboardFixture({ baseline, descriptor, caseSpec }) {
  const selectedCase = descriptor ?? caseSpec;
  const mutation = selectedCase.mutation;
  if (mutation.kind === 'inventory_density') {
    return setFixtureIdentity(generateDenseDashboardFixture(mutation.parameters), selectedCase);
  }
  const fixture = setFixtureIdentity(clone(baseline), selectedCase);
  const targetId = mutation.parameters.targetId ?? fixture.selection.selectedResourceId;
  const row = fixture.railRows.find((entry) => entry.resourceId === targetId) ?? fixture.railRows[0];
  const resource = fixture.truth.resources.find((entry) => entry.resourceId === targetId) ?? fixture.truth.resources[0];
  const action = fixture.actions[0];
  const kind = mutation.kind;

  if (kind === 'typed_convergence_warning') {
    fixture.warnings.convergenceHealth = 'failed';
    fixture.warnings.displayedAxes = ['convergence'];
    fixture.warnings.convergenceActionIds = ['refresh-status'];
    fixture.actions.push({
      actionId: 'refresh-status', declaredTargetResourceId: targetId,
      invokedTargetResourceId: null, eligible: true, displayedEligible: true, rendered: true,
    });
    return fixture;
  }
  if (kind === 'missing_rail_row') fixture.railRows = fixture.railRows.filter((entry) => entry.resourceId !== targetId);
  if (kind === 'duplicate_rail_row') fixture.railRows.push({ ...clone(row), rowId: `${row.rowId}-duplicate` });
  if (kind === 'stale_rail_row') row.snapshotRevision = fixture.truth.snapshotRevision - 1;
  if (kind === 'wrong_rail_row') row.state = `${resource.state}-wrong`;
  if (kind === 'rail_order_mismatch') {
    const second = { ...clone(resource), resourceId: `${targetId}-second`, orderKey: resource.orderKey + 1 };
    fixture.truth.resources.push(second);
    fixture.truth.counts.browsers += second.resourceType === 'browser' ? 1 : 0;
    fixture.railRows.unshift({ ...clone(row), rowId: `row-${second.resourceId}`, resourceId: second.resourceId, orderKey: second.orderKey });
  }
  if (kind === 'rail_label_mismatch') row.label = `${resource.label} wrong`;
  if (kind === 'rail_badge_mismatch') row.badge = row.badge === null ? 'wrong' : `${row.badge}-wrong`;
  if (kind === 'rail_count_mismatch') row.count += 1;
  if (kind === 'unstable_same_label_identity') {
    const sameLabelId = `${targetId}-same-label`;
    const second = { ...clone(resource), resourceId: sameLabelId, orderKey: resource.orderKey + 1 };
    fixture.truth.resources.push(second);
    fixture.truth.counts.browsers += second.resourceType === 'browser' ? 1 : 0;
    fixture.railRows.push({ ...clone(row), rowId: `row-${sameLabelId}`, resourceId: sameLabelId, orderKey: second.orderKey });
    row.rowId = `${row.rowId}-unstable`;
  }
  if (kind === 'selected_record_mismatch') fixture.selection.selectedExists = !fixture.selection.selectedExists;
  if (kind === 'inspector_record_mismatch') fixture.selection.inspectorResourceId = `${targetId}-wrong`;
  if (kind === 'multi_client_selection_leakage') {
    const parameters = mutation.parameters;
    const secondResourceId = parameters.expectedClientBResourceId;
    if (!fixture.truth.resources.some((entry) => entry.resourceId === secondResourceId)) {
      const second = { ...clone(resource), resourceId: secondResourceId, orderKey: resource.orderKey + 1 };
      fixture.truth.resources.push(second);
      fixture.railRows.push({ ...clone(row), rowId: `row-${secondResourceId}`, resourceId: secondResourceId, orderKey: second.orderKey });
    }
    const clientA = fixture.clientSelections.find((client) => client.clientId === parameters.clientAId);
    const clientB = fixture.clientSelections.find((client) => client.clientId === parameters.clientBId);
    Object.assign(clientA, {
      expectedSelectedResourceId: parameters.expectedClientAResourceId,
      observedSelectedResourceId: parameters.expectedClientAResourceId,
      expectedInspectorResourceId: parameters.expectedClientAResourceId,
      observedInspectorResourceId: parameters.expectedClientAResourceId,
    });
    Object.assign(clientB, {
      expectedSelectedResourceId: parameters.expectedClientBResourceId,
      observedSelectedResourceId: parameters.expectedClientAResourceId,
      expectedInspectorResourceId: parameters.expectedClientBResourceId,
      observedInspectorResourceId: parameters.expectedClientAResourceId,
    });
  }
  if (kind === 'selection_recovery_invalid') fixture.selection.recoveryActionCount = 2;
  if (kind === 'deep_link_recovery_invalid') {
    fixture.selection.deepLinkRequestedId = `${targetId}-missing`;
    fixture.selection.deepLinkResolvedId = null;
  }
  if (kind === 'wrong_action_target') action.invokedTargetResourceId = `${targetId}-wrong`;
  if (kind === 'action_eligibility_mismatch') action.displayedEligible = false;
  if (kind === 'ineligible_action_exposed') {
    action.eligible = false;
    action.displayedEligible = true;
    action.rendered = true;
  }
  if (kind === 'warning_axis_conflated') {
    fixture.warnings.accessHealth = 'failed';
    fixture.warnings.displayedAxes = ['convergence'];
  }
  if (kind === 'warning_axis_mismatch') {
    fixture.warnings.accessHealth = 'failed';
    fixture.warnings.displayedAxes = [];
  }
  if (kind === 'convergence_action_count_invalid') {
    fixture.warnings.convergenceHealth = 'failed';
    fixture.warnings.displayedAxes = ['convergence'];
    fixture.warnings.convergenceActionIds = [];
  }
  if (kind === 'internal_handoff_url') fixture.handoffUrls = ['http://127.0.0.1:9222/remote-view/internal'];
  if (kind === 'stale_stream_ready') fixture.stream.authoritativeReady = false;
  if (kind === 'stream_not_converged') {
    fixture.stream.streamRevision = fixture.stream.snapshotRevision - 1;
    fixture.stream.displayedReady = false;
  }
  if (kind === 'console_error') fixture.consoleEntries = [{ entryId: 'console-1', level: 'error', message: 'synthetic error' }];
  if (kind === 'network_failure') {
    fixture.networkEntries = [{
      entryId: 'network-1',
      url: 'https://dashboard.example.test/api/service/status',
      status: 503,
      error: 'synthetic network failure',
    }];
  }
  const uiKind = Object.entries(UI_FINDING).find(([, code]) => code === kind)?.[0];
  if (uiKind) fixture.uiChecks.find((check) => check.kind === uiKind).state = 'failed';
  if (kind === 'latency_budget_exceeded') fixture.timings[0].p95Ms = fixture.timings[0].p95BudgetMs + 1;
  const slopeEntry = Object.entries(RESOURCE_FIELDS).find(([, [, code]]) => code === kind);
  if (slopeEntry) mutateResourceSlope(fixture, slopeEntry[0]);
  if (kind === 'capture_gap') fixture.captureGaps.push({ gapId: 'gap-1', evidenceClass: 'screenshot', detail: 'synthetic missing screenshot' });
  return fixture;
}

export function auditDashboardProjection({ fixtureSet, options = {} }) {
  const input = clone(fixtureSet);
  const reports = (input.fixtures ?? []).map((entry) => {
    const fixture = entry.mutation
      ? materializeDashboardFixture({ baseline: input.baseline, descriptor: entry })
      : entry;
    return auditDashboardFixture({ fixture, options });
  });
  return {
    schemaVersion: 'agent-browser.p158-dashboard-oracle-report-set.v1',
    planId: 'P158',
    inputSha256: stableDashboardHash(input),
    repairAttempted: false,
    passed: reports.every((report) => report.passed),
    reports,
  };
}

function denseResource(resourceType, index, labelCardinality, namespace, rowExpected) {
  const resourceId = `${namespace}-${resourceType}-${String(index + 1).padStart(5, '0')}`;
  return {
    resourceId,
    resourceType,
    label: `${resourceType} ${index % labelCardinality}`,
    state: 'ready',
    rowExpected,
    orderKey: index,
    badge: null,
    count: 0,
  };
}

export function generateDenseDashboardFixture({
  seed = 158,
  profiles = 100,
  browsers = 500,
  tabs = 2_000,
  jobs = 10_000,
  events = 10_000,
  idNamespace = `p158-${seed}`,
  labelCardinality = 17,
} = {}) {
  const groups = [
    ['profile', profiles, true],
    ['browser', browsers, true],
    ['tab', tabs, false],
    ['job', jobs, false],
    ['event', events, false],
  ];
  let orderKey = 0;
  const resources = groups.flatMap(([type, count, rowExpected]) =>
    Array.from({ length: count }, (_, index) => {
      const resource = denseResource(type, index, labelCardinality, idNamespace, rowExpected);
      resource.orderKey = orderKey;
      orderKey += 1;
      return resource;
    }),
  );
  const railRows = resources.filter((resource) => resource.rowExpected).map((resource) => ({
    rowId: `row-${resource.resourceId}`,
    resourceId: resource.resourceId,
    resourceType: resource.resourceType,
    label: resource.label,
    state: resource.state,
    snapshotRevision: 1,
    orderKey: resource.orderKey,
    badge: resource.badge,
    count: resource.count,
  }));
  return {
    fixtureId: `${idNamespace}-dense`,
    description: 'Deterministic Plan 0158 dense dashboard truth fixture.',
    density: 'dense',
    generator: { generatorVersion: 'p158-dashboard-dense.v1', seed, profiles, browsers, tabs, jobs, events, idNamespace, labelCardinality },
    truth: { snapshotRevision: 1, counts: { profiles, browsers, tabs, jobs, events }, resources },
    railRows,
    selection: {
      selectedResourceId: railRows[0]?.resourceId ?? null,
      inspectorResourceId: railRows[0]?.resourceId ?? null,
      selectedExists: railRows.length > 0,
      recoveryActionCount: 1,
      deepLinkRequestedId: railRows[0]?.resourceId ?? null,
      deepLinkResolvedId: railRows[0]?.resourceId ?? null,
    },
    actions: [],
    warnings: { runtimeHealth: 'healthy', convergenceHealth: 'healthy', accessHealth: 'healthy', acquisitionHealth: 'healthy', displayedAxes: [], convergenceActionIds: [] },
    handoffUrls: [],
    stream: { snapshotRevision: 1, streamRevision: 1, displayedReady: true, authoritativeReady: true },
    clientSelections: [],
    consoleEntries: [],
    networkEntries: [],
    uiChecks: [],
    timings: [],
    resourceSlopeBudgets: clone(DEFAULT_SLOPE_BUDGETS),
    resourceSamples: [],
    captureGaps: [],
    expectedFindingCodes: [],
  };
}
