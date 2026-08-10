#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const cargoSafe = path.join(repoRoot, 'scripts/ci/cargo-safe.sh');

const commands = [
  {
    label: 'wrong flag placement parser fixture',
    command: cargoSafe,
    args: [
      'test',
      '--manifest-path',
      'cli/Cargo.toml',
      'test_remote_view_open_preserves_global_flags_after_subcommand',
      '--',
      '--test-threads=1',
    ],
  },
  {
    label: 'named-session route pool mismatch fixture',
    command: cargoSafe,
    args: [
      'test',
      '--manifest-path',
      'cli/Cargo.toml',
      'test_remote_view_route_checkout_reports_route_pool_unavailable',
      '--',
      '--test-threads=1',
    ],
  },
  {
    label: 'same-owner route-pool repeat fixture',
    command: cargoSafe,
    args: [
      'test',
      '--manifest-path',
      'cli/Cargo.toml',
      'test_remote_view_route_checkout_reuses_checked_out_same_owner',
      '--',
      '--test-threads=1',
    ],
  },
  {
    label: 'profile lock known-owner fixture',
    command: cargoSafe,
    args: [
      'test',
      '--manifest-path',
      'cli/Cargo.toml',
      'test_locked_profile_message_reports_runtime_and_service_owner',
      '--',
      '--test-threads=1',
    ],
  },
  {
    label: 'direct remote-headed route-handoff audit fixture',
    command: process.execPath,
    args: ['scripts/test-route-handoff-audit.js'],
  },
  {
    label: 'dashboard missing-proof and terminal-only row fixture',
    command: process.execPath,
    args: ['--no-warnings', '--experimental-strip-types', 'scripts/test-dashboard-workspace-nodes.js'],
  },
  {
    label: 'RDP route xsession no-terminal fixture',
    command: process.execPath,
    args: ['scripts/test-rdp-route-xsession.js'],
  },
  {
    label: 'RDP Guacamole Postgres hardening fixture',
    command: process.execPath,
    args: ['scripts/test-rdp-guac-postgres-hardening.js'],
  },
];

for (const item of commands) {
  const result = spawnSync(item.command, item.args, {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: 'pipe',
  });
  if (result.status !== 0) {
    console.error(`Slice H no-launch gate failed: ${item.label}`);
    console.error(`$ ${item.command} ${item.args.join(' ')}`);
    process.stdout.write(result.stdout);
    process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }
  console.log(`passed: ${item.label}`);
}

console.log('route confusion no-launch gates passed');
