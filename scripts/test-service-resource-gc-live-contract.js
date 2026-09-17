#!/usr/bin/env node

import assert from 'node:assert/strict';

import { assertTerminalRetirementState } from './service-resource-gc-live-contract.js';

const browserId = 'session:managed-resource-gc-contract';
const plan = { browserId };
const receipt = { planId: 'plan-contract' };
const terminalLifecycle = {
  lifecycleState: 'terminal',
  cleanupObligationState: 'satisfied',
};

function terminalState(overrides = {}) {
  return {
    browsers: {},
    browserProcessIdentities: {},
    runtimeOwnerRegistry: {
      lifecycleRecords: { [browserId]: terminalLifecycle },
    },
    abandonedBrowserRetirements: {
      'plan-contract': { plan, receipt },
    },
    ...overrides,
  };
}

assert.doesNotThrow(() => assertTerminalRetirementState(terminalState(), browserId));

assert.throws(
  () =>
    assertTerminalRetirementState(
      terminalState({
        browsers: {
          [browserId]: { health: 'process_exited', pid: null },
        },
      }),
      browserId,
    ),
  /terminal browser operational row remains/,
);

assert.throws(
  () =>
    assertTerminalRetirementState(
      terminalState({
        runtimeOwnerRegistry: { lifecycleRecords: {} },
      }),
      browserId,
    ),
  /terminal lifecycle is incoherent/,
);

assert.throws(
  () =>
    assertTerminalRetirementState(
      terminalState({
        abandonedBrowserRetirements: {},
      }),
      browserId,
    ),
  /Expected one terminal retirement receipt/,
);

console.log('service-resource-gc-live-contract: ok');
