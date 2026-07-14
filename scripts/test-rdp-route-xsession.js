#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const files = [
  'scripts/libexec/agent-browser-privileged-helper',
  'scripts/setup-rdp-guac-route-pool.sh',
];

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
  }
}

console.log('RDP route helper contract guard passed');
