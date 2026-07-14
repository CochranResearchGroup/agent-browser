#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  classifyScenarioFailure,
  routeBoundFinalizationEvidence,
  scenarioSpec,
  supportedScenarioIds,
  validateScenarioSpec,
} from './lib/p46-scenario-harness.js';

assert.deepEqual(supportedScenarioIds(), ['s0', 's1', 's2', 's3', 's3-open', 's4', 's5', 's6', 's7', 's8', 's9', 's10', 's11', 's12']);

const s2 = scenarioSpec('S2');
assert.equal(s2.id, 's2');
assert.equal(validateScenarioSpec(s2).ok, true);
assert.equal(s2.roles.filter((role) => role.type === 'target-browser').length, 1);
assert.equal(s2.roles.filter((role) => role.type === 'viewer-client').length, 2);
assert.equal(
  s2.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  1,
);
assert.equal(
  s2.roles
    .filter((role) => role.type === 'viewer-client')
    .reduce((total, role) => total + role.routeLeases, 0),
  0,
);

const s3 = scenarioSpec('S3');
assert.equal(s3.id, 's3');
assert.equal(validateScenarioSpec(s3).ok, true);
assert.equal(s3.roles.filter((role) => role.type === 'target-browser').length, 1);
assert.equal(s3.roles.filter((role) => role.type === 'viewer-client').length, 2);
assert.ok(s3.invariants.some((invariant) => /default runtime profile/.test(invariant)));
assert.ok(s3.invariants.some((invariant) => /different tab IDs/.test(invariant)));

const s3Open = scenarioSpec('S3-open');
assert.equal(s3Open.id, 's3-open');
assert.equal(validateScenarioSpec(s3Open).ok, true);
assert.equal(s3Open.roles.filter((role) => role.type === 'target-browser').length, 1);
assert.equal(s3Open.roles.filter((role) => role.type === 'viewer-client').length, 0);
assert.ok(s3Open.invariants.some((invariant) => /explicit agent-browser command/.test(invariant)));
assert.ok(s3Open.invariants.some((invariant) => /browser_window_visible/.test(invariant)));

const s4 = scenarioSpec('S4');
assert.equal(s4.id, 's4');
assert.equal(validateScenarioSpec(s4).ok, true);
assert.equal(s4.roles.filter((role) => role.type === 'target-browser').length, 2);
assert.equal(s4.roles.filter((role) => role.type === 'viewer-client').length, 2);
assert.equal(
  s4.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  1,
);
assert.equal(
  s4.roles
    .filter((role) => role.type === 'viewer-client')
    .reduce((total, role) => total + role.routeLeases, 0),
  0,
);
assert.ok(s4.invariants.some((invariant) => /one runtime profile/.test(invariant)));
assert.ok(s4.invariants.some((invariant) => /one retained browser process/.test(invariant)));
assert.ok(s4.invariants.some((invariant) => /closing one browser window/.test(invariant)));

const s5 = scenarioSpec('S5');
assert.equal(s5.id, 's5');
assert.equal(validateScenarioSpec(s5).ok, true);
assert.equal(s5.roles.filter((role) => role.type === 'target-browser').length, 2);
assert.equal(s5.roles.filter((role) => role.type === 'viewer-client').length, 2);
assert.equal(
  s5.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  2,
);
assert.equal(
  s5.roles
    .filter((role) => role.type === 'viewer-client')
    .reduce((total, role) => total + role.routeLeases, 0),
  0,
);
assert.ok(s5.invariants.some((invariant) => /two distinct runtime profiles/.test(invariant)));
assert.ok(s5.invariants.some((invariant) => /distinct route leases/.test(invariant)));
assert.ok(s5.invariants.some((invariant) => /closing profile A/.test(invariant)));

const s6 = scenarioSpec('S6');
assert.equal(s6.id, 's6');
assert.equal(validateScenarioSpec(s6).ok, true);
assert.equal(s6.roles.filter((role) => role.type === 'target-browser').length, 2);
assert.equal(s6.roles.filter((role) => role.type === 'viewer-client').length, 2);
assert.equal(
  s6.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  2,
);
assert.equal(
  s6.roles
    .filter((role) => role.type === 'viewer-client')
    .reduce((total, role) => total + role.routeLeases, 0),
  0,
);
assert.ok(s6.invariants.some((invariant) => /swap dashboard selection/.test(invariant)));
assert.ok(s6.invariants.some((invariant) => /wrong browser/.test(invariant)));

const s7 = scenarioSpec('S7');
assert.equal(s7.id, 's7');
assert.equal(validateScenarioSpec(s7).ok, true);
assert.equal(s7.roles.filter((role) => role.type === 'target-browser').length, 3);
assert.equal(s7.roles.filter((role) => role.type === 'viewer-client').length, 0);
assert.equal(
  s7.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  3,
);
assert.ok(s7.invariants.some((invariant) => /capacity blocker/.test(invariant)));
assert.ok(s7.invariants.some((invariant) => /fake live dashboard row/.test(invariant)));

const s8 = scenarioSpec('S8');
assert.equal(s8.id, 's8');
assert.equal(validateScenarioSpec(s8).ok, true);
assert.equal(s8.roles.filter((role) => role.type === 'target-browser').length, 1);
assert.equal(s8.roles.filter((role) => role.type === 'viewer-client').length, 0);
assert.equal(
  s8.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  1,
);
assert.ok(s8.roles.some((role) => role.id === 'display-access-fixture' && role.type === 'runtime'));
assert.ok(s8.invariants.some((invariant) => /display-access denial/.test(invariant)));
assert.ok(s8.invariants.some((invariant) => /same route-bound open succeeds/.test(invariant)));

const s9 = scenarioSpec('S9');
assert.equal(s9.id, 's9');
assert.equal(validateScenarioSpec(s9).ok, true);
assert.equal(s9.roles.filter((role) => role.type === 'target-browser').length, 1);
assert.equal(s9.roles.filter((role) => role.type === 'viewer-client').length, 3);
assert.equal(
  s9.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  1,
);

const s10 = scenarioSpec('S10');
assert.equal(s10.id, 's10');
assert.equal(validateScenarioSpec(s10).ok, true);
assert.equal(s10.roles.filter((role) => role.type === 'target-browser').length, 1);
assert.equal(s10.roles.filter((role) => role.type === 'foreign-cdp-browser').length, 1);
assert.equal(s10.roles.filter((role) => role.type === 'viewer-client').length, 1);
assert.equal(
  s10.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  1,
);
assert.equal(
  s10.roles
    .filter((role) => role.type === 'foreign-cdp-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  0,
);
assert.ok(s10.invariants.some((invariant) => /foreign CDP browser is inventoried as non-owned/.test(invariant)));
assert.ok(s10.invariants.some((invariant) => /does not borrow service route or display state/.test(invariant)));

const s11 = scenarioSpec('S11');
assert.equal(s11.id, 's11');
assert.equal(validateScenarioSpec(s11).ok, true);
assert.equal(s11.roles.filter((role) => role.type === 'target-browser').length, 1);
assert.equal(s11.roles.filter((role) => role.type === 'viewer-client').length, 1);
assert.equal(
  s11.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  1,
);
assert.equal(
  s11.roles
    .filter((role) => role.type === 'viewer-client')
    .reduce((total, role) => total + role.routeLeases, 0),
  0,
);
assert.ok(s11.invariants.some((invariant) => /survives dashboard reload/.test(invariant)));
assert.ok(s11.invariants.some((invariant) => /stale dashboard tab URLs recover/.test(invariant)));

const s12 = scenarioSpec('S12');
assert.equal(s12.id, 's12');
assert.equal(validateScenarioSpec(s12).ok, true);
assert.equal(s12.roles.filter((role) => role.type === 'target-browser').length, 1);
assert.equal(s12.roles.filter((role) => role.type === 'viewer-client').length, 1);
assert.ok(s12.roles.some((role) => role.id === 'route-pool' && role.type === 'route-pool'));
assert.equal(
  s12.roles
    .filter((role) => role.type === 'target-browser')
    .reduce((total, role) => total + role.routeLeases, 0),
  1,
);
assert.equal(
  s12.roles
    .filter((role) => role.type === 'viewer-client')
    .reduce((total, role) => total + role.routeLeases, 0),
  0,
);
assert.ok(s12.invariants.some((invariant) => /at least ten normal-use cycles/.test(invariant)));
assert.ok(s12.invariants.some((invariant) => /pressure do not trend upward/.test(invariant)));
assert.equal(
  s9.roles
    .filter((role) => role.type === 'viewer-client')
    .reduce((total, role) => total + role.routeLeases, 0),
  0,
);
assert.ok(s9.invariants.some((invariant) => /blank and duplicate same-origin tabs/.test(invariant)));
assert.ok(s9.invariants.some((invariant) => /Blank|blank|stale metadata/.test(invariant)));

const contaminatedS2 = {
  ...s2,
  roles: s2.roles.map((role) =>
    role.type === 'viewer-client' && role.id === 'operator-b'
      ? { ...role, routeLeases: 1 }
      : role,
  ),
};
const contaminatedValidation = validateScenarioSpec(contaminatedS2);
assert.equal(contaminatedValidation.ok, false);
assert.ok(contaminatedValidation.failures.some((failure) => /viewer-client role operator-b/.test(failure)));

assert.equal(
  classifyScenarioFailure(['operator A dashboard did not show a remote viewport iframe']),
  'viewer_client_adapter',
);
assert.equal(
  classifyScenarioFailure(['terminal content visible on route display :13']),
  'route_display_runtime',
);
assert.equal(
  classifyScenarioFailure(['route-bound finalization incomplete: display allocation remote-view-display:13 is pending']),
  'route_bound_finalization',
);
assert.equal(
  classifyScenarioFailure(['same_profile_multi_process_unsupported: duplicate profile lane requires reviewed intent']),
  'profile_topology',
);

const finalizedEvidence = routeBoundFinalizationEvidence({
  openJson: {
    data: {
      acquisitionLease: { id: 'lease-finalized' },
      browserId: 'session:default',
      displayAllocationId: 'remote-view-display:13',
      routeId: 'guacamole:3',
      routePoolEntryId: 'guacamole-rdp-a',
    },
  },
  statusJson: {
    data: {
      service_state: {
        browsers: {
          'session:default': {
            displayAllocationId: 'remote-view-display:13',
            health: 'ready',
            viewStreams: [
              {
                displayAllocationId: 'remote-view-display:13',
                remoteReadiness: { state: 'ready' },
                routeId: 'guacamole:3',
              },
            ],
          },
        },
        displayAllocations: {
          'remote-view-display:13': { state: 'ready', readiness: { state: 'ready' } },
        },
        remoteViewAcquisitionLeases: {
          'lease-finalized': { state: 'completed', phase: 'checked_out' },
        },
        remoteViewRoutes: {
          'guacamole:3': { state: 'ready', readiness: { state: 'ready' } },
        },
        routePool: {
          'guacamole-rdp-a': {
            currentRouteAllocationId: 'guacamole:3',
            state: 'checked_out',
            readiness: { state: 'ready' },
          },
        },
      },
    },
  },
  incidentsJson: { data: { incidents: [] } },
});
assert.equal(finalizedEvidence.finalized, true);
assert.deepEqual(finalizedEvidence.blockers, []);

const finalizedEvidenceFromRouteBinding = routeBoundFinalizationEvidence({
  openJson: {
    data: {
      acquisitionLease: { id: 'lease-finalized' },
      browserId: 'session:default',
      displayAllocationId: 'remote-view-display:13',
      routeId: 'guacamole:3',
      routePoolEntryId: 'guacamole-rdp-a',
    },
  },
  statusJson: {
    data: {
      service_state: {
        browsers: {
          'session:default': {
            displayAllocationId: 'remote-view-display:13',
            health: 'ready',
            viewStreams: [
              {
                displayAllocationId: 'remote-view-display:13',
                provider: 'rdp_gateway',
                url: 'https://agent-browser.example/guacamole/',
              },
            ],
          },
        },
        displayAllocations: {
          'remote-view-display:13': {
            routeIds: ['guacamole:3'],
            state: 'ready',
            readiness: { state: 'ready' },
          },
        },
        remoteViewAcquisitionLeases: {
          'lease-finalized': { state: 'completed', phase: 'checked_out' },
        },
        remoteViewRoutes: {
          'guacamole:3': {
            browserId: 'session:default',
            displayAllocationId: 'remote-view-display:13',
            state: 'ready',
            readiness: { state: 'ready' },
          },
        },
        routePool: {
          'guacamole-rdp-a': {
            currentRouteAllocationId: 'guacamole:3',
            state: 'checked_out',
            readiness: { state: 'ready' },
          },
        },
      },
    },
  },
  incidentsJson: { data: { incidents: [] } },
});
assert.equal(finalizedEvidenceFromRouteBinding.finalized, true);
assert.equal(
  finalizedEvidenceFromRouteBinding.states.browser.routeBindingSource,
  'route_display_binding',
);

const incompleteEvidence = routeBoundFinalizationEvidence({
  openJson: {
    data: {
      acquisitionLease: { id: 'lease-completed' },
      browserId: 'session:default',
      displayAllocationId: 'remote-view-display:13',
      routeId: 'guacamole:3',
      routePoolEntryId: 'guacamole-rdp-a',
    },
  },
  statusJson: {
    data: {
      service_state: {
        browsers: {
          'session:default': {
            displayAllocationId: 'remote-view-display:13',
            health: 'ready',
            viewStreams: [
              {
                displayAllocationId: 'remote-view-display:13',
                remoteReadiness: { state: 'ready' },
                routeId: 'guacamole:3',
              },
            ],
          },
        },
        displayAllocations: {
          'remote-view-display:13': { state: 'pending', readiness: { state: 'pending' } },
        },
        remoteViewAcquisitionLeases: {
          'lease-completed': { state: 'completed', phase: 'checked_out' },
        },
        remoteViewRoutes: {
          'guacamole:3': { state: 'orphaned', readiness: { state: 'orphaned' } },
        },
        routePool: {
          'guacamole-rdp-a': {
            currentRouteAllocationId: 'guacamole:3',
            state: 'pending',
            readiness: { state: 'pending' },
          },
        },
      },
    },
  },
  incidentsJson: {
    data: {
      incidents: [
        {
          id: 'remote-view-route:guacamole:3',
          latestKind: 'remote_view_finalization_incomplete',
          latestMessage: 'Remote route has completed acquisition but display is pending',
          state: 'active',
        },
      ],
    },
  },
});
assert.equal(incompleteEvidence.finalized, false);
assert.ok(incompleteEvidence.blockers.some((blocker) => /display allocation/.test(blocker)));
assert.ok(incompleteEvidence.blockers.some((blocker) => /remote_view_finalization_incomplete/.test(blocker)));

const runnerSource = readFileSync('scripts/run-p46-stress-scenario.js', 'utf8');
const viewerClientSource = readFileSync('scripts/lib/p47-viewer-client.js', 'utf8');
const captureS7Source = runnerSource.slice(
  runnerSource.indexOf('async function captureS7'),
  runnerSource.indexOf('function captureS3OpenProof'),
);
const captureS8Source = runnerSource.slice(
  runnerSource.indexOf('async function captureS8'),
  runnerSource.indexOf('function captureS3OpenProof'),
);
const captureS9Source = runnerSource.slice(
  runnerSource.indexOf('async function captureS9'),
  runnerSource.indexOf('async function captureS10'),
);
const captureS10Source = runnerSource.slice(
  runnerSource.indexOf('async function captureS10'),
  runnerSource.indexOf('async function captureS11'),
);
const captureS11Source = runnerSource.slice(
  runnerSource.indexOf('async function captureS11'),
  runnerSource.indexOf('function captureS3OpenProof'),
);
assert.match(
  runnerSource,
  /scenarioSpec\(normalized\)[\s\S]*validateScenarioSpec\(spec\)/,
  'P46 runner must validate declarative scenario specs before live execution',
);
assert.match(
  runnerSource,
  /classifyScenarioFailure\(result\.failures\)/,
  'P46 runner failure audits must use harness classification',
);
assert.match(
  runnerSource,
  /routeBoundFinalizationEvidence[\s\S]*route-bound-finalization-evidence\.json/,
  'P46 S2 runner must write route-bound finalization evidence',
);
assert.match(
  runnerSource,
  /remoteViewOpenReady\(open\)[\s\S]*failedStage: 'remote_view_open'/,
  'P46 S3 runner must stop before tab controls when remote-view open is not ready',
);
assert.match(
  runnerSource,
  /failedStage: 'tab_handles'[\s\S]*S3 did not obtain two distinct service tab handles/,
  'P46 S3 runner must stop before dashboard launch when tab handles are unsafe',
);
assert.match(
  runnerSource,
  /--agent-browser-command[\s\S]*--require-explicit-agent-browser-command/,
  'P46 runner must expose explicit command authority for remediation runs',
);
assert.match(
  runnerSource,
  /agent-browser-command\.json[\s\S]*missing_explicit_agent_browser_command/,
  'P46 runner must write command metadata and fail before live work when explicit command is required',
);
assert.match(
  runnerSource,
  /--require-agent-browser-daemon-command-match[\s\S]*singleMatchingListener[\s\S]*noListeners[\s\S]*agent_browser_daemon_command_mismatch/,
  'P50 runner must fail before live work when daemon command authority is mismatched while allowing no pre-existing daemon',
);
assert.match(
  runnerSource,
  /same-profile browser row\(s\) before close instead of one retained browser process/,
  'P53-shaped S4 runner must expect one retained browser row for same-profile windows',
);
assert.match(
  runnerSource,
  /captureS3OpenProof[\s\S]*evaluateS3OpenProof/,
  'P50 S3-open runner must isolate route-bound open proof before full S3',
);
assert.match(
  runnerSource,
  /captureS4[\s\S]*evaluateS4/,
  'P46 S4 runner must implement one-profile two-window capture and evaluation',
);
assert.match(
  runnerSource,
  /p46-s4-profile[\s\S]*p46-s4-window-\$\{runId\}[\s\S]*const sessionB = sessionA/,
  'P46 S4 runner must use one daemon session against one runtime profile',
);
assert.match(
  runnerSource,
  /s4SameProfileTopologyPreflight[\s\S]*same_profile_multi_process_unsupported[\s\S]*s4-topology-preflight\.json/,
  'P53 S4 runner must stop with a typed topology blocker before launching a second same-profile process',
);
assert.match(
  runnerSource,
  /window-b-get-url-after-close-a\.json/,
  'P46 S4 runner must verify window B after closing window A',
);
assert.match(
  runnerSource,
  /guacamole-rdp-a[\s\S]*window-b-same-profile-window-open\.json/,
  'P46 S4 runner must use one route-bound open and one same-profile window open',
);
assert.match(
  runnerSource,
  /captureS5[\s\S]*evaluateS5/,
  'P46 S5 runner must implement two-profile concurrent capture and evaluation',
);
assert.match(
  runnerSource,
  /p46-s5-profile-a[\s\S]*p46-s5-profile-b[\s\S]*guacamole-rdp-a[\s\S]*guacamole-rdp-b/,
  'P46 S5 runner must use two explicit runtime profiles and two route-pool entries',
);
assert.match(
  runnerSource,
  /profile-b-get-url-after-close-a\.json/,
  'P46 S5 runner must verify profile B after closing profile A',
);
assert.match(
  runnerSource,
  /captureS5\('s6'\)[\s\S]*evaluateS6/,
  'P46 S6 runner must implement two-profile cross-observation capture and evaluation',
);
assert.match(
  runnerSource,
  /operator-a-swapped-to-profile-b-dashboard-state\.json[\s\S]*operator-b-swapped-to-profile-a-dashboard-state\.json/,
  'P46 S6 runner must capture swapped dashboard selection state for both operators',
);
assert.match(
  runnerSource,
  /operator-a-swapped-to-profile-b-navigate\.json[\s\S]*operator-b-swapped-to-profile-a-navigate\.json/,
  'P46 S6 runner must capture explicit swapped navigation artifacts',
);
assert.match(
  runnerSource,
  /operator-a-swapped-to-profile-b-reconnect\.json[\s\S]*operator-b-swapped-to-profile-a-reconnect\.json/,
  'P46 S6 runner must reconnect viewer-client CDP after swapped dashboard navigation',
);
assert.match(
  runnerSource,
  /operator-a-swapped-to-profile-b-reconnect-discovery\.json[\s\S]*operator-b-swapped-to-profile-a-reconnect-discovery\.json/,
  'P57 S6 runner must write swapped reconnect target-discovery artifacts before reconnect commands',
);
assert.match(
  runnerSource,
  /operator-a-swapped-to-profile-b-page-url\.json[\s\S]*operator-b-swapped-to-profile-a-page-url\.json[\s\S]*reconnectDashboardViewerClient/,
  'P58-shaped S6 runner must wait for swapped dashboard page URL before reconnecting CDP',
);
assert.doesNotMatch(
  runnerSource,
  /if \(isS6\)[\s\S]*Page\.navigate[\s\S]*operator-a-swapped-to-profile-b-dashboard-state\.json/,
  'P46 S6 runner must not use raw Page.navigate for swapped dashboard selection',
);
assert.match(
  runnerSource,
  /operatorASwappedBrowserParam[\s\S]*operatorBSwappedBrowserParam/,
  'P46 S6 evaluator must report swapped selected-browser readback evidence',
);
assert.match(
  runnerSource,
  /activeSessionIds[\s\S]*browser\.id\.startsWith\('session:'\)/,
  'P46 reset helper must close retained browser rows even when session rows are missing',
);
assert.match(
  runnerSource,
  /defaultCommandMaxBuffer = 32 \* 1024 \* 1024[\s\S]*maxBuffer,/,
  'P46 command runner must keep large service status payloads from truncating reset evidence',
);
assert.match(
  runnerSource,
  /S5 saw \$\{browserRowsBeforeClose\.length\} profile browser row\(s\) before close instead of two/,
  'P46 S5 evaluator must require two retained profile browser rows before close',
);
assert.match(
  runnerSource,
  /captureS7[\s\S]*profile-c-third-open-while-occupied\.json[\s\S]*profile-c-retry-after-release\.json/,
  'P46 S7 runner must capture third-demand exhaustion and retry-after-release artifacts',
);
assert.match(
  runnerSource,
  /evaluateS7[\s\S]*routeCapacityBlocker[\s\S]*failed third demand created a retained profile C browser row/,
  'P46 S7 evaluator must require a typed capacity blocker and no fake retained row',
);
assert.doesNotMatch(
  runnerSource,
  /function routeCapacityBlocker[\s\S]*text\.includes\('unavailable'\)[\s\S]*function captureS7/,
  'P46 S7 capacity blocker classifier must not accept generic unavailable owner-mismatch failures',
);
assert.doesNotMatch(
  captureS7Source,
  /s7 third profile remote-view open while routes occupied[\s\S]{0,500}env: poolEnv/,
  'P46 S7 third demand must not reuse stale baseline route-pool JSON',
);
assert.match(
  captureS7Source,
  /s7 third profile remote-view open while routes occupied[\s\S]*routePoolEnvFromServiceStatus\(statusOccupied\.json\)/,
  'P46 S7 third demand must use current occupied route-pool state',
);
assert.match(
  runnerSource,
  /createDisplayAccessDeniedFixture[\s\S]*p46-s8 display access denied fixture[\s\S]*displayAccessBlocker/,
  'P46 S8 runner must use a local display-access denial fixture and typed blocker classifier',
);
assert.match(
  captureS8Source,
  /display-access-denied-open\.json[\s\S]*service-status-after-display-access-denied\.json[\s\S]*display-access-repair-open\.json/,
  'P46 S8 runner must capture denied open, post-denial state, and repair open artifacts',
);
assert.match(
  runnerSource,
  /PATH: `\$\{fixtureDir\}:\$\{process\.env\.PATH \|\| ''\}`[\s\S]*s8 route-bound open with simulated display access denial/,
  'P46 S8 denied open must run with the fixture PATH only for the failure probe',
);
assert.match(
  runnerSource,
  /evaluateS8[\s\S]*displayAccessBlocker\(capture\.deniedOpen\)[\s\S]*failed display-access demand created a retained denied-profile browser row/,
  'P46 S8 evaluator must require typed display-access blocker and no fake retained row',
);
assert.match(
  runnerSource,
  /evaluateS8[\s\S]*display-access repair open did not produce operatorVisible\.state=ready[\s\S]*displayAccessGrantState/,
  'P46 S8 evaluator must require successful repair open and display access grant evidence',
);
assert.match(
  captureS9Source,
  /tab-new-duplicate-b\.json[\s\S]*tab-new-blank\.json[\s\S]*tab-list-after-setup\.json/,
  'P46 S9 runner must create duplicate and blank tab setup artifacts',
);
assert.match(
  captureS9Source,
  /operator-a-duplicate-a-dashboard-state\.json[\s\S]*operator-b-duplicate-b-dashboard-state\.json[\s\S]*allowRecoveredStaleTab[\s\S]*operator-c-blank-dashboard-state\.json/,
  'P46 S9 runner must capture dashboard state for both duplicate tabs and recovered stale blank tab proof',
);
assert.match(
  captureS9Source,
  /navigate-blank-tab\.json[\s\S]*operator-c-blank-recovered-navigate\.json[\s\S]*navigate-duplicate-a\.json[\s\S]*navigate-duplicate-b\.json/,
  'P46 S9 runner must navigate blank and duplicate tabs independently after blank-tab dashboard recovery',
);
assert.match(
  captureS9Source,
  /waitForDashboardViewerClientPageUrl[\s\S]*operator-c-blank-recovered-page-url\.json[\s\S]*reconnectDashboardViewerClient/,
  'P46 S9 runner must use the imported viewer-client page URL wait before reconnecting operator C',
);
assert.doesNotMatch(
  captureS9Source,
  /waitForViewerClientDashboardPageUrl/,
  'P46 S9 runner must not call an undefined viewer-client page URL helper alias',
);
assert.match(
  runnerSource,
  /evaluateS9[\s\S]*three distinct duplicate and blank tab IDs[\s\S]*operator C initial blank tab state neither preserved the requested blank target nor proved stale selected-tab recovery[\s\S]*duplicate tab B changed after duplicate A navigate/,
  'P46 S9 evaluator must require distinct tabs, exact or recovered blank selection, and duplicate-tab isolation',
);
assert.match(
  runnerSource,
  /evaluateS9[\s\S]*operator C initial blank tab dashboard did not mark stale blank tab identity as recovered[\s\S]*route display content after S9 controls/,
  'P46 S9 evaluator must require stale blank tab identity recovery before route display proof',
);
assert.match(
  runnerSource,
  /evaluateS9[\s\S]*operator C blank tab after controls[\s\S]*dashboard tab param mismatch/,
  'P46 S9 evaluator must reject stale blank tab identity while requiring final dashboard selected-tab readback and route display proof',
);
assert.match(
  runnerSource,
  /launchForeignCdpBrowser[\s\S]*remote-debugging-port=0[\s\S]*role: 'foreign-cdp-browser'/,
  'P46 S10 runner must launch a separate foreign CDP browser outside service ownership',
);
assert.match(
  runnerSource,
  /cleanupError[\s\S]*rmSync\(profileDir,[\s\S]*maxRetries:[\s\S]*writeLaunch\(\{ closed: true, cleanupError \}\)/,
  'P46 S10 foreign CDP browser cleanup must be retry-tolerant and must not mask scenario failures',
);
assert.match(
  captureS10Source,
  /remote-view-open-service-owned\.json[\s\S]*foreign-cdp-ready\.json[\s\S]*api\/sessions[\s\S]*dashboard-sessions-inventory\.json[\s\S]*foreign-cdp-session\.json/,
  'P46 S10 runner must capture service-owned open, foreign CDP readiness, and dashboard inventory artifacts',
);
assert.match(
  captureS10Source,
  /api\/session-tabs\?port=\$\{foreignReady\.port\}/,
  'P46 S10 runner must use the dashboard session-tabs API for foreign CDP tab inventory',
);
assert.match(
  captureS10Source,
  /operator-service-owned-selected-workspace-panel\.json[\s\S]*operator-foreign-cdp-selected-workspace-panel\.json[\s\S]*operator-service-owned-selected-workspace-panel-after-switch-back\.json/,
  'P46 S10 runner must capture service-owned, foreign, and switch-back selected workspace panels',
);
assert.match(
  runnerSource,
  /panelReady: Boolean\(panel\)[\s\S]*contextReady: Boolean\(selectedWorkspaceId\)/,
  'P46 S10 selected-workspace probe must distinguish mounted panel evidence from viewport-route context evidence',
);
assert.match(
  runnerSource,
  /Refresh workspace context[\s\S]*Refresh workspace viewport[\s\S]*target: button \? "context" : "viewport"/,
  'P46 S10 selected-workspace refresh must fall back to the mounted viewport refresh control',
);
assert.match(
  runnerSource,
  /evaluateS10[\s\S]*foreignReadOnlyCapabilitiesPresent[\s\S]*foreign selected workspace exposed a mutation[\s\S]*foreign selected workspace borrowed service-owned route/,
  'P46 S10 evaluator must require non-owned foreign CDP classification, capability-backed mutation gating, and no route borrowing',
);
assert.match(
  runnerSource,
  /foreignRouteBorrowed[\s\S]*foreignPanel\?\.viewport\?\.text[\s\S]*displayAllocationId/,
  'P46 S10 evaluator must scope foreign route-borrow detection to selected-workspace viewport evidence',
);
assert.match(
  runnerSource,
  /evaluateS10[\s\S]*service-owned selected workspace context did not render[\s\S]*service-owned selected workspace context did not survive refresh and foreign row switch-back/,
  'P46 S10 evaluator must require service-owned context readiness and selection stability after foreign row switching',
);
assert.match(
  captureS11Source,
  /operator-dashboard-reload\.json[\s\S]*operator-dashboard-state-after-reload\.json[\s\S]*operator-stale-url-navigate\.json/,
  'P46 S11 runner must capture dashboard reload and stale URL navigation artifacts',
);
assert.match(
  captureS11Source,
  /operator-dashboard-state-after-stale-url\.json[\s\S]*operator-stale-url-readback\.json[\s\S]*operator-stale-url-reconnect\.json[\s\S]*operator-viewport-refresh-after-stale-url\.json/,
  'P46 S11 runner must capture stale URL recovery, reconnect after stale URL navigation, and exercise viewport refresh',
);
assert.match(
  captureS11Source,
  /direct-guacamole-initial\.json[\s\S]*direct-guacamole-after-stale-recovery\.json[\s\S]*route-bound-finalization-evidence\.json/,
  'P46 S11 runner must capture direct Guacamole URL readback and route-bound finalization evidence',
);
assert.match(
  runnerSource,
  /evaluateS11[\s\S]*recoveredLiveTab[\s\S]*stale dashboard tab URL did not report selected-target recovery[\s\S]*viewer-client reconnect after stale dashboard URL failed[\s\S]*direct Guacamole frame URL after stale recovery/,
  'P46 S11 evaluator must require stale target recovery, including live-tab URL rewrite, reconnect proof, and direct Guacamole readback',
);
assert.match(
  viewerClientSource,
  /function recoveredLiveTabState[\s\S]*last\.tabParam !== expected\.tabId[\s\S]*allowRecoveredLiveTab/,
  'P46 S11 viewer-client helper must support stale URL recovery by immediate live-tab rewrite without weakening default tab matching',
);
assert.match(
  runnerSource,
  /evaluateS11[\s\S]*route display content after S11 controls[\s\S]*service-owned browser no longer carries the route-bound view stream after S11 controls/,
  'P46 S11 evaluator must require route display and service-owned stream stability after refresh/reconnect stress',
);
assert.match(
  runnerSource,
  /s12CycleCount = Math\.max\(10[\s\S]*captureS12Boundary[\s\S]*install[\s\S]*doctor[\s\S]*service[\s\S]*incidents[\s\S]*--summary/,
  'P46 S12 runner must force at least ten cycles and capture doctor and incident summaries at cycle boundaries',
);
assert.match(
  runnerSource,
  /async function captureS12[\s\S]*operator-dashboard-reload\.json[\s\S]*operator-reconnect\.json[\s\S]*operator-viewport-refresh\.json[\s\S]*switch-tab\.json[\s\S]*resetRuntime\(label\)/,
  'P46 S12 runner must repeat reload, reconnect, refresh, tab switch, and reset in each cycle',
);
assert.match(
  runnerSource,
  /async function captureS12[\s\S]*tabMatchesCommandData\(tab, tabNew\.json\?\.data\)[\s\S]*tabSelector\(newTab, 2\)/,
  'P46 S12 runner must switch to the tab opened by the current cycle rather than a stale tab-list position',
);
assert.match(
  runnerSource,
  /function evaluateS12[\s\S]*route-pool state did not return to baseline after reset[\s\S]*pressure increased after reset[\s\S]*cycleCount: cycles\.length/,
  'P46 S12 evaluator must reject route-pool drift and upward pressure after resets',
);

console.log('P47 scenario harness no-live checks passed');
