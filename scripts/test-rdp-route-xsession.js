#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
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
      /^2026-06-23\.p44-route-desktop-v\d+$/,
      `${file} status-json should expose the P44 helper contract version`,
    );
    assert.equal(report.routeDesktopSession?.ready, true);
    assert.equal(report.routeDesktopSession?.state, 'browser_control_ready_template');
    assert.equal(report.routeDesktopSession?.terminalStartupDetected, false);
    assert.equal(report.routeDesktopSession?.startsWindowManager, true);
    assert.equal(report.routeDesktopSession?.keepsSessionAlive, true);
    assert.equal(report.displayAccess?.supportsFilesystemX11Socket, true);
    assert.equal(report.displayAccess?.supportsAbstractX11Socket, true);
    assert.equal(report.displayAccess?.boundedXhostTimeoutSeconds, 2);
    assert.equal(report.routeUserCredentialUpdate?.pamBypassed, true);
    assert.equal(report.routeUserCredentialUpdate?.cryptMethod, 'SHA512');
    assert.equal(report.routeUserCredentialUpdate?.shaRounds, 100000);
    assert.equal(
      report.managedChromeSandboxPolicy?.profileName,
      'agent-browser-managed-chrome',
    );
    assert.equal(typeof report.managedChromeSandboxPolicy?.loaded, 'boolean');

    const fixture = mkdtempSync(join(tmpdir(), 'agent-browser-route-helper-'));
    const commandLog = join(fixture, 'commands.log');
    try {
      writeFileSync(join(fixture, 'id'), '#!/bin/sh\nprintf "0\\n"\n', { mode: 0o755 });
      writeFileSync(
        join(fixture, 'getent'),
        '#!/bin/sh\nprintf "%s:x:2001:2001:agent-browser route-pool RDP session:/home/%s:/bin/bash\\n" "$2" "$2"\n',
        { mode: 0o755 },
      );
      writeFileSync(
        join(fixture, 'loginctl'),
        `#!/bin/sh\nprintf '%s\\n' "$*" >>${JSON.stringify(commandLog)}\n`,
        { mode: 0o755 },
      );
      writeFileSync(join(fixture, 'pgrep'), '#!/bin/sh\nexit 1\n', { mode: 0o755 });
      const terminate = spawnSync('bash', [file, 'terminate-rdp-route-session', '--user', 'agent-browser-rdp-dev-6'], {
        encoding: 'utf8',
        env: { ...process.env, PATH: `${fixture}:${process.env.PATH}` },
      });
      assert.equal(terminate.status, 0, terminate.stderr);
      assert.equal(
        readFileSync(commandLog, 'utf8'),
        'show-user agent-browser-rdp-dev-6 --property=State --value\nterminate-user agent-browser-rdp-dev-6\n',
      );

      const rejected = spawnSync('bash', [file, 'terminate-rdp-route-session', '--user', 'ecochran76'], {
        encoding: 'utf8',
        env: { ...process.env, PATH: `${fixture}:${process.env.PATH}` },
      });
      assert.equal(rejected.status, 2);
      assert.match(rejected.stderr, /route user must be agent-browser-rdp/);
      assert.equal(
        readFileSync(commandLog, 'utf8'),
        'show-user agent-browser-rdp-dev-6 --property=State --value\nterminate-user agent-browser-rdp-dev-6\n',
      );
    } finally {
      rmSync(fixture, { recursive: true, force: true });
    }
  }
}

console.log('RDP route helper contract guard passed');
