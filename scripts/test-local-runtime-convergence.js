#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const script = readFileSync('scripts/converge-local-runtime.js', 'utf8');
const installer = readFileSync('scripts/install-dashboard-user-service.sh', 'utf8');
const publisher = readFileSync('scripts/publish-local-dashboard-runtime.js', 'utf8');
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'));
const plan = readFileSync('docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md', 'utf8');

assert.equal(
  packageJson.scripts['converge:local-runtime'],
  'node scripts/converge-local-runtime.js',
  'package script must expose the local convergence command',
);

assert.match(
  script,
  /const options = \{[\s\S]*apply: false,[\s\S]*json: false,[\s\S]*evidencePath: '',[\s\S]*\};/,
  'local convergence command must dry-run by default',
);

assert.match(
  script,
  /skipPublish: false[\s\S]*arg === '--skip-publish'/,
  'local convergence must support a health-only mode that does not replace installed runtime artifacts',
);

assert.match(
  script,
  /if \(arg === '--'\)[\s\S]*continue;/,
  'local convergence command must tolerate pnpm argument separator forwarding',
);

assert.match(
  script,
  /report\.initial = readDoctors\('initial', \{ required: !options\.apply \}\)/,
  'apply mode must proceed after repairable nonzero initial doctor JSON',
);

assert.match(
  script,
  /function isSafeStaleDaemonRemedy\(argv\)[\s\S]*argv\[0\] === 'agent-browser'[\s\S]*argv\[1\] === 'close'[\s\S]*argv\[2\] === '--session'/,
  'apply mode must only run agent-browser close --session stale-daemon remedies',
);

assert.match(
  script,
  /function repairConfirmedStaleDaemons\(install, label\)[\s\S]*sleep\(2000\)[\s\S]*confirm_stale_daemons_install_doctor[\s\S]*confirmedSessions\.has\(remedy\.session\)/,
  'stale daemon cleanup must confirm the same session after the startup metadata grace period',
);

assert.match(
  script,
  /daemonListenerInventory[\s\S]*function staleDaemonListenerSessions\(install\)[\s\S]*deletedExecutable === true[\s\S]*matchesCurrentExecutable === false[\s\S]*staleDaemonCandidates\(confirmedInstall\)/,
  'interlock must map confirmed deleted or mismatched daemon listeners back to scoped sessions',
);

assert.match(
  script,
  /prepare_stale_daemon_handoff_[\s\S]*handoff', 'prepare'[\s\S]*resume_stale_daemon_handoff_[\s\S]*handoff', 'resume'[\s\S]*resumedBrowserPid !== handoff\.browserPid[\s\S]*resumedCdpUrl !== handoff\.cdpUrl/,
  'stale daemon repair must preserve and verify browser PID and CDP endpoint through handoff',
);

assert.match(
  script,
  /retireConfirmedIdleDaemon\(remedy\.listenerPid\)[\s\S]*function retireConfirmedIdleDaemon\(pid\)[\s\S]*process\.kill\(pid, 'SIGTERM'\)/,
  'idle stale daemons must retire after confirming there is no browser to hand off',
);

assert.doesNotMatch(
  script,
  /close_stale_daemon_/,
  'stale executable repair must not close active browser sessions',
);

assert.match(
  script,
  /const staleMetadataCandidates = staleSessionMetadataNames\(\)[\s\S]*sleep\(2000\)[\s\S]*confirmedStaleMetadata[\s\S]*close_stale_session_metadata_/,
  'apply mode must confirm stale session metadata before using scoped session close commands',
);

assert.match(
  script,
  /function staleSessionMetadataNames\(\{ minimumAgeMs = 60_000 \} = \{\}\)[\s\S]*name\.endsWith\('\.token'\)[\s\S]*mtimeMs >= minimumAgeMs[\s\S]*'\.pid'[\s\S]*'\.sock'/,
  'stale session metadata discovery must be limited to aged token-only runtime entries',
);

assert.match(
  script,
  /const agentBrowserCommand = process\.env\.AGENT_BROWSER_BIN \|\| 'agent-browser'/,
  'local convergence must accept an absolute installed binary path from service environments',
);

assert.match(
  script,
  /const pnpmCommand = process\.env\.PNPM_BIN \|\| 'pnpm'/,
  'local convergence must accept an absolute pnpm path from service environments',
);

assert.match(
  script,
  /publish:local-dashboard[\s\S]*--skip-browser[\s\S]*repairConfirmedStaleDaemons\(afterPublish\.install[\s\S]*ensure:rdp-guac-postgres[\s\S]*test:rdp-guac-route-pool-readiness/,
  'apply mode must sequence local publish, stale daemon remedies, Guacamole schema ensure, and route-pool readiness',
);

assert.match(
  script,
  /routeDisplayRecoveryRequired\(afterRoutePool\.remoteView\.nextAction\)[\s\S]*open:rdp-route-displays[\s\S]*after_route_display_restore/,
  'apply mode must restore missing RDP route displays and verify the result',
);

assert.match(
  script,
  /afterRoutePool\.remoteView\.nextAction === 'grant_route_display_access'[\s\S]*grant:rdp-route-display-access/,
  'apply mode must run display-access grant only when remote-view doctor requests it',
);

assert.match(
  script,
  /function writeEvidence\(payload\)[\s\S]*\.agent-browser\/convergence\/local-runtime-latest\.json[\s\S]*writeFileSync/,
  'apply mode must retain convergence evidence in a runtime-local JSON file',
);

assert.doesNotMatch(
  script,
  /killall|pkill|rm -rf|docker compose down|TerminateProcess\(/,
  'local convergence command must not contain broad destructive process or filesystem operations',
);

assert.match(
  installer,
  /Description=agent-browser runtime health interlock[\s\S]*converge:local-runtime -- --apply --skip-publish --json/,
  'dashboard service installation must install the runtime-health interlock service',
);

assert.match(
  publisher,
  /prepareRuntimeHandoffs\(builtBin, installBin\)[\s\S]*installBinaryAtomically\(builtBin, installBin[\s\S]*resumeRuntimeHandoffs\(installBin\)/,
  'local publishing must bracket executable replacement with daemon handoff',
);

assert.match(
  publisher,
  /data\.browserPid !== prepared\.browserPid[\s\S]*data\.cdpUrl !== prepared\.cdpUrl/,
  'local publishing must verify browser PID and CDP endpoint continuity',
);

assert.match(
  publisher,
  /unsupportedActiveSessions[\s\S]*publish was stopped before replacing the executable/,
  'local publishing must fail closed when an old daemon cannot hand off an active browser',
);

assert.match(
  publisher,
  /const daemonClientBin = runtimeDaemonClientBinary\(daemonPid, rollbackBin\)[\s\S]*serviceBrowserForSession\(daemonClientBin[\s\S]*runAgentJson\(daemonClientBin, sessionName, \['close'\]\)/,
  'publisher inventory and idle retirement must use the running daemon executable without triggering hash replacement',
);

assert.match(
  publisher,
  /function runtimeDaemonClientBinary\(daemonPid, fallbackBin\)[\s\S]*`\/proc\/\$\{daemonPid\}\/exe`/,
  'Linux publisher preflight must resolve the running daemon executable through procfs',
);

assert.match(
  installer,
  /Environment=PATH=\$PATH[\s\S]*Environment=AGENT_BROWSER_BIN=\$AGENT_BROWSER_BIN[\s\S]*Environment=PNPM_BIN=\$PNPM_BIN[\s\S]*Environment=AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD=\$AGENT_BROWSER_BIN/,
  'runtime-health interlock must bind command paths explicitly for the systemd user environment',
);

assert.match(
  installer,
  /Description=Periodically reconcile agent-browser runtime health[\s\S]*OnBootSec=20s[\s\S]*OnUnitInactiveSec=\$INTERLOCK_INTERVAL/,
  'dashboard service installation must install the boot and recurring interlock timer',
);

assert.match(
  installer,
  /systemctl --user enable --now[\s\S]*agent-browser-dashboard\.service[\s\S]*agent-browser-runtime-interlock\.timer[\s\S]*systemctl --user start agent-browser-runtime-interlock\.service/,
  'dashboard service installation must enable the timer and run the interlock immediately',
);

assert.match(
  plan,
  /### Slice F: One-Command Local Convergence[\s\S]*`pnpm --silent converge:local-runtime -- --json`[\s\S]*`pnpm --silent converge:local-runtime -- --apply --json`[\s\S]*`~\/\.agent-browser\/convergence\/local-runtime-latest\.json`/,
  'P42 must document the local runtime convergence command, apply mode, and retained evidence path',
);

console.log('Local runtime convergence command contract smoke passed');
