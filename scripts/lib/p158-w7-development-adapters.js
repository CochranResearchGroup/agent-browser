import { execFile as nodeExecFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { createP158CaseAdapter } from './p158-execution-schedule.js';
import { sha256 } from './p158-campaign-controller.js';
import { compileP158W7AgentOrchestration } from './p158-w7-agent-orchestration.js';

export const P158_W7_CASE_IDS = Object.freeze([
  ...Array.from({ length: 15 }, (_, index) => `A${String(index + 1).padStart(2, '0')}`),
  ...Array.from({ length: 10 }, (_, index) => `X${String(index + 1).padStart(2, '0')}`),
]);

export const P158_W7_REVIEWED_LIVE_CASE_IDS = Object.freeze(['A07', 'A13', 'X06']);

export const P158_W7_LIVE_HOOK_GAPS = Object.freeze({
  A01: 'multi_client_identity_load_driver',
  A02: 'shared_browser_concurrency_driver',
  A03: 'same_label_connection_driver',
  A04: 'arbitrary_profile_operation_decision_oracle_missing',
  A05: 'acl_barrier_transition_driver',
  A06: 'queued_command_barrier_seam_missing',
  A08: 'identity_proof_fixture_driver',
  A09: 'cdp_target_pathology_driver',
  A10: 'owned_foreign_inventory_churn_driver',
  A11: 'scheduler_terminal_fault_driver',
  A12: 'effect_boundary_lock_timeout_driver',
  A14: 'development_scoped_full_shutdown_driver',
  A15: 'cross_transport_history_marker_driver',
  X01: 'development_xvfb_orphan_driver',
  X02: 'display_allocator_race_driver',
  X03: 'display_evidence_fault_driver',
  X04: 'owned_foreign_display_exhaustion_driver',
  X05: 'x11_authority_matrix_driver',
  X07: 'supervisor_fault_state_injection_driver',
  X08: 'development_install_handoff_shutdown_driver',
  X09: 'generation_digest_mismatch_driver',
  X10: 'disposable_host_epoch_driver',
});

export const P158_W7_REQUIRED_SEAMS = Object.freeze({
  A01: {
    kind: 'campaign_harness',
    minimalSeam: 'Drive existing labeled Service requests from 100 sequential and 25 concurrent distinct client IDs.',
  },
  A02: {
    kind: 'campaign_harness',
    minimalSeam: 'Coordinate ten existing shared-browser clients at each of twenty frozen barriers.',
  },
  A03: {
    kind: 'campaign_harness',
    minimalSeam: 'Open ten existing Service clients with one shared label and distinct immutable connection IDs.',
  },
  A04: {
    kind: 'product_source',
    minimalSeam: 'Expose an access-decision contract for arbitrary profile operations; the current access-plan path hardcodes tab_create.',
  },
  A05: {
    kind: 'campaign_harness',
    minimalSeam: 'Apply existing revisioned profile-policy updates at six frozen workload barriers.',
  },
  A06: {
    kind: 'product_source',
    minimalSeam: 'Add a development-only queued-command hold and exact once release contract around existing revocation and tab eviction.',
  },
  A08: {
    kind: 'campaign_harness',
    minimalSeam: 'Install the frozen unproven and inconsistent identity fixtures, then invoke the four existing actions.',
  },
  A09: {
    kind: 'campaign_harness',
    minimalSeam: 'Orchestrate target creation and closure for seven remaining pathologies; about:blank is already bound.',
  },
  A10: {
    kind: 'campaign_harness',
    minimalSeam: 'Pre-stage exact owned and foreign CDP processes and run existing inventory observations under churn.',
  },
  A11: {
    kind: 'product_source',
    minimalSeam: 'Add a development-only, barrier-controlled scheduler executor outcome injector for every terminal boundary.',
  },
  A12: {
    kind: 'product_source',
    minimalSeam: 'Add a development-only Service State lock-holder barrier that can release before effect, after effect, or after disconnect.',
  },
  A14: {
    kind: 'product_source',
    minimalSeam: 'Add a development-runtime-scoped full-shutdown plan and apply command that cannot address production resources.',
  },
  A15: {
    kind: 'campaign_harness',
    minimalSeam: 'Coordinate one frozen marker through HTTP, MCP, dashboard, and remote control; CLI marker navigation is already bound.',
  },
});

const CASE_SPECS = Object.freeze({
  A01: { hook: 'cli', stimuli: [] },
  A02: { hook: 'browser', stimuli: [] },
  A03: { hook: 'browser', stimuli: [] },
  A04: { hook: 'cli', stimuli: ['policy_mutation'] },
  A05: { hook: 'cli', stimuli: ['policy_transition'] },
  A06: { hook: 'browser', stimuli: ['revocation', 'eviction'] },
  A07: { hook: 'process', stimuli: ['browser_crash'] },
  A08: { hook: 'browser', stimuli: ['identity_fixture'] },
  A09: { hook: 'browser', stimuli: ['target_pathology'] },
  A10: { hook: 'browser', stimuli: ['inventory_churn'] },
  A11: { hook: 'cli', stimuli: ['scheduler_fault'] },
  A12: { hook: 'cli', stimuli: ['lock_or_timeout'] },
  A13: { hook: 'systemd', stimuli: ['daemon_transition', 'supervisor_transition'] },
  A14: { hook: 'shutdown', stimuli: ['full_shutdown'] },
  A15: { hook: 'browser', stimuli: ['history_marker'] },
  X01: { hook: 'display', stimuli: ['xvfb_orphan'] },
  X02: { hook: 'display', stimuli: ['allocator_race'] },
  X03: { hook: 'display', stimuli: ['display_evidence_fault'] },
  X04: { hook: 'display', stimuli: ['display_exhaustion'] },
  X05: { hook: 'display', stimuli: ['x11_authority_fixture'] },
  X06: { hook: 'display', stimuli: ['desktop_locator_state'] },
  X07: { hook: 'systemd', stimuli: ['supervisor_transition'] },
  X08: { hook: 'shutdown', stimuli: ['preserve_transition', 'full_shutdown'] },
  X09: { hook: 'cli', stimuli: ['generation_mismatch'] },
  X10: { hook: 'systemd', stimuli: ['service_restart', 'host_restart'] },
});

const RESULT_STATES = new Set([
  'passed',
  'reproduced_historical_failure',
  'new_product_failure',
  'harness_failure',
  'inconclusive',
  'skipped_blocked',
  'safety_stopped',
]);

const RESULT_PRIORITY = Object.freeze([
  'safety_stopped',
  'harness_failure',
  'new_product_failure',
  'reproduced_historical_failure',
  'inconclusive',
  'skipped_blocked',
  'passed',
]);

const W7_SOURCE_PATH = 'scripts/lib/p158-w7-development-adapters.js';

function w7SourceSha256() {
  return createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
}

export class P158W7AdapterError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W7AdapterError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W7AdapterError(code, message, details);
}

function assertDevelopmentTarget(target) {
  const valid =
    target?.runtimeLane === 'development' &&
    target?.isolationState === 'isolated' &&
    target?.ownership === 'p158_campaign' &&
    target?.production === false &&
    target?.foreign === false &&
    target?.tenantDataPresent === false &&
    typeof target?.targetId === 'string' && target.targetId.length > 0 &&
    typeof target?.campaignRunId === 'string' && target.campaignRunId.length > 0;
  if (!valid) {
    fail('development_target_unproven', 'W7 adapters require an isolated development target', {
      targetId: target?.targetId ?? null,
      runtimeLane: target?.runtimeLane ?? null,
    });
  }
}

function cartesian(rows) {
  return rows.reduce((products, row) => products.flatMap((product) =>
    row.values.map((value) => [...product, { dimensionId: row.id, value }])), [[]]);
}

function plannedActions(contract, attempt, planAction) {
  const actions = [];
  for (const allocation of attempt.cardinalityAllocations ?? []) {
    for (const actionId of allocation.actionIds) {
      actions.push({
        actionId,
        source: 'cardinality',
        cardinalityId: allocation.id,
        ordinal: Number.parseInt(actionId.slice(actionId.lastIndexOf(':') + 1), 10),
      });
    }
  }
  const cartesianDimensions = contract.executionContract.dimensions.filter(
    (dimension) => dimension.coverage === 'cartesian',
  );
  if (cartesianDimensions.length > 0 && attempt.executionUnit.ordinal === 1) {
    for (const [index, assignment] of cartesian(cartesianDimensions).entries()) {
      actions.push({
        actionId: `${attempt.attemptId}:matrix:${String(index + 1).padStart(5, '0')}`,
        source: 'matrix',
        dimensionAssignments: assignment,
      });
    }
  }
  const eachDimensions = contract.executionContract.dimensions.filter(
    (dimension) => dimension.coverage === 'each' &&
      dimension.id !== contract.executionContract.expansion.dimensionId,
  );
  for (const dimension of eachDimensions) {
    const values = contract.executionContract.expansion.count === 1
      ? dimension.values
      : [dimension.values[(attempt.executionUnit.ordinal - 1) % dimension.values.length]];
    for (const value of values) {
      actions.push({
        actionId: `${attempt.attemptId}:dimension:${dimension.id}:${value}`,
        source: 'dimension',
        dimensionAssignments: [{ dimensionId: dimension.id, value }],
      });
    }
  }
  if (attempt.executionUnit.dimensionAssignment) {
    const { dimensionId, value } = attempt.executionUnit.dimensionAssignment;
    actions.push({
      actionId: `${attempt.attemptId}:dimension:${dimensionId}:${value}`,
      source: 'expanded_dimension',
      dimensionAssignments: [{ dimensionId, value }],
    });
  }
  if (actions.length === 0) {
    actions.push({ actionId: `${attempt.attemptId}:unit:00001`, source: 'execution_unit' });
  }
  const unique = new Map(actions.map((action) => [action.actionId, action]));
  if (unique.size !== actions.length) fail('duplicate_planned_action_id', attempt.attemptId);
  return [...unique.values()].map((action) => {
    const planned = planAction({
      caseId: contract.caseId,
      attempt: structuredClone(attempt),
      action: structuredClone(action),
      caseSpec: structuredClone(CASE_SPECS[contract.caseId]),
    });
    if (planned === null || typeof planned !== 'object' || Array.isArray(planned)) {
      fail('action_plan_invalid', `${contract.caseId} planner did not return an action object`);
    }
    for (const field of ['actionId', 'source', 'cardinalityId', 'ordinal', 'dimensionAssignments']) {
      if (Object.hasOwn(planned, field) &&
          JSON.stringify(planned[field]) !== JSON.stringify(action[field])) {
        fail('immutable_action_identity_changed',
          `${contract.caseId} planner changed immutable action field ${field}`);
      }
    }
    return { ...action, ...planned };
  });
}

function assertResourceBindings(action, target) {
  for (const bindingName of ['command', 'evidenceCommand', 'logCommand']) {
    const executable = action[bindingName]?.executable;
    if (executable !== undefined &&
        (!Array.isArray(target.allowedExecutables) ||
          !target.allowedExecutables.includes(executable))) {
      fail('executable_not_owned', `${action.actionId} executable is not development-owned`, {
        bindingName,
        executable,
      });
    }
  }
  if (action.systemd?.unit !== undefined &&
      (!Array.isArray(target.allowedSystemdUnits) ||
        !target.allowedSystemdUnits.includes(action.systemd.unit))) {
    fail('systemd_unit_not_owned', `${action.actionId} systemd unit is not development-owned`);
  }
  if (action.process?.pid !== undefined &&
      (!Array.isArray(target.allowedProcessIds) ||
        !target.allowedProcessIds.includes(action.process.pid))) {
    fail('process_not_owned', `${action.actionId} process is not development-owned`);
  }
  if (action.profilePath !== undefined &&
      (!Array.isArray(target.allowedProfilePaths) ||
        !target.allowedProfilePaths.includes(action.profilePath))) {
    fail('profile_not_owned', `${action.actionId} profile is not development-owned`);
  }
  if (action.browserId !== undefined &&
      (!Array.isArray(target.allowedBrowserIds) ||
        !target.allowedBrowserIds.includes(action.browserId))) {
    fail('browser_not_owned', `${action.actionId} browser is not development-owned`);
  }
  if (action.displayName !== undefined &&
      (!Array.isArray(target.allowedDisplayNames) ||
        !target.allowedDisplayNames.includes(action.displayName))) {
    fail('display_not_owned', `${action.actionId} display is not development-owned`);
  }
}

function assertCommandBinding(actionId, field, binding) {
  if (typeof binding?.executable !== 'string' || !binding.executable.startsWith('/') ||
      !Array.isArray(binding.args) ||
      binding.args.some((argument) => typeof argument !== 'string')) {
    fail('live_command_binding_invalid',
      `${actionId} requires an absolute executable and exact string arguments for ${field}`);
  }
}

function deepFreeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value)) deepFreeze(child);
  }
  return value;
}

function requiredStimulus(caseId, action, attempt, contract) {
  const value = (dimensionId) =>
    action.dimensionAssignments?.find((entry) => entry.dimensionId === dimensionId)?.value;
  if (caseId === 'A13') {
    const owner = value('transition_owner') ??
      contract.executionContract.dimensions.find((entry) =>
        entry.id === 'transition_owner').values[
        (attempt.executionUnit.ordinal - 1) %
          contract.executionContract.dimensions.find((entry) =>
            entry.id === 'transition_owner').values.length
      ];
    return owner === 'daemon'
      ? 'daemon_transition'
      : 'supervisor_transition';
  }
  if (caseId === 'A14') {
    return value('shutdown_plan') === 'authorized_sacrificial' ? 'full_shutdown' : null;
  }
  if (caseId === 'X08') {
    return value('install_path_transition') === 'sealed_full_shutdown'
      ? 'full_shutdown'
      : 'preserve_transition';
  }
  if (caseId === 'X10') {
    return value('epoch_transition') === 'boot' ? 'host_restart' : 'service_restart';
  }
  if (caseId === 'A06') return 'eviction';
  return CASE_SPECS[caseId].stimuli.length === 1 ? CASE_SPECS[caseId].stimuli[0] : null;
}

export function enumerateP158W7ActionPlans({ schedule }) {
  const contracts = new Map(schedule.caseContracts.map((contract) => [contract.caseId, contract]));
  const actions = [];
  for (const attempt of schedule.attempts.filter((entry) =>
    P158_W7_CASE_IDS.includes(entry.caseId))) {
    const contract = contracts.get(attempt.caseId);
    if (!contract) fail('case_contract_missing', `Schedule omitted W7 case ${attempt.caseId}`);
    for (const action of plannedActions(contract, attempt, () => ({}))) {
      actions.push({
        actionId: action.actionId,
        caseId: attempt.caseId,
        attemptId: attempt.attemptId,
        environmentIds: [...attempt.environmentIds],
        hook: CASE_SPECS[attempt.caseId].hook,
        dimensionAssignments: structuredClone(action.dimensionAssignments ?? []),
        allowedStimuli: [...CASE_SPECS[attempt.caseId].stimuli],
        requiredStimulus: requiredStimulus(attempt.caseId, action, attempt, contract),
      });
    }
  }
  if (new Set(actions.map((action) => action.actionId)).size !== actions.length) {
    fail('duplicate_planned_action_id', 'W7 action IDs are not globally unique');
  }
  return deepFreeze(structuredClone(actions));
}

function reviewedEvidenceCommands(target, unit = target.daemonUnit) {
  return {
    evidenceCommand: {
      executable: '/usr/bin/systemctl',
      args: ['--user', 'show', unit, '--property=ActiveState,SubState,MainPID'],
    },
    logCommand: {
      executable: '/usr/bin/journalctl',
      args: ['--user-unit', unit, '--since', target.evidenceSince, '--output=json'],
    },
  };
}

function reviewedBrowserBinding(action, target) {
  const dimension = (id) =>
    action.dimensionAssignments.find((entry) => entry.dimensionId === id)?.value;
  let url = null;
  if (action.caseId === 'A09' && dimension('target_pathology') === 'blank') {
    url = 'about:blank';
  } else if (action.caseId === 'A15' && dimension('control_transport') === 'cli') {
    url = `${target.localFixtureOrigin}/${target.campaignRunId}/markers/${encodeURIComponent(action.actionId)}`;
  } else if (action.caseId === 'X06') {
    const fixture = target.desktopFixtureBindingsByActionId[action.actionId];
    if (!fixture ||
        fixture.windowState !== dimension('window_state') ||
        typeof fixture.browserId !== 'string' ||
        typeof fixture.profilePath !== 'string' ||
        typeof fixture.displayName !== 'string' ||
        typeof fixture.locatorId !== 'string') {
      fail('reviewed_live_target_binding_invalid',
        `W7 X06 action ${action.actionId} requires its exact staged desktop fixture`);
    }
    return {
      ...action,
      targetId: target.targetId,
      campaignRunId: target.campaignRunId,
      stimulusKind: action.requiredStimulus,
      browserId: fixture.browserId,
      profilePath: fixture.profilePath,
      displayName: fixture.displayName,
      command: {
        executable: target.developmentBinary,
        args: [
          '--json',
          '--session', target.sessionName,
          'desktop', 'locate',
          '--browser-id', fixture.browserId,
          '--locator-id', fixture.locatorId,
          '--max-candidates', '32',
        ],
      },
      evidenceCommand: {
        executable: target.developmentBinary,
        args: ['--json', '--session', target.sessionName, 'service', 'status'],
      },
      logCommand: {
        executable: '/usr/bin/journalctl',
        args: ['--user-unit', target.daemonUnit, '--since', target.evidenceSince, '--output=json'],
      },
    };
  } else {
    return null;
  }
  return {
    ...action,
    targetId: target.targetId,
    campaignRunId: target.campaignRunId,
    stimulusKind: action.requiredStimulus,
    browserId: target.agentBrowserId,
    profilePath: target.agentProfilePath,
    command: {
      executable: target.developmentBinary,
      args: [
        '--json',
        '--session', target.sessionName,
        '--runtime-profile', target.runtimeProfile,
        'open', url,
      ],
    },
    evidenceCommand: {
      executable: target.developmentBinary,
      args: [
        '--json',
        '--session', target.sessionName,
        '--runtime-profile', target.runtimeProfile,
        'get', 'url',
      ],
    },
    logCommand: {
      executable: '/usr/bin/journalctl',
      args: ['--user-unit', target.daemonUnit, '--since', target.evidenceSince, '--output=json'],
    },
  };
}

export function assessP158W7ReviewedLiveDispatcher({ schedule, target }) {
  assertDevelopmentTarget(target);
  const requiredTargetFields = [
    'disposableRoot',
    'browserBindingsByActionId',
    'daemonUnit',
    'supervisorUnit',
    'evidenceSince',
    'developmentBinary',
    'agentBrowserId',
    'agentProfilePath',
    'runtimeProfile',
    'sessionName',
    'localFixtureOrigin',
    'desktopFixtureBindingsByActionId',
  ];
  for (const field of requiredTargetFields) {
    if (target[field] === undefined || target[field] === null || target[field] === '') {
      fail('reviewed_live_target_binding_missing', `W7 reviewed dispatcher requires ${field}`);
    }
  }
  if (!target.disposableRoot.startsWith('/tmp/') ||
      !target.disposableRoot.includes(target.campaignRunId)) {
    fail('reviewed_live_target_binding_invalid',
      'W7 disposable root must be campaign-bound under /tmp');
  }
  const expected = enumerateP158W7ActionPlans({ schedule });
  const bindings = [];
  for (const action of expected) {
    let binding;
    if (action.caseId === 'A07') {
      const browser = target.browserBindingsByActionId[action.actionId];
      if (!Number.isInteger(browser?.pid) || browser.pid < 2 ||
          typeof browser.browserId !== 'string' ||
          typeof browser.profilePath !== 'string' ||
          !browser.profilePath.startsWith(`${target.disposableRoot}/`)) {
        fail('reviewed_live_target_binding_invalid',
          `W7 A07 action ${action.actionId} requires an exact disposable browser binding`);
      }
      binding = {
        ...action,
        targetId: target.targetId,
        campaignRunId: target.campaignRunId,
        stimulusKind: 'browser_crash',
        browserId: browser.browserId,
        profilePath: browser.profilePath,
        process: { pid: browser.pid, signal: 'SIGKILL' },
        evidenceCommand: {
          executable: '/usr/bin/ps',
          args: ['-o', 'pid=,ppid=,lstart=,args=', '-p', String(browser.pid)],
        },
        logCommand: {
          executable: '/usr/bin/journalctl',
          args: ['--user-unit', target.daemonUnit, '--since', target.evidenceSince, '--output=json'],
        },
      };
    } else if (action.caseId === 'A13') {
      const unit = action.caseId === 'A13' &&
        action.requiredStimulus === 'daemon_transition'
        ? target.daemonUnit
        : target.supervisorUnit;
      binding = {
        ...action,
        targetId: target.targetId,
        campaignRunId: target.campaignRunId,
        stimulusKind: action.requiredStimulus,
        systemd: { unit, verb: 'restart', executable: '/usr/bin/systemctl' },
        ...reviewedEvidenceCommands(target, unit),
      };
    } else {
      binding = reviewedBrowserBinding(action, target);
      if (!binding) continue;
    }
    const {
      allowedStimuli: _allowedStimuli,
      requiredStimulus: _requiredStimulus,
      ...exactBinding
    } = binding;
    assertCommandBinding(action.actionId, 'evidenceCommand', exactBinding.evidenceCommand);
    assertCommandBinding(action.actionId, 'logCommand', exactBinding.logCommand);
    assertActionPlan(action.caseId, exactBinding, target);
    bindings.push(deepFreeze(exactBinding));
  }
  const blockers = Object.entries(P158_W7_LIVE_HOOK_GAPS).map(([caseId, missingHook]) => ({
    caseId,
    code: 'live_case_hook_missing',
    missingHook,
    requiredSeam: P158_W7_REQUIRED_SEAMS[caseId] ?? null,
    affectedActionCount: expected.filter((action) => action.caseId === caseId).length -
      bindings.filter((binding) => binding.caseId === caseId).length,
  }));
  return deepFreeze({
    schemaVersion: 'agent-browser.p158-w7-reviewed-live-dispatcher-readiness.v1',
    scheduleSha256: schedule.scheduleSha256,
    targetSha256: sha256(target),
    ready: blockers.length === 0,
    implementedCaseIds: [...P158_W7_REVIEWED_LIVE_CASE_IDS],
    partiallyImplementedCaseIds: ['A09', 'A15'],
    implementedActionCount: bindings.length,
    blockerCount: blockers.length,
    blockers,
    bindings,
    effectsExecuted: false,
  });
}

export function validateP158W7LiveBindingManifest({ schedule, target, manifest }) {
  assertDevelopmentTarget(target);
  const frozenTarget = structuredClone(target);
  const expectedActions = enumerateP158W7ActionPlans({ schedule });
  if (manifest?.schemaVersion !== 'agent-browser.p158-w7-live-bindings.v1' ||
      !Array.isArray(manifest.actions)) {
    fail('live_binding_manifest_invalid', 'W7 live binding manifest is missing or malformed');
  }
  const expectedById = new Map(expectedActions.map((action) => [action.actionId, action]));
  const observedById = new Map();
  for (const supplied of manifest.actions) {
    if (typeof supplied?.actionId !== 'string' || observedById.has(supplied.actionId)) {
      fail('live_binding_action_duplicate', supplied?.actionId ?? 'missing actionId');
    }
    const expected = expectedById.get(supplied.actionId);
    if (!expected) fail('live_binding_action_unexpected', supplied.actionId);
    for (const field of ['caseId', 'attemptId', 'hook']) {
      if (supplied[field] !== expected[field]) {
        fail('live_binding_identity_mismatch', `${supplied.actionId} mismatched ${field}`);
      }
    }
    if (supplied.targetId !== frozenTarget.targetId ||
        supplied.campaignRunId !== frozenTarget.campaignRunId) {
      fail('foreign_target_prohibited', `${supplied.actionId} has a foreign target binding`);
    }
    if (supplied.stimulusKind !== expected.requiredStimulus) {
      fail('declared_stimulus_binding_missing', supplied.actionId);
    }
    if (supplied.repair === true || supplied.retry === true ||
        supplied.garbageCollect === true) {
      fail('reactionary_action_prohibited', supplied.actionId);
    }
    assertCommandBinding(supplied.actionId, 'evidenceCommand', supplied.evidenceCommand);
    assertCommandBinding(supplied.actionId, 'logCommand', supplied.logCommand);
    if (['cli', 'browser', 'display', 'shutdown'].includes(expected.hook)) {
      assertCommandBinding(supplied.actionId, 'command', supplied.command);
    } else if (expected.hook === 'systemd') {
      if (typeof supplied.systemd?.unit !== 'string' ||
          typeof supplied.systemd?.verb !== 'string') {
        fail('live_systemd_binding_invalid', supplied.actionId);
      }
    } else if (expected.hook === 'process') {
      if (!Number.isInteger(supplied.process?.pid) || supplied.process.pid < 2 ||
          typeof supplied.process?.signal !== 'string') {
        fail('live_process_binding_invalid', supplied.actionId);
      }
    }
    const compiled = {
      ...structuredClone(supplied),
      environmentIds: [...expected.environmentIds],
      repair: false,
      retry: false,
      garbageCollect: false,
    };
    assertActionPlan(expected.caseId, compiled, frozenTarget);
    observedById.set(supplied.actionId, compiled);
  }
  const missingActionIds = expectedActions
    .map((action) => action.actionId)
    .filter((actionId) => !observedById.has(actionId));
  if (missingActionIds.length > 0) {
    fail('live_binding_action_missing', 'W7 live binding manifest is incomplete', {
      missingActionIds,
    });
  }
  const actions = expectedActions.map((action) => observedById.get(action.actionId));
  const body = {
    schemaVersion: 'agent-browser.p158-w7-live-bindings.v1',
    scheduleSha256: schedule.scheduleSha256,
    target: frozenTarget,
    targetSha256: sha256(frozenTarget),
    actionCount: actions.length,
    actions,
  };
  const compiledManifest = deepFreeze({
    ...body,
    manifestSha256: sha256(body),
  });
  return {
    manifest: compiledManifest,
    liveReady: false,
    blockerCode: 'live_w7_dispatcher_implementation_unproven',
  };
}

function assertActionPlan(caseId, action, target) {
  const spec = CASE_SPECS[caseId];
  if (action.hook !== spec.hook) {
    fail('undeclared_hook_prohibited', `${caseId} cannot invoke ${action.hook}`, {
      expected: spec.hook,
    });
  }
  if (action.stimulusKind && !spec.stimuli.includes(action.stimulusKind)) {
    fail('undeclared_stimulus_prohibited', `${caseId} cannot invoke ${action.stimulusKind}`);
  }
  if (action.targetId !== target.targetId || action.campaignRunId !== target.campaignRunId) {
    fail('foreign_target_prohibited', `${action.actionId} targets a foreign campaign resource`);
  }
  if (action.repair === true || action.retry === true || action.garbageCollect === true) {
    fail('reactionary_action_prohibited', `${action.actionId} requests repair, retry, or GC`);
  }
  assertResourceBindings(action, target);
}

function terminalState(receipts) {
  for (const state of RESULT_PRIORITY) {
    if (receipts.some((receipt) => receipt.resultState === state)) return state;
  }
  return 'harness_failure';
}

export function createP158DevelopmentCommandPrimitives({
  target,
  execFile = promisify(nodeExecFile),
  kill = process.kill.bind(process),
  clock = () => new Date().toISOString(),
} = {}) {
  assertDevelopmentTarget(target);
  const targetBinding = structuredClone(target);
  async function executeCommand(action, bindingName = 'command') {
    assertResourceBindings(action, targetBinding);
    const binding = action[bindingName];
    const executable = binding?.executable;
    const args = binding?.args ?? [];
    if (typeof executable !== 'string' || executable.length === 0 || !Array.isArray(args)) {
      fail('command_binding_missing', `${action.actionId} has no reviewed command binding`);
    }
    const result = await execFile(executable, args, {
      cwd: binding.cwd,
      env: binding.env,
      timeout: binding.timeoutMilliseconds,
      maxBuffer: binding.maxBufferBytes ?? 4 * 1024 * 1024,
    });
    const stdout = Buffer.from(result.stdout ?? '');
    const stderr = Buffer.from(result.stderr ?? '');
    return {
      resultState: 'passed',
      observedAt: clock(),
      stdoutSha256: createHash('sha256').update(stdout).digest('hex'),
      stdoutByteCount: stdout.byteLength,
      stderrSha256: createHash('sha256').update(stderr).digest('hex'),
      stderrByteCount: stderr.byteLength,
    };
  }
  return {
    captureEvidence: (action) => executeCommand(action, 'evidenceCommand'),
    captureLogs: (action) => executeCommand(action, 'logCommand'),
    executeCli: executeCommand,
    executeBrowser: executeCommand,
    executeDisplay: executeCommand,
    executeShutdown: executeCommand,
    async executeSystemd(action) {
      assertResourceBindings(action, targetBinding);
      if (!action.systemd?.unit || !action.systemd?.verb) {
        fail('systemd_binding_missing', `${action.actionId} has no exact systemd binding`);
      }
      return executeCommand({
        ...action,
        command: {
          executable: action.systemd.executable ?? '/usr/bin/systemctl',
          args: ['--user', action.systemd.verb, action.systemd.unit],
          ...action.systemd.commandOptions,
        },
      });
    },
    async executeProcess(action) {
      assertResourceBindings(action, targetBinding);
      if (!Number.isInteger(action.process?.pid) || action.process.pid < 2 ||
          typeof action.process.signal !== 'string') {
        fail('process_binding_missing', `${action.actionId} has no exact process binding`);
      }
      kill(action.process.pid, action.process.signal);
      return { resultState: 'passed', observedAt: clock(), pid: action.process.pid };
    },
  };
}

export function createP158W7DevelopmentAdapterBundle({
  schedule,
  target,
  primitives,
  planAction,
  additionalAdapters = [],
  caseIds = P158_W7_CASE_IDS,
}) {
  assertDevelopmentTarget(target);
  const targetBinding = structuredClone(target);
  if (typeof planAction !== 'function') fail('action_planner_missing', 'W7 requires an action planner');
  for (const method of ['captureEvidence', 'captureLogs']) {
    if (typeof primitives?.[method] !== 'function') {
      fail('primitive_missing', `W7 requires primitive ${method}`);
    }
  }
  const contracts = new Map(schedule.caseContracts.map((contract) => [contract.caseId, contract]));
  for (const caseId of caseIds) {
    if (!contracts.has(caseId)) fail('case_contract_missing', `Schedule omitted W7 case ${caseId}`);
  }
  const executedActionIds = new Set();
  const effects = {};
  const adapters = caseIds.map((caseId) => {
    const contract = contracts.get(caseId);
    const effectId = contract.declaredEffectIds[0];
    effects[effectId] = async (payload, attempt) => {
      assertDevelopmentTarget(targetBinding);
      const action = payload.action;
      assertActionPlan(caseId, action, targetBinding);
      if (executedActionIds.has(action.actionId)) {
        fail('action_already_executed', `${action.actionId} cannot execute twice`);
      }
      executedActionIds.add(action.actionId);
      const evidence = [];
      let operation;
      let stage = 'pre';
      try {
        evidence.push(await primitives.captureEvidence({ ...action, stage: 'pre', attempt }));
        stage = 'effect';
        const method = `execute${action.hook[0].toUpperCase()}${action.hook.slice(1)}`;
        if (typeof primitives[method] !== 'function') fail('primitive_missing', method);
        operation = await primitives[method]({ ...action, stage: 'effect', attempt });
      } catch (error) {
        operation = {
          resultState: stage === 'effect' && !(error instanceof P158W7AdapterError)
            ? 'new_product_failure'
            : 'harness_failure',
          failureStage: stage,
          errorCode: error?.code ?? error?.name ?? 'effect_failed',
          errorMessage: error?.message ?? String(error),
        };
      }
      try {
        evidence.push(await primitives.captureLogs({ ...action, stage: 'logs', attempt }));
        evidence.push(await primitives.captureEvidence({ ...action, stage: 'post', attempt }));
      } catch (error) {
        operation = {
          ...operation,
          resultState: 'harness_failure',
          evidenceError: error?.message ?? String(error),
        };
      }
      const resultState = RESULT_STATES.has(operation?.resultState)
        ? operation.resultState
        : 'harness_failure';
      return {
        actionId: action.actionId,
        resultState,
        operation,
        evidence,
        artifactIds: evidence.flatMap((entry) =>
          entry.artifactIds ?? (entry.artifactId ? [entry.artifactId] : [])),
        repairAttempted: false,
        retryAttempted: false,
        garbageCollectionAttempted: false,
      };
    };
    return createP158CaseAdapter({
      caseId,
      evidenceProfile: contract.evidenceProfile,
      executionContract: contract.executionContract,
      execute: async ({ attempt, requestEffect }) => {
        if (!['E0', 'E1', 'E2', 'E3'].includes(attempt.environmentId)) {
          fail('environment_prohibited', attempt.environmentId);
        }
        if (attempt.environmentIds.includes('E3') && targetBinding.redactedComparisonOnly !== true) {
          fail('production_comparison_prohibited', 'E3 requires a pre-frozen redacted comparison');
        }
        const actions = plannedActions(contract, attempt, planAction);
        const receipts = [];
        for (const action of actions) {
          receipts.push(await requestEffect(effectId, { action }));
        }
        return {
          resultState: terminalState(receipts),
          actionCount: actions.length,
          actionIds: actions.map((action) => action.actionId),
          receipts,
          repairAttempted: false,
          retryAttempted: false,
          garbageCollectionAttempted: false,
        };
      },
    });
  });
  return {
    adapters: [...adapters, ...additionalAdapters],
    w7Adapters: adapters,
    effects,
    executedActionIds,
  };
}

export function createP158W7LiveDevelopmentAdapterBundle({
  schedule,
  target,
  primitives,
  agentWorkflowManifest = null,
  agentWorkflowDrivers = null,
  a01A03LiveBundle = null,
  a04A06LiveBundle = null,
  a07A13LiveBundle = null,
  a08LiveBundle = null,
  additionalAdapters = [],
  liveHookManifestSha256,
}) {
  if (!/^[a-f0-9]{64}$/u.test(liveHookManifestSha256 ?? '')) {
    fail('live_hook_manifest_unproven', 'W7 live adapters require the exact frozen live-hook manifest digest');
  }
  const reviewed = assessP158W7ReviewedLiveDispatcher({ schedule, target });
  const liveBindings = new Map(reviewed.bindings
    .filter((binding) => P158_W7_REVIEWED_LIVE_CASE_IDS.includes(binding.caseId))
    .map((binding) => [binding.actionId, binding]));
  const concrete = createP158W7DevelopmentAdapterBundle({
    schedule,
    target,
    primitives,
    caseIds: P158_W7_REVIEWED_LIVE_CASE_IDS,
    planAction: ({ caseId, attempt, action }) => {
      const binding = liveBindings.get(action.actionId);
      if (!binding || binding.caseId !== caseId || binding.attemptId !== attempt.attemptId) {
        fail('reviewed_live_binding_missing', action.actionId);
      }
      const {
        caseId: _caseId,
        attemptId: _attemptId,
        environmentIds: _environmentIds,
        dimensionAssignments: _dimensionAssignments,
        ...plan
      } = binding;
      return structuredClone(plan);
    },
  });
  const agentOrchestration = agentWorkflowManifest === null
    ? null
    : compileP158W7AgentOrchestration({
        schedule,
        target,
        actionPlans: enumerateP158W7ActionPlans({ schedule }),
        manifest: agentWorkflowManifest,
        drivers: agentWorkflowDrivers,
      });
  const agentConcreteCaseIds = new Set(agentOrchestration?.concreteCaseIds ?? []);
  if (a01A03LiveBundle !== null &&
      (a01A03LiveBundle.freezeEligible !== true || a01A03LiveBundle.providerFree !== false ||
       a01A03LiveBundle.liveHookManifestSha256 !== liveHookManifestSha256 ||
       a01A03LiveBundle.campaignRunId !== target.campaignRunId ||
       a01A03LiveBundle.candidateSha256 !== target.candidateSha256 ||
       !/^[a-f0-9]{64}$/u.test(a01A03LiveBundle.ownershipManifestSha256 ?? '') ||
       !/^[a-f0-9]{64}$/u.test(a01A03LiveBundle.driverSource?.sourceSha256 ?? '') ||
       typeof a01A03LiveBundle.driverSource?.sourcePath !== 'string' ||
       JSON.stringify(a01A03LiveBundle.concreteCaseIds) !== JSON.stringify(['A01', 'A02', 'A03']) ||
       !Array.isArray(a01A03LiveBundle.adapters) || a01A03LiveBundle.adapters.length !== 3 ||
       !Array.isArray(a01A03LiveBundle.liveHookIds) || a01A03LiveBundle.liveHookIds.length !== 1)) {
    fail('a01_a03_live_bundle_unproven', 'A01-A03 promotion requires the exact frozen live bundle');
  }
  if (a04A06LiveBundle !== null &&
      (a04A06LiveBundle.freezeEligible !== true || a04A06LiveBundle.providerFree !== false ||
       a04A06LiveBundle.liveHookManifestSha256 !== liveHookManifestSha256 ||
       a04A06LiveBundle.campaignRunId !== target.campaignRunId ||
       a04A06LiveBundle.candidateSha256 !== target.candidateSha256 ||
       !/^[a-f0-9]{64}$/u.test(a04A06LiveBundle.ownershipManifestSha256 ?? '') ||
       !/^[a-f0-9]{64}$/u.test(a04A06LiveBundle.driverSource?.sourceSha256 ?? '') ||
       typeof a04A06LiveBundle.driverSource?.sourcePath !== 'string' ||
       JSON.stringify(a04A06LiveBundle.concreteCaseIds) !== JSON.stringify(['A05']) ||
       !Array.isArray(a04A06LiveBundle.adapters) || a04A06LiveBundle.adapters.length !== 1 ||
       !Array.isArray(a04A06LiveBundle.liveHookIds) || a04A06LiveBundle.liveHookIds.length !== 1)) {
    fail('a04_a06_live_bundle_unproven', 'A05 promotion requires the exact frozen A04-A06 boundary bundle');
  }
  if (a07A13LiveBundle !== null &&
      (a07A13LiveBundle.freezeEligible !== true || a07A13LiveBundle.providerFree !== false ||
       a07A13LiveBundle.liveHookManifestSha256 !== liveHookManifestSha256 ||
       a07A13LiveBundle.campaignRunId !== target.campaignRunId ||
       a07A13LiveBundle.candidateSha256 !== target.candidateSha256 ||
       !/^[a-f0-9]{64}$/u.test(a07A13LiveBundle.ownershipManifestSha256 ?? '') ||
       !/^[a-f0-9]{64}$/u.test(a07A13LiveBundle.driverSource?.sourceSha256 ?? '') ||
       a07A13LiveBundle.driverSource?.sourcePath !== 'scripts/lib/p158-w7-a07-a13-live.js' ||
       JSON.stringify(a07A13LiveBundle.concreteCaseIds) !== JSON.stringify(['A13']) ||
       !Array.isArray(a07A13LiveBundle.adapters) || a07A13LiveBundle.adapters.length !== 1 ||
       JSON.stringify(a07A13LiveBundle.liveHookIds) !== JSON.stringify(['w7.a07_a13.retained_generation']) ||
       !Array.isArray(a07A13LiveBundle.loggingOperationDescriptors) ||
       a07A13LiveBundle.loggingOperationDescriptors.length !== 211)) {
    fail('a07_a13_live_bundle_unproven', 'A13 promotion requires the exact frozen retained-generation bundle');
  }
  if (a08LiveBundle !== null &&
      (a08LiveBundle.freezeEligible !== true || a08LiveBundle.providerFree !== false ||
       a08LiveBundle.liveHookManifestSha256 !== liveHookManifestSha256 ||
       a08LiveBundle.campaignRunId !== target.campaignRunId ||
       a08LiveBundle.candidateSha256 !== target.candidateSha256 ||
       !/^[a-f0-9]{64}$/u.test(a08LiveBundle.replayManifestSha256 ?? '') ||
       a08LiveBundle.driverSource?.sourcePath !== 'scripts/lib/p158-w7-a08-live.js' ||
       !/^[a-f0-9]{64}$/u.test(a08LiveBundle.driverSource?.sourceSha256 ?? '') ||
       JSON.stringify(a08LiveBundle.concreteCaseIds) !== JSON.stringify(['A08']) ||
       !Array.isArray(a08LiveBundle.adapters) || a08LiveBundle.adapters.length !== 1 ||
       JSON.stringify(a08LiveBundle.liveHookIds) !==
         JSON.stringify(['w7.a08.profile_identity_fixture_replay']) ||
       !Array.isArray(a08LiveBundle.loggingOperationDescriptors) ||
       a08LiveBundle.loggingOperationDescriptors.length !== 8)) {
    fail('a08_live_bundle_unproven', 'A08 promotion requires the exact frozen identity replay bundle');
  }
  const specializedBundles = [a01A03LiveBundle, a04A06LiveBundle, a07A13LiveBundle,
    a08LiveBundle].filter(Boolean);
  const specializedByCase = new Map(specializedBundles.flatMap((bundle) =>
    bundle.concreteCaseIds.map((caseId) => [caseId, bundle])));
  const specializedCaseIds = new Set(specializedByCase.keys());
  // Reviewed command shapes are not sufficient ownership proof. Until frozen
  // candidate/environment receipts bind and are revalidated against each PID,
  // unit, profile, display, browser, and target at effect time, every W7 case
  // remains explicit_blocked.
  const concreteCaseIds = new Set([...agentConcreteCaseIds, ...specializedCaseIds]);
  const contracts = new Map(schedule.caseContracts.map((contract) => [contract.caseId, contract]));
  const sourceSha256 = w7SourceSha256();
  const explicitBlockedAdapters = Object.entries(P158_W7_LIVE_HOOK_GAPS)
    .filter(([caseId]) => !concreteCaseIds.has(caseId))
    .map(([caseId, missingHook]) => {
      const contract = contracts.get(caseId);
      const blocker = deepFreeze({
        code: 'live_case_hook_missing',
        detail: missingHook,
        sourcePath: W7_SOURCE_PATH,
        sourceSha256,
      });
      return createP158CaseAdapter({
        caseId,
        evidenceProfile: contract.evidenceProfile,
        executionContract: contract.executionContract,
        execute: async () => ({
          resultState: 'skipped_blocked',
          blocker,
          effectState: 'not_started',
          requestedEffects: [],
          retryDisposition: 'prohibited_opportunistic_retry',
          repairAttempted: false,
          retryAttempted: false,
          garbageCollectionAttempted: false,
        }),
      });
    });
  const actionCounts = new Map(P158_W7_CASE_IDS.map((caseId) => [
    caseId,
    enumerateP158W7ActionPlans({ schedule }).filter((action) => action.caseId === caseId).length,
  ]));
  const adapterBindings = P158_W7_CASE_IDS.map((caseId) => {
    const concreteLive = concreteCaseIds.has(caseId);
    const agentConcrete = agentConcreteCaseIds.has(caseId);
    const specialized = specializedCaseIds.has(caseId);
    const specializedBundle = specializedByCase.get(caseId);
    const partial = ['A09', 'A15'].includes(caseId);
    return deepFreeze({
      caseId,
      mode: concreteLive ? 'concrete_live' : 'explicit_blocked',
      providerFree: false,
      sourcePath: specialized
        ? specializedBundle.driverSource.sourcePath
        : (agentConcrete ? agentOrchestration.driverSource.sourcePath : W7_SOURCE_PATH),
      sourceSha256: specialized
        ? specializedBundle.driverSource.sourceSha256
        : (agentConcrete ? agentOrchestration.driverSource.sourceSha256 : sourceSha256),
      hookIds: concreteLive
        ? (specialized
            ? [...specializedBundle.liveHookIds]
            : (agentConcrete
            ? ['w7.agent_existing_seam_workflow']
            : (caseId === 'A07'
            ? ['w7.evidence', 'w7.logs', 'w7.process']
            : (caseId === 'A13'
                ? ['w7.evidence', 'w7.logs', 'w7.systemd']
                : ['w7.display', 'w7.evidence', 'w7.logs']))))
        : (partial ? ['w7.browser', 'w7.evidence', 'w7.logs'] : []),
      implementedActionCount: concreteLive
        ? actionCounts.get(caseId)
        : reviewed.bindings.filter((binding) => binding.caseId === caseId).length,
      blockedActionCount: concreteLive ? 0 : actionCounts.get(caseId),
      effectsAllowed: concreteLive,
      blocker: concreteLive ? null : {
        code: 'live_case_hook_missing',
        detail: P158_W7_LIVE_HOOK_GAPS[caseId],
      },
    });
  });
  const undecoratedW7ByCase = new Map([
    ...concrete.w7Adapters,
    ...(agentOrchestration?.adapters ?? []),
    ...(a01A03LiveBundle?.adapters ?? []),
    ...(a04A06LiveBundle?.adapters ?? []),
    ...(a07A13LiveBundle?.adapters ?? []),
    ...(a08LiveBundle?.adapters ?? []),
    ...explicitBlockedAdapters,
  ].map((adapter) => [adapter.caseId, adapter]));
  const w7Adapters = P158_W7_CASE_IDS.map((caseId) => {
    const adapter = undecoratedW7ByCase.get(caseId);
    const binding = adapterBindings.find((entry) => entry.caseId === caseId);
    return deepFreeze({
      ...adapter,
      executionMode: binding.mode,
      providerFree: false,
      effectsAllowed: binding.effectsAllowed,
      sourcePath: binding.sourcePath,
      sourceSha256: binding.sourceSha256,
      liveHookManifestSha256,
      liveBindingSha256: sha256(binding),
      liveHookIds: [...binding.hookIds],
      blocker: binding.blocker === null ? null : {
        ...binding.blocker,
        sourcePath: binding.sourcePath,
        sourceSha256: binding.sourceSha256,
      },
    });
  });
  const w7ByCase = new Map(w7Adapters.map((adapter) => [adapter.caseId, adapter]));
  const adapters = [...w7Adapters, ...additionalAdapters];
  return {
    adapters,
    w7Adapters,
    effects: { ...concrete.effects, ...(agentOrchestration?.effects ?? {}) },
    executedActionIds: concrete.executedActionIds,
    adapterBindings: deepFreeze(adapterBindings),
    reviewedLiveDispatcher: reviewed,
    agentOrchestration,
    ready: true,
  };
}
