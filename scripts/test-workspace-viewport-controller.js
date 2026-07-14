#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  INITIAL_WORKSPACE_VIEWPORT_CONTROLLER_STATE,
  workspaceViewportControllerReducer,
  workspaceViewportTargetToken,
} from '../packages/dashboard/src/lib/workspace-viewport-controller.ts';

function target(overrides = {}) {
  return {
    browserId: 'browser-a',
    streamId: 'stream-a',
    streamUrl: 'https://dashboard.example.test/guacamole/#/client/a',
    routeId: 'route-a',
    mode: 'control',
    browserAvailable: true,
    ...overrides,
  };
}

function reduce(state, event) {
  return workspaceViewportControllerReducer(state, event);
}

const firstTarget = target();
const firstToken = workspaceViewportTargetToken(firstTarget);
assert.equal(
  firstToken,
  'browser=browser-a|stream=stream-a|url=https%3A%2F%2Fdashboard.example.test%2Fguacamole%2F%23%2Fclient%2Fa|route=route-a|mode=control',
);

let state = reduce(INITIAL_WORKSPACE_VIEWPORT_CONTROLLER_STATE, {
  type: 'target_changed',
  target: target({ browserAvailable: false }),
});
assert.equal(state.targetStatus, 'browser-unavailable');

state = reduce(state, {
  type: 'target_changed',
  target: target({ browserId: 'browser-b', streamId: 'stream-b', routeId: 'route-b' }),
});
assert.equal(state.targetStatus, 'available');
assert.deepEqual(state.preflight, { status: 'idle', message: '' });
assert.deepEqual(state.frame, { issue: null });
assert.deepEqual(state.recovery, { status: 'idle', action: null, message: '' });

const currentToken = state.targetToken;
const staleToken = firstToken;
assert.ok(currentToken);
assert.ok(staleToken);
assert.notEqual(currentToken, staleToken);

state = reduce(state, {
  type: 'preflight_started',
  targetToken: currentToken,
  message: 'Checking stream access.',
});
assert.deepEqual(state.preflight, { status: 'checking', message: 'Checking stream access.' });

state = reduce(state, {
  type: 'preflight_succeeded',
  targetToken: staleToken,
});
assert.deepEqual(
  state.preflight,
  { status: 'checking', message: 'Checking stream access.' },
  'late preflight success for an old token must be ignored',
);

state = reduce(state, {
  type: 'preflight_failed',
  targetToken: staleToken,
  status: 'error',
  message: 'old stream failed',
});
assert.deepEqual(
  state.preflight,
  { status: 'checking', message: 'Checking stream access.' },
  'late preflight failure for an old token must be ignored',
);

state = reduce(state, {
  type: 'preflight_failed',
  targetToken: currentToken,
  status: 'login-required',
  message: 'The remote stream rejected the current dashboard session.',
});
assert.deepEqual(state.preflight, {
  status: 'login-required',
  message: 'The remote stream rejected the current dashboard session.',
});

state = reduce(state, {
  type: 'preflight_succeeded',
  targetToken: currentToken,
});
assert.deepEqual(state.preflight, { status: 'ready', message: '' });

state = reduce(state, {
  type: 'frame_failed',
  targetToken: staleToken,
  kind: 'remote-disconnected',
  message: 'old frame disconnected',
});
assert.equal(state.frame.issue, null, 'late frame failure for an old token must be ignored');

state = reduce(state, {
  type: 'frame_failed',
  targetToken: currentToken,
  kind: 'taken-over',
  message: 'current frame was taken over',
});
assert.deepEqual(state.frame.issue, {
  kind: 'taken-over',
  message: 'current frame was taken over',
});

state = reduce(state, {
  type: 'recovery_started',
  targetToken: currentToken,
  action: 'controller-takeover',
  message: 'Requesting explicit controller takeover.',
});
assert.deepEqual(state.recovery, {
  status: 'pending',
  action: 'controller-takeover',
  message: 'Requesting explicit controller takeover.',
});

state = reduce(state, {
  type: 'recovery_accepted',
  targetToken: staleToken,
  message: 'old recovery accepted',
});
assert.deepEqual(
  state.recovery,
  {
    status: 'pending',
    action: 'controller-takeover',
    message: 'Requesting explicit controller takeover.',
  },
  'late recovery acceptance for an old token must be ignored',
);

state = reduce(state, {
  type: 'recovery_failed',
  targetToken: staleToken,
  message: 'old recovery failed',
});
assert.deepEqual(
  state.recovery,
  {
    status: 'pending',
    action: 'controller-takeover',
    message: 'Requesting explicit controller takeover.',
  },
  'late recovery failure for an old token must be ignored',
);

state = reduce(state, {
  type: 'recovery_accepted',
  targetToken: currentToken,
  message: 'Controller lease takeover was accepted.',
});
assert.deepEqual(state.recovery, {
  status: 'accepted',
  action: 'controller-takeover',
  message: 'Controller lease takeover was accepted.',
});

state = reduce(state, {
  type: 'recovery_started',
  targetToken: currentToken,
  action: 'route-refresh',
});
state = reduce(state, {
  type: 'recovery_failed',
  targetToken: currentToken,
  message: 'Browser route recovery failed.',
});
assert.deepEqual(state.recovery, {
  status: 'failed',
  action: 'route-refresh',
  message: 'Browser route recovery failed.',
});

console.log('workspace viewport controller tests passed');
