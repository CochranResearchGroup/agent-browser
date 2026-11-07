#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { closeSession, createSmokeContext, runCli, sendRawCommand } from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-worker-status-', sessionPrefix: 'worker-status' });
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_HEADLESS = '1';
for (const suffix of ['CONFIG', 'DATA', 'STATE', 'CACHE']) {
  const path = join(context.tempHome, suffix.toLowerCase());
  mkdirSync(path, { mode: 0o700 });
  context.env[`XDG_${suffix}_HOME`] = path;
}
context.env.XDG_RUNTIME_DIR = join(context.tempHome, 'runtime');
mkdirSync(context.env.XDG_RUNTIME_DIR, { mode: 0o700 });
const profile = join(context.tempHome, 'profile');
mkdirSync(profile, { mode: 0o700 });
const raw = (action, params = {}) => sendRawCommand({ ...context, session: 'runtime-host' }, {
  id: `worker-status-${action}`,
  action,
  _agentBrowserRuntimeLane: context.session,
  sessionName: context.session,
  ...params,
});
let opened;
try {
  await runCli(context, ['--json', '--session', context.session, 'stream', 'status']);
  const before = await raw('worker_status');
  assert.equal(before.success, true, JSON.stringify(before));
  opened = await raw('browser_session_open', { runtimeProfile: 'fixture', profileId: 'fixture', profile });
  assert.equal(opened.success, true, JSON.stringify(opened));
  const stateBefore = await raw('browser_session_status');
  const status = await raw('worker_status');
  assert.equal(status.success, true, JSON.stringify(status));
  assert.equal(status.data.worker_state, 'Ready');
  assert.equal(status.data.browser_health, 'NotStarted');
  assert.equal(status.browserSession, undefined, 'worker status reached a browser task');
  const stateAfter = await raw('browser_session_status');
  assert.deepEqual(stateAfter.data, stateBefore.data, 'worker status changed managed browser state');
  console.log('PASS: worker_status stays on the control plane with a managed task present');
} finally {
  try {
    if (opened?.success) {
      const closed = await raw('browser_session_close', { sessionId: opened.data.sessionId });
      assert.equal(closed.success, true, JSON.stringify(closed));
      assert.equal(closed.data.disposition, 'browser_closed');
    }
  } finally {
    await closeSession(context);
    context.cleanupTempHome();
  }
}
