#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  canControlViewStream,
  canEmbedViewStream,
  controlInputLabel,
  viewStreamCapabilityLabel,
  viewStreamLabel,
} from '../packages/dashboard/src/lib/service-view-streams.ts';

const rdpGatewayStream = {
  id: 'remote-headed-view',
  provider: 'rdp_gateway',
  controlInput: 'manual_attached_desktop',
  url: 'http://127.0.0.1:8080/rdp/session',
  readOnly: false,
};

assert.equal(viewStreamLabel(rdpGatewayStream), 'rdp gateway');
assert.equal(controlInputLabel(rdpGatewayStream), 'manual attached desktop');
assert.equal(viewStreamCapabilityLabel(rdpGatewayStream), 'rdp gateway / manual attached desktop');
assert.equal(canEmbedViewStream(rdpGatewayStream), true);
assert.equal(canControlViewStream(rdpGatewayStream), true);

assert.equal(
  canEmbedViewStream({
    provider: 'rdp_gateway',
    url: null,
  }),
  false,
);
assert.equal(
  canEmbedViewStream({
    provider: 'cdp_screencast',
    url: 'http://127.0.0.1:8080/cdp/session',
  }),
  false,
);
assert.equal(
  canControlViewStream({
    provider: 'rdp_gateway',
    readOnly: true,
    controlInput: 'manual_attached_desktop',
  }),
  false,
);
assert.equal(controlInputLabel({ readOnly: true }), 'view only');
assert.equal(viewStreamLabel({}), 'view stream');

console.log('Dashboard view stream contract smoke passed');
