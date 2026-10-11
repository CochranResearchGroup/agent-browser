#!/usr/bin/env node
/** Public CLI regression: a supported persistent profile is immediately plannable. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const binary = process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
assert(binary, 'Pin AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD to the candidate under test');
const home = mkdtempSync(join(tmpdir(), 'ab-created-profile-'));
const env = Object.fromEntries(Object.entries(process.env).filter(([key]) =>
  !key.startsWith('AGENT_BROWSER_') && !key.startsWith('XDG_')));
Object.assign(env, {
  HOME: home, AGENT_BROWSER_HOME: join(home, '.agent-browser'),
  AGENT_BROWSER_SOCKET_DIR: join(home, 'sockets'), XDG_RUNTIME_DIR: join(home, 'runtime'),
  XDG_CONFIG_HOME: join(home, 'config'), XDG_DATA_HOME: join(home, 'data'),
});
mkdirSync(env.XDG_RUNTIME_DIR, { mode: 0o700 });
function run(args) {
  const result = spawnSync(binary, ['--json', ...args], { env, encoding: 'utf8', timeout: 15000 });
  assert.equal(result.status, 0, `${args.join(' ')} failed: ${result.stdout}${result.stderr}`);
  return JSON.parse(result.stdout);
}
try {
  const profile = 'persistent-login';
  const created = run(['runtime', 'create', profile, '--browser-family', 'chromium']);
  assert.equal(created.created, true);
  const result = run(['--runtime-profile', profile, '--session', 'alice', '--session-name', 'alice',
    'service', 'access-plan', '--service-name', 'fixture', '--agent-name', 'codex',
    '--task-name', 'login', '--url', 'https://example.test/login']);
  assert.equal(result.success, true);
  const plan = result.data;
  assert.equal(plan.selectedProfile?.id, profile,
    'supported runtime creation must be immediately selected by access planning');
  assert.equal(plan.selectedProfile.userDataDir, created.userDataDir);
  assert.notEqual(plan.decision.recommendedAction, 'register_managed_profile_or_request_throwaway_browser');
  assert.equal(plan.decision.serviceRequest.request.runtimeProfile, profile);
  assert.equal(plan.decision.serviceRequest.request.sessionName, 'alice');
  const missing = run(['service', 'access-plan', '--runtime-profile', 'nonexistent-profile',
    '--service-name', 'fixture', '--agent-name', 'codex', '--task-name', 'login']);
  assert.equal(missing.data.selectedProfile, null, 'unknown names must not select another profile');
  console.log('Created persistent profile access planning passed');
} finally {
  rmSync(home, { recursive: true, force: true });
}
