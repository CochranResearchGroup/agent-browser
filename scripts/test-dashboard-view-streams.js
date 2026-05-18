#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  canEmbedViewStream,
  viewStreamLabel,
} from '../packages/dashboard/src/lib/service-view-streams.ts';

const rdpGatewayStream = {
  id: 'remote-headed-view',
  provider: 'rdp_gateway',
  url: 'http://127.0.0.1:8080/rdp/session',
  readOnly: false,
};

assert.equal(viewStreamLabel(rdpGatewayStream), 'rdp gateway');
assert.equal(canEmbedViewStream(rdpGatewayStream), true);

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
assert.equal(viewStreamLabel({}), 'view stream');

console.log('Dashboard view stream contract smoke passed');

