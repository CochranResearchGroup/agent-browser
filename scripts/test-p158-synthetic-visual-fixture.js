#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { connect } from 'node:net';
import { inflateSync } from 'node:zlib';
import {
  P158_PIXEL_REGION,
  P158_SYNTHETIC_FIXTURE_HOST,
  P158_SYNTHETIC_FIXTURE_ID,
  P158_SYNTHETIC_FIXTURE_PORT,
  P158_SYNTHETIC_PAGE_MARKER,
  P158_SYNTHETIC_VIEWPORT,
  buildPixelMarkerPng,
  buildSyntheticFixtureHtml,
  buildSyntheticVisualAttestation,
  buildSystemdUserLaunchPlan,
  createSyntheticVisualFixtureServer,
} from './p158-synthetic-visual-fixture.js';

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

function parsePng(bytes) {
  assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  let offset = 8;
  let width;
  let height;
  const compressed = [];
  while (offset < bytes.length) {
    const length = bytes.readUInt32BE(offset);
    const type = bytes.subarray(offset + 4, offset + 8).toString('ascii');
    const data = bytes.subarray(offset + 8, offset + 8 + length);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      assert.equal(data[8], 8);
      assert.equal(data[9], 6);
    }
    if (type === 'IDAT') compressed.push(data);
    offset += 12 + length;
  }
  return { width, height, scanlines: inflateSync(Buffer.concat(compressed)) };
}

function rawWebSocketProbe(port) {
  return new Promise((resolve, reject) => {
    const socket = connect({ host: P158_SYNTHETIC_FIXTURE_HOST, port });
    const chunks = [];
    socket.once('error', reject);
    socket.once('connect', () => socket.write([
      'GET /ws HTTP/1.1',
      `Host: ${P158_SYNTHETIC_FIXTURE_HOST}:${port}`,
      'Upgrade: websocket',
      'Connection: Upgrade',
      'Sec-WebSocket-Key: MDEyMzQ1Njc4OWFiY2RlZg==',
      'Sec-WebSocket-Version: 13',
      '',
      '',
    ].join('\r\n')));
    socket.on('data', (chunk) => {
      chunks.push(chunk);
      const bytes = Buffer.concat(chunks);
      const boundary = bytes.indexOf('\r\n\r\n');
      if (boundary < 0 || bytes.length < boundary + 6) return;
      const headers = bytes.subarray(0, boundary).toString('utf8');
      const frame = bytes.subarray(boundary + 4);
      assert.match(headers, /^HTTP\/1\.1 101 Switching Protocols/);
      assert.equal(frame[0], 0x81);
      const length = frame[1] & 0x7f;
      assert.equal(frame.subarray(2, 2 + length).toString('utf8'), 'p158-synthetic-websocket-ready-v1');
      socket.destroy();
      resolve();
    });
  });
}

assert.equal(P158_SYNTHETIC_FIXTURE_HOST, '127.0.0.1');
assert.equal(P158_SYNTHETIC_FIXTURE_PORT, 19058);
assert.deepEqual(P158_SYNTHETIC_VIEWPORT, { width: 1440, height: 1000 });
assert.throws(() => createSyntheticVisualFixtureServer({ host: '0.0.0.0' }), /loopback/);
assert.throws(() => createSyntheticVisualFixtureServer({ port: 0 }), /test-only/);
assert.throws(() => createSyntheticVisualFixtureServer({ port: 19059 }), /must be 19058/);

const html = buildSyntheticFixtureHtml();
assert.equal(html, buildSyntheticFixtureHtml());
assert.match(html, new RegExp(P158_SYNTHETIC_PAGE_MARKER));
for (const surface of ['pixel-marker', 'focus-action', 'modal-action', 'popup-action', 'redirect-action', 'safe-form', 'error-action', 'websocket-action', 'reconnect-action', 'fixture-frame', 'overflow-surface']) {
  assert.match(html, new RegExp(`id="${surface}"`), `missing synthetic surface ${surface}`);
}
assert.doesNotMatch(html, /https?:\/\/(?!127\.0\.0\.1)/);
assert.doesNotMatch(html, /password|authorization|bearer|cookie/i);
assert.match(html, /prefers-reduced-motion/);
assert.match(html, /modalReturnTarget\.focus\(\)/);

const pixel = buildPixelMarkerPng();
assert.deepEqual(pixel.bytes, buildPixelMarkerPng().bytes);
const decoded = parsePng(pixel.bytes);
assert.equal(decoded.width, P158_PIXEL_REGION.width);
assert.equal(decoded.height, P158_PIXEL_REGION.height);
const expectedLine = Buffer.alloc(1 + decoded.width * 4);
for (let offset = 1; offset < expectedLine.length; offset += 4) expectedLine.set(P158_PIXEL_REGION.rgba, offset);
for (let row = 0; row < decoded.height; row += 1) {
  assert.deepEqual(decoded.scanlines.subarray(row * expectedLine.length, (row + 1) * expectedLine.length), expectedLine);
}

const source = readFileSync(new URL('./p158-synthetic-visual-fixture.js', import.meta.url));
const attestation = buildSyntheticVisualAttestation();
assert.equal(attestation.fixtureId, P158_SYNTHETIC_FIXTURE_ID);
assert.equal(attestation.syntheticOnly, true);
assert.equal(attestation.forbiddenPrivateFieldsExcluded, true);
assert.equal(attestation.source.sha256, sha256(source));
assert.equal(attestation.documentSha256, sha256(Buffer.from(html)));
assert.equal(attestation.pixelRegions[0].assetSha256, sha256(pixel.bytes));
const { redactionReceiptSha256, ...unsignedAttestation } = attestation;
assert.equal(redactionReceiptSha256, sha256(canonicalJson(unsignedAttestation)));
assert.match(redactionReceiptSha256, /^[a-f0-9]{64}$/);
assert.equal(attestation.redaction.credentialsCaptured, false);
assert.equal(attestation.redaction.cookiesCaptured, false);
assert.equal(attestation.redaction.requestBodiesCaptured, false);

const launchPlan = buildSystemdUserLaunchPlan();
assert.equal(launchPlan.apply, false);
assert.equal(launchPlan.scope, 'systemd-user');
assert.deepEqual(launchPlan.bind, { host: '127.0.0.1', port: 19058 });
assert.match(launchPlan.unit, /^\[Unit\]/);
assert.match(launchPlan.unit, /Restart=no/);
assert.match(launchPlan.unit, /NoNewPrivileges=true/);
assert.match(launchPlan.unit, /ProtectSystem=strict/);
assert.doesNotMatch(launchPlan.unit, /Environment=|password|credential|secret/i);
assert.equal(launchPlan.unitSha256, sha256(launchPlan.unit));
assert.throws(() => buildSystemdUserLaunchPlan({ nodeExecutable: 'node' }), /absolute path/);

const fixture = createSyntheticVisualFixtureServer({ port: 0, allowEphemeralTestPort: true });
try {
  const address = await fixture.listen();
  assert.equal(address.address, P158_SYNTHETIC_FIXTURE_HOST);
  const origin = `http://${P158_SYNTHETIC_FIXTURE_HOST}:${address.port}`;
  const page = await fetch(`${origin}/fixture`);
  assert.equal(page.status, 200);
  assert.equal(page.headers.get('x-p158-synthetic-fixture'), P158_SYNTHETIC_FIXTURE_ID);
  assert.equal(await page.text(), html);

  const marker = await fetch(`${origin}/pixel-marker.png`);
  assert.equal(marker.status, 200);
  assert.deepEqual(Buffer.from(await marker.arrayBuffer()), pixel.bytes);

  const redirect = await fetch(`${origin}/redirect`, { redirect: 'manual' });
  assert.equal(redirect.status, 302);
  assert.equal(redirect.headers.get('location'), '/fixture?via=redirect');
  assert.equal((await fetch(`${origin}/frame`)).status, 200);
  assert.equal((await fetch(`${origin}/popup`)).status, 200);
  assert.equal((await fetch(`${origin}/form-result?safe-text=fixture-value`)).status, 200);
  assert.equal((await fetch(`${origin}/error-action`)).status, 503);
  assert.deepEqual(await (await fetch(`${origin}/healthz`)).json(), { fixtureId: P158_SYNTHETIC_FIXTURE_ID, state: 'ready', synthetic: true });
  assert.deepEqual(await (await fetch(`${origin}/manifest.json`)).json(), attestation);
  assert.equal((await fetch(`${origin}/missing`)).status, 404);
  assert.equal((await fetch(`${origin}/fixture`, { method: 'POST' })).status, 405);
  await rawWebSocketProbe(address.port);
} finally {
  await fixture.close();
}

process.stdout.write('Plan 0158 synthetic visual fixture provider-free checks passed\n');
