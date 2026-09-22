#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const files = ['scripts/libexec/agent-browser-privileged-helper'];

const routePoolSource = readFileSync('scripts/setup-rdp-guac-route-pool.sh', 'utf8');
const displayAccessSource = readFileSync('scripts/grant-rdp-route-display-access.sh', 'utf8');
const workstationInstallSource = readFileSync('cli/src/workstation_install.rs', 'utf8');

assert.doesNotMatch(
  routePoolSource,
  /\bsudo -v\b|\bsudo (?:useradd|chpasswd|usermod|tee|chmod|chown|systemctl)\b|\bsudo -u\b/,
  'installed route-pool setup must never fall back to interactive or unbounded sudo commands',
);
assert.match(routePoolSource, /sudo -n "\$PRIVILEGED_HELPER" ensure-rdp-route-user/);
assert.doesNotMatch(
  routePoolSource,
  /sudo -n "\$PRIVILEGED_HELPER" restart-xrdp/,
  'route-user setup must preserve live XRDP desktops',
);
const workstationRouteUsers = workstationInstallSource.match(
  /fn ensure_route_users\([^]*?\n}\n\nfn route_readiness/,
)?.[0];
assert.ok(workstationRouteUsers, 'workstation route-user reconciliation source must be present');
assert.doesNotMatch(
  workstationRouteUsers,
  /restart-xrdp|restart XRDP/,
  'workstation reconciliation must preserve live XRDP desktops',
);
assert.doesNotMatch(
  displayAccessSource,
  /\bsudo -u\b/,
  'installed display-access setup must never fall back to direct sudo as a route user',
);
assert.match(displayAccessSource, /sudo -n "\$PRIVILEGED_HELPER" grant-display-access/);

function xsessionBlocks(source) {
  const blocks = [];
  const pattern = /(?:cat|tee)[^\n]*\.xsession[^\n]*<<'EOF'\n([\s\S]*?)\nEOF/g;
  for (const match of source.matchAll(pattern)) {
    blocks.push(match[1]);
  }
  return blocks;
}

for (const file of files) {
  const source = readFileSync(file, 'utf8');
  const blocks = xsessionBlocks(source);
  assert.ok(blocks.length > 0, `${file} must write an .xsession heredoc`);

  for (const block of blocks) {
    assert.doesNotMatch(
      block,
      /\b(?:xterm|gnome-terminal|xfce4-terminal|konsole|x-terminal-emulator)\b/i,
      `${file} route .xsession must not start a terminal`,
    );
    assert.match(
      block,
      /openbox-session/,
      `${file} route .xsession should start the window manager when available`,
    );
    assert.match(
      block,
      /while true;\s*do[\s\S]*sleep 3600[\s\S]*done/,
      `${file} route .xsession must keep the XRDP session alive without helper UI`,
    );
  }

  if (file.endsWith('agent-browser-privileged-helper')) {
    assert.match(
      source,
      /chpasswd --crypt-method SHA512 --sha-rounds 100000/,
      `${file} route-user password updates must bypass PAM and GNOME Keyring`,
    );
    assert.doesNotMatch(
      source,
      /\|\s*chpasswd\s*(?:\n|$)/,
      `${file} route-user password updates must never use chpasswd's PAM default`,
    );
    assert.match(
      source,
      /\/proc\/net\/unix/,
      `${file} display access grant must inspect abstract X11 sockets`,
    );
    assert.match(
      source,
      /@\/tmp\/\.X11-unix\/X/,
      `${file} display access grant must accept abstract XRDP X11 sockets`,
    );
    assert.match(
      source,
      /timeout --kill-after=1 2s/,
      `${file} display access grant must bound xhost execution`,
    );

    const status = spawnSync('bash', [file, 'status-json'], {
      encoding: 'utf8',
    });
    assert.equal(status.status, 0, `${file} status-json should exit successfully: ${status.stderr}`);
    const report = JSON.parse(status.stdout);
    assert.equal(report.schemaVersion, 1, `${file} status-json schema version should be stable`);
    assert.match(
      report.helperVersion,
      /^2026-09-19\.p211-route-desktop-v\d+$/,
      `${file} status-json should expose the P211 helper contract version`,
    );
    assert.equal(report.routeDesktopSession?.ready, true);
    assert.equal(report.routeDesktopSession?.state, 'browser_control_ready_template');
    assert.equal(report.routeDesktopSession?.terminalStartupDetected, false);
    assert.equal(report.routeDesktopSession?.startsWindowManager, true);
    assert.equal(report.routeDesktopSession?.keepsSessionAlive, true);
    assert.equal(report.routeSessionObservation?.exactCgroupV2Identity, true);
    assert.equal(report.routeSessionObservation?.xServerProcessIdentity, true);
    assert.equal(report.routeSessionObservation?.x11SocketOwnership, true);
    assert.equal(report.routeSessionTermination?.exactCgroupV2Identity, true);
    assert.equal(report.routeSessionTermination?.retainedDirectoryIdentity, true);
    assert.equal(report.routeSessionTermination?.usesCgroupKill, true);
    assert.equal(report.routeSessionTermination?.broadUserTermination, false);
    assert.equal(report.displayAccess?.supportsFilesystemX11Socket, true);
    assert.equal(report.displayAccess?.supportsAbstractX11Socket, true);
    assert.equal(report.displayAccess?.boundedXhostTimeoutSeconds, 2);
    assert.equal(report.routeUserCredentialUpdate?.pamBypassed, true);
    assert.equal(report.routeUserCredentialUpdate?.cryptMethod, 'SHA512');
    assert.equal(report.routeUserCredentialUpdate?.shaRounds, 100000);
    assert.equal(report.routeUserOwnedProvisioning?.supported, true);
    assert.equal(report.routeUserOwnedProvisioning?.gecosOperationMarker, true);
    assert.equal(report.routeUserOwnedProvisioning?.retrySafe, true);
    assert.equal(
      report.managedChromeSandboxPolicy?.profileName,
      'agent-browser-managed-chrome',
    );
    assert.equal(typeof report.managedChromeSandboxPolicy?.loaded, 'boolean');

    const ownedFixture = mkdtempSync(join(tmpdir(), 'agent-browser-owned-route-helper-'));
    const ownedBin = join(ownedFixture, 'bin');
    const ownedPasswd = join(ownedFixture, 'passwd');
    const ownedLog = join(ownedFixture, 'commands.log');
    const ownedHome = join(ownedFixture, 'home');
    const ownedUser = 'agent-browser-rdp-owned';
    const ownedOperation = '123e4567-e89b-12d3-a456-426614174000';
    try {
      mkdirSync(ownedBin, { recursive: true });
      writeFileSync(ownedLog, '');
      writeFileSync(join(ownedBin, 'id'), '#!/bin/sh\nprintf "0\\n"\n', { mode: 0o755 });
      writeFileSync(join(ownedBin, 'getent'), `#!/bin/sh
if [ "$1" = passwd ] && [ -s ${JSON.stringify(ownedPasswd)} ]; then
  /bin/cat ${JSON.stringify(ownedPasswd)}
  exit 0
fi
exit 2
`, { mode: 0o755 });
      writeFileSync(join(ownedBin, 'useradd'), `#!/bin/sh
comment=""
user=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --comment) comment="$2"; shift 2 ;;
    --create-home|--shell) shift 2 ;;
    *) user="$1"; shift ;;
  esac
done
mkdir -p ${JSON.stringify(ownedHome)}/"$user"
printf '%s:x:2001:2001:%s:%s/%s:/bin/bash\\n' "$user" "$comment" ${JSON.stringify(ownedHome)} "$user" >${JSON.stringify(ownedPasswd)}
printf 'useradd:%s\\n' "$comment" >>${JSON.stringify(ownedLog)}
`, { mode: 0o755 });
      writeFileSync(join(ownedBin, 'chpasswd'), `#!/bin/sh
IFS= read -r value
printf 'chpasswd:%s\\n' "$value" >>${JSON.stringify(ownedLog)}
`, { mode: 0o755 });
      writeFileSync(join(ownedBin, 'usermod'), `#!/bin/sh
printf 'usermod:%s\\n' "$*" >>${JSON.stringify(ownedLog)}
`, { mode: 0o755 });
      writeFileSync(join(ownedBin, 'install'), `#!/bin/sh
for target in "$@"; do :; done
mkdir -p "$target"
`, { mode: 0o755 });
      for (const command of ['chmod', 'chown']) {
        writeFileSync(join(ownedBin, command), '#!/bin/sh\nexit 0\n', { mode: 0o755 });
      }
      const ownedEnv = {
        ...process.env,
        PATH: `${ownedBin}:${process.env.PATH}`,
        AGENT_BROWSER_HELPER_TEST_HOME_ROOT: ownedHome,
      };
      const invokeOwned = (operation = ownedOperation, password = 'fixture-password') => spawnSync(
        'bash',
        [file, 'ensure-rdp-route-user-owned', '--user', ownedUser, '--operation-id', operation],
        { encoding: 'utf8', env: ownedEnv, input: `${password}\n` },
      );

      writeFileSync(ownedPasswd, `${ownedUser}:x:2001:2001:foreign owner:${ownedHome}/${ownedUser}:/bin/bash\n`);
      const collision = invokeOwned();
      assert.equal(collision.status, 2);
      assert.match(collision.stderr, /owned route user identity collision/);
      assert.equal(readFileSync(ownedLog, 'utf8'), '', 'collisions must not mutate the account');

      writeFileSync(ownedPasswd, '');
      const provisioned = invokeOwned();
      assert.equal(provisioned.status, 0, provisioned.stderr);
      assert.match(readFileSync(ownedPasswd, 'utf8'), new RegExp(`operation-id=${ownedOperation}`));
      assert.match(readFileSync(ownedLog, 'utf8'), /useradd:agent-browser route-pool RDP session; operation-id=/);

      writeFileSync(ownedLog, '');
      const retried = invokeOwned(ownedOperation, 'retry-password');
      assert.equal(retried.status, 0, retried.stderr);
      assert.doesNotMatch(readFileSync(ownedLog, 'utf8'), /useradd:/);
      assert.match(readFileSync(ownedLog, 'utf8'), /chpasswd:agent-browser-rdp-owned:retry-password/);

      writeFileSync(ownedLog, '');
      const foreignOperation = invokeOwned('123e4567-e89b-12d3-a456-426614174001');
      assert.equal(foreignOperation.status, 2);
      assert.equal(readFileSync(ownedLog, 'utf8'), '');
    } finally {
      rmSync(ownedFixture, { recursive: true, force: true });
    }

    const fixture = mkdtempSync(join(tmpdir(), 'agent-browser-route-helper-'));
    const bin = join(fixture, 'bin');
    const proc = join(fixture, 'proc');
    const cgroup = join(fixture, 'cgroup');
    const scope = join(cgroup, 'user.slice', 'user-2001.slice', 'session-c42.scope');
    const commandLog = join(fixture, 'commands.log');
    try {
      mkdirSync(bin, { recursive: true });
      mkdirSync(join(proc, '41002', 'fd'), { recursive: true });
      mkdirSync(join(proc, '41003', 'fd'), { recursive: true });
      mkdirSync(scope, { recursive: true });
      writeFileSync(join(bin, 'id'), '#!/bin/sh\nprintf "0\\n"\n', { mode: 0o755 });
      writeFileSync(
        join(bin, 'getent'),
        '#!/bin/sh\nprintf "%s:x:2001:2001:%s:/home/%s:/bin/bash\\n" "$2" "${FIXTURE_GECOS:-agent-browser route-pool RDP session}" "$2"\n',
        { mode: 0o755 },
      );
      writeFileSync(
        join(bin, 'loginctl'),
        `#!/bin/sh
printf '%s\\n' "$*" >>${JSON.stringify(commandLog)}
if [ "$1" = "list-sessions" ]; then
  printf 'c42 2001 agent-browser-rdp-dev-6 seat0 -\\n'
  if [ "\${FIXTURE_LOGINCTL_MODE:-}" = "multiple" ]; then
    printf 'c43 2001 agent-browser-rdp-dev-6 seat0 -\\n'
  fi
  if [ "\${FIXTURE_LOGINCTL_MODE:-}" = "list-fail" ]; then
    exit 7
  fi
elif [ "$1" = "show-session" ]; then
  printf 'Name=agent-browser-rdp-dev-6\\nUser=2001\\nService=xrdp-sesman\\nType=x11\\nState=active\\nLeader=41002\\nScope=session-c42.scope\\n'
  if [ "\${FIXTURE_LOGINCTL_MODE:-}" = "show-fail" ]; then
    exit 8
  fi
else
  exit 2
fi
`,
        { mode: 0o755 },
      );
      writeFileSync(
        join(bin, 'systemctl'),
        '#!/bin/sh\nprintf "0123456789abcdef0123456789abcdef\\n"\n',
        { mode: 0o755 },
      );
      writeFileSync(join(proc, '41002', 'status'), 'Name:\txrdp-chansrv\nUid:\t2001\t2001\t2001\t2001\n');
      writeFileSync(join(proc, '41003', 'status'), 'Name:\tXorg\nUid:\t2001\t2001\t2001\t2001\n');
      const statTail = 'S 1 1 1 0 0 0 0 0 0 0 0 0 0 20 0 1 0 0';
      writeFileSync(join(proc, '41002', 'stat'), `41002 (xrdp-chansrv) ${statTail} 5101\n`);
      writeFileSync(join(proc, '41003', 'stat'), `41003 (Xorg) ${statTail} 5102\n`);
      const cgroupPath = '0::/user.slice/user-2001.slice/session-c42.scope\n';
      writeFileSync(join(proc, '41002', 'cgroup'), cgroupPath);
      writeFileSync(join(proc, '41003', 'cgroup'), cgroupPath);
      writeFileSync(join(proc, '41002', 'cmdline'), Buffer.from('/usr/bin/xrdp-chansrv\0'));
      writeFileSync(join(proc, '41003', 'cmdline'), Buffer.from('/usr/lib/xorg/Xorg\0:21\0-auth\0.Xauthority\0'));
      symlinkSync('socket:[6101]', join(proc, '41003', 'fd', '8'));
      writeFileSync(join(proc, 'net-unix'), 'Num RefCount Protocol Flags Type St Inode Path\n000: 2 0 10000 1 01 6101 /tmp/.X11-unix/X21\n');
      writeFileSync(join(proc, 'boot-id'), '11111111-2222-3333-4444-555555555555\n');
      writeFileSync(join(cgroup, 'cgroup.controllers'), 'cpu memory pids\n');
      writeFileSync(join(scope, 'cgroup.kill'), '');
      writeFileSync(join(scope, 'cgroup.events'), 'populated 1\nfrozen 0\n');
      const postOpenRetainedScope = `${scope}-post-open-retained`;
      writeFileSync(
        join(bin, 'stat'),
        `#!/bin/sh
target=""
for argument in "$@"; do target="$argument"; done
if [ "\${FIXTURE_REBIND_AFTER_OPEN:-}" = "1" ] && [ "\${target#/proc/self/fd/}" != "$target" ] && [ ! -e ${JSON.stringify(postOpenRetainedScope)} ]; then
  mv ${JSON.stringify(scope)} ${JSON.stringify(postOpenRetainedScope)}
  mkdir ${JSON.stringify(scope)}
  : >${JSON.stringify(join(scope, 'cgroup.kill'))}
  printf 'populated 1\\nfrozen 0\\n' >${JSON.stringify(join(scope, 'cgroup.events'))}
fi
exec /usr/bin/stat "$@"
`,
        { mode: 0o755 },
      );
      writeFileSync(
        join(bin, 'sleep'),
        `#!/bin/sh
if [ -e ${JSON.stringify(postOpenRetainedScope)} ] && [ -s ${JSON.stringify(join(postOpenRetainedScope, 'cgroup.kill'))} ]; then
  printf 'populated 0\\nfrozen 0\\n' >${JSON.stringify(join(postOpenRetainedScope, 'cgroup.events'))}
elif [ -s ${JSON.stringify(join(scope, 'cgroup.kill'))} ] && [ "\${FIXTURE_MALFORMED_AFTER_KILL:-}" = "1" ]; then
  : >${JSON.stringify(join(proc, '41002', 'stat'))}
  : >${JSON.stringify(join(proc, '41003', 'stat'))}
  printf 'populated 0\\nfrozen 0\\n' >${JSON.stringify(join(scope, 'cgroup.events'))}
elif [ -s ${JSON.stringify(join(scope, 'cgroup.kill'))} ]; then
  rm -rf ${JSON.stringify(join(proc, '41002'))} ${JSON.stringify(join(proc, '41003'))}
  printf 'populated 0\\nfrozen 0\\n' >${JSON.stringify(join(scope, 'cgroup.events'))}
fi
`,
        { mode: 0o755 },
      );
      const helperEnv = {
        ...process.env,
        PATH: `${bin}:${process.env.PATH}`,
        AGENT_BROWSER_HELPER_TEST_PROC_ROOT: proc,
        AGENT_BROWSER_HELPER_TEST_CGROUP_ROOT: cgroup,
        AGENT_BROWSER_HELPER_TEST_BOOT_ID_PATH: join(proc, 'boot-id'),
        AGENT_BROWSER_HELPER_TEST_PROC_NET_UNIX_PATH: join(proc, 'net-unix'),
      };
      const observed = spawnSync('bash', [file, 'observe-rdp-route-session', '--user', 'agent-browser-rdp-dev-6'], {
        encoding: 'utf8',
        env: helperEnv,
      });
      assert.equal(observed.status, 0, observed.stderr);
      const observation = JSON.parse(observed.stdout);
      assert.equal(observation.state, 'ready', `${observed.stdout}\n${observed.stderr}`);
      assert.equal(
        observation.witness.schemaVersion,
        'agent-browser.route-keeper-xrdp-ownership.v1',
      );
      assert.equal(observation.witness.sessionId, 'c42');
      assert.equal(observation.witness.displayName, ':21');
      assert.equal(observation.witness.leaderPid, 41002);
      assert.equal(observation.witness.xServerPid, 41003);
      assert.equal(observation.witness.x11SocketInode, 6101);
      assert.equal(readFileSync(join(scope, 'cgroup.kill'), 'utf8'), '');

      const ownedMarkerObservation = spawnSync(
        'bash',
        [file, 'observe-rdp-route-session', '--user', 'agent-browser-rdp-dev-6'],
        {
          encoding: 'utf8',
          env: {
            ...helperEnv,
            FIXTURE_GECOS: 'agent-browser route-pool RDP session; operation-id=123e4567-e89b-12d3-a456-426614174000',
          },
        },
      );
      assert.equal(ownedMarkerObservation.status, 0, ownedMarkerObservation.stderr);
      assert.equal(JSON.parse(ownedMarkerObservation.stdout).state, 'ready');

      for (const socketRows of [
        [
          '000: 2 0 10000 1 01 6101 /tmp/.X11-unix/X21',
          '001: 2 0 00000 1 03 6102 /tmp/.X11-unix/X21',
          '002: 2 0 00000 1 02 6103 /tmp/.X11-unix/X21',
        ],
        [
          '000: 2 0 00000 1 03 6102 /tmp/.X11-unix/X21',
          '001: 2 0 00000 1 02 6103 /tmp/.X11-unix/X21',
          '002: 2 0 00010000 1 01 6101 /tmp/.X11-unix/X21',
        ],
      ]) {
        writeFileSync(
          join(proc, 'net-unix'),
          `Num RefCount Protocol Flags Type St Inode Path\n${socketRows.join('\n')}\n`,
        );
        const listenerWithConnectedRows = spawnSync(
          'bash',
          [file, 'observe-rdp-route-session', '--user', 'agent-browser-rdp-dev-6'],
          { encoding: 'utf8', env: helperEnv },
        );
        assert.equal(listenerWithConnectedRows.status, 0, listenerWithConnectedRows.stderr);
        assert.deepEqual(
          JSON.parse(listenerWithConnectedRows.stdout),
          observation,
          'connected rows sharing the X11 pathname must not replace or invalidate the owned listener',
        );
      }
      writeFileSync(
        join(proc, 'net-unix'),
        'Num RefCount Protocol Flags Type St Inode Path\n000: 2 0 00000 1 01 6101 /tmp/.X11-unix/X21\n',
      );
      const boundNonListener = spawnSync(
        'bash',
        [file, 'observe-rdp-route-session', '--user', 'agent-browser-rdp-dev-6'],
        { encoding: 'utf8', env: helperEnv },
      );
      assert.equal(boundNonListener.status, 0, boundNonListener.stderr);
      assert.deepEqual(
        JSON.parse(boundNonListener.stdout),
        {
          schemaVersion: 1,
          state: 'ownership_unproven',
          code: 'rdp_route_session_evidence_invalid',
        },
        'a bound stream socket without the listener flag must not establish X11 ownership',
      );
      writeFileSync(
        join(proc, 'net-unix'),
        'Num RefCount Protocol Flags Type St Inode Path\n000: 2 0 10000 1 01 6101 /tmp/.X11-unix/X21\n',
      );

      const ambiguous = spawnSync(
        'bash',
        [file, 'observe-rdp-route-session', '--user', 'agent-browser-rdp-dev-6'],
        { encoding: 'utf8', env: { ...helperEnv, FIXTURE_LOGINCTL_MODE: 'multiple' } },
      );
      assert.equal(ambiguous.status, 0, ambiguous.stderr);
      assert.deepEqual(JSON.parse(ambiguous.stdout), {
        schemaVersion: 1,
        state: 'ownership_unproven',
        code: 'rdp_route_session_ambiguous',
      });

      for (const mode of ['list-fail', 'show-fail']) {
        const failedLoginctl = spawnSync(
          'bash',
          [file, 'observe-rdp-route-session', '--user', 'agent-browser-rdp-dev-6'],
          { encoding: 'utf8', env: { ...helperEnv, FIXTURE_LOGINCTL_MODE: mode } },
        );
        assert.equal(failedLoginctl.status, 0, failedLoginctl.stderr);
        assert.deepEqual(JSON.parse(failedLoginctl.stdout), {
          schemaVersion: 1,
          state: 'ownership_unproven',
          code: 'rdp_route_session_evidence_invalid',
        });
      }

      const normalSocketTable = readFileSync(join(proc, 'net-unix'), 'utf8');
      for (const conflictingRows of [
        '000: 2 0 10000 1 01 6101 /tmp/.X11-unix/X21\n001: 2 0 10000 1 01 6102 @/tmp/.X11-unix/X21\n',
        '000: 2 0 10000 1 01 6102 @/tmp/.X11-unix/X21\n001: 2 0 10000 1 01 6101 /tmp/.X11-unix/X21\n',
      ]) {
        writeFileSync(join(proc, 'net-unix'), `Num RefCount Protocol Flags Type St Inode Path\n${conflictingRows}`);
        const conflictingSocket = spawnSync(
          'bash',
          [file, 'observe-rdp-route-session', '--user', 'agent-browser-rdp-dev-6'],
          { encoding: 'utf8', env: helperEnv },
        );
        assert.equal(conflictingSocket.status, 0, conflictingSocket.stderr);
        assert.equal(
          JSON.parse(conflictingSocket.stdout).code,
          'rdp_route_x11_socket_owner_mismatch',
        );
      }
      writeFileSync(join(proc, 'net-unix'), normalSocketTable);

      const witness = observation.witness;
      const exactArgs = [
        file, 'terminate-rdp-route-session-exact',
        '--user', witness.routeUser,
        '--session-id', witness.sessionId,
        '--boot-id', witness.bootId,
        '--scope-invocation-id', witness.scopeInvocationId,
        '--cgroup-device', String(witness.cgroupDevice),
        '--cgroup-inode', String(witness.cgroupInode),
        '--leader-pid', String(witness.leaderPid),
        '--leader-start-ticks', String(witness.leaderStartTicks),
        '--x-server-pid', String(witness.xServerPid),
        '--x-server-start-ticks', String(witness.xServerStartTicks),
        '--display', witness.displayName,
        '--x11-socket-inode', String(witness.x11SocketInode),
      ];
      const mismatched = spawnSync('bash', exactArgs.map((value) =>
        value === String(witness.cgroupInode) ? String(witness.cgroupInode + 1) : value), {
        encoding: 'utf8', env: helperEnv,
      });
      assert.equal(mismatched.status, 0, mismatched.stderr);
      assert.equal(JSON.parse(mismatched.stdout).state, 'ownership_unproven');
      assert.equal(readFileSync(join(scope, 'cgroup.kill'), 'utf8'), '');

      const retainedScope = `${scope}-retained`;
      renameSync(scope, retainedScope);
      mkdirSync(scope);
      writeFileSync(join(scope, 'cgroup.kill'), '');
      writeFileSync(join(scope, 'cgroup.events'), 'populated 1\nfrozen 0\n');
      const rebound = spawnSync('bash', exactArgs, { encoding: 'utf8', env: helperEnv });
      assert.equal(rebound.status, 0, rebound.stderr);
      assert.equal(JSON.parse(rebound.stdout).state, 'ownership_unproven');
      assert.equal(readFileSync(join(scope, 'cgroup.kill'), 'utf8'), '');
      assert.equal(readFileSync(join(retainedScope, 'cgroup.kill'), 'utf8'), '');
      rmSync(scope, { recursive: true });
      renameSync(retainedScope, scope);

      const postOpenRebound = spawnSync('bash', exactArgs, {
        encoding: 'utf8',
        env: { ...helperEnv, FIXTURE_REBIND_AFTER_OPEN: '1' },
      });
      assert.equal(postOpenRebound.status, 0, postOpenRebound.stderr);
      assert.equal(JSON.parse(postOpenRebound.stdout).state, 'incomplete');
      assert.equal(readFileSync(join(postOpenRetainedScope, 'cgroup.kill'), 'utf8'), '1\n');
      assert.equal(readFileSync(join(scope, 'cgroup.kill'), 'utf8'), '');
      rmSync(scope, { recursive: true });
      renameSync(postOpenRetainedScope, scope);
      writeFileSync(join(scope, 'cgroup.kill'), '');
      writeFileSync(join(scope, 'cgroup.events'), 'populated 1\nfrozen 0\n');

      const malformedAfterKill = spawnSync('bash', exactArgs, {
        encoding: 'utf8',
        env: { ...helperEnv, FIXTURE_MALFORMED_AFTER_KILL: '1' },
      });
      assert.equal(malformedAfterKill.status, 0, malformedAfterKill.stderr);
      assert.equal(JSON.parse(malformedAfterKill.stdout).state, 'incomplete');
      writeFileSync(join(proc, '41002', 'stat'), `41002 (xrdp-chansrv) ${statTail} 5101\n`);
      writeFileSync(join(proc, '41003', 'stat'), `41003 (Xorg) ${statTail} 5102\n`);
      writeFileSync(join(scope, 'cgroup.kill'), '');
      writeFileSync(join(scope, 'cgroup.events'), 'populated 1\nfrozen 0\n');

      const terminate = spawnSync('bash', exactArgs, { encoding: 'utf8', env: helperEnv });
      assert.equal(terminate.status, 0, terminate.stderr);
      const stopped = JSON.parse(terminate.stdout);
      assert.equal(stopped.state, 'stopped', `${terminate.stdout}\n${terminate.stderr}`);
      const canonicalWitness = [
        'agent-browser.route-keeper-xrdp-ownership.v1',
        witness.bootId,
        witness.routeUser,
        witness.routeUid,
        witness.sessionId,
        witness.sessionService,
        witness.sessionScope,
        witness.scopeInvocationId,
        witness.cgroupPath,
        witness.cgroupDevice,
        witness.cgroupInode,
        witness.leaderPid,
        witness.leaderStartTicks,
        witness.xServerPid,
        witness.xServerStartTicks,
        witness.displayName,
        witness.x11SocketInode,
        '',
      ].join('\n');
      assert.equal(
        stopped.witnessDigest,
        createHash('sha256').update(canonicalWitness).digest('hex'),
      );
      assert.equal(readFileSync(join(scope, 'cgroup.kill'), 'utf8'), '1\n');
      assert.doesNotMatch(readFileSync(commandLog, 'utf8'), /terminate-user/);

      const rejected = spawnSync('bash', [file, 'observe-rdp-route-session', '--user', 'ecochran76'], {
        encoding: 'utf8',
        env: helperEnv,
      });
      assert.equal(rejected.status, 2);
      assert.match(rejected.stderr, /route user must be agent-browser-rdp/);
    } finally {
      rmSync(fixture, { recursive: true, force: true });
    }
  }
}

console.log('RDP route helper contract guard passed');
