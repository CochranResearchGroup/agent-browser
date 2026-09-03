#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { deflateSync } from 'node:zlib';

export const P158_SYNTHETIC_FIXTURE_HOST = '127.0.0.1';
export const P158_SYNTHETIC_FIXTURE_PORT = 19058;
export const P158_SYNTHETIC_FIXTURE_ID = 'p158-synthetic-visual-v1';
export const P158_SYNTHETIC_PAGE_MARKER = 'P158-SYNTHETIC-VISUAL-FIXTURE-V1';
export const P158_SYNTHETIC_VIEWPORT = Object.freeze({ width: 1440, height: 1000 });
export const P158_PIXEL_REGION = Object.freeze({
  id: 'primary-solid-pixel-region',
  x: 240,
  y: 120,
  width: 960,
  height: 320,
  rgba: Object.freeze([18, 92, 142, 255]),
  assetPath: '/pixel-marker.png',
});

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCRIPT_RELATIVE_PATH = 'scripts/p158-synthetic-visual-fixture.js';
const FIXED_WS_MESSAGE = 'p158-synthetic-websocket-ready-v1';

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(',')}}`;
  }
  return JSON.stringify(value);
}

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function pngChunk(type, data) {
  const typeBytes = Buffer.from(type, 'ascii');
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const checksum = Buffer.alloc(4);
  checksum.writeUInt32BE(crc32(Buffer.concat([typeBytes, data])));
  return Buffer.concat([length, typeBytes, data, checksum]);
}

export function buildPixelMarkerPng(region = P158_PIXEL_REGION) {
  const { width, height, rgba } = region;
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1) {
    throw new TypeError('pixel region width and height must be positive integers');
  }
  if (!Array.isArray(rgba) || rgba.length !== 4 || rgba.some((value) => !Number.isInteger(value) || value < 0 || value > 255)) {
    throw new TypeError('pixel region rgba must contain four byte values');
  }

  const scanline = Buffer.alloc(1 + width * 4);
  const rawRgba = Buffer.alloc(width * height * 4);
  scanline[0] = 0;
  for (let offset = 1; offset < scanline.length; offset += 4) {
    scanline.set(rgba, offset);
  }
  for (let offset = 0; offset < rawRgba.length; offset += 4) rawRgba.set(rgba, offset);
  const raw = Buffer.concat(Array.from({ length: height }, () => scanline));
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;
  return {
    bytes: Buffer.concat([
      Buffer.from('89504e470d0a1a0a', 'hex'),
      pngChunk('IHDR', header),
      pngChunk('IDAT', deflateSync(raw, { level: 9 })),
      pngChunk('IEND', Buffer.alloc(0)),
    ]),
    rawRgbaSha256: sha256(rawRgba),
  };
}

export function buildSyntheticFixtureHtml() {
  return `<!doctype html>
<html lang="en" data-p158-fixture="${P158_SYNTHETIC_FIXTURE_ID}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=1440, initial-scale=1">
  <meta name="p158-page-marker" content="${P158_SYNTHETIC_PAGE_MARKER}">
  <title>${P158_SYNTHETIC_PAGE_MARKER}</title>
  <style>
    * { box-sizing: border-box; }
    html, body { width: 1440px; min-width: 1440px; height: 1000px; min-height: 1000px; margin: 0; overflow: hidden; }
    body { background: #f2f5f7; color: #15202b; font: 18px/1.35 system-ui, sans-serif; }
    #fixture { position: relative; width: 1440px; height: 1000px; }
    #page-marker { position: absolute; left: 24px; top: 18px; font: 700 20px/1 monospace; }
    #pixel-marker { position: absolute; left: 240px; top: 120px; width: 960px; height: 320px; image-rendering: pixelated; }
    #secondary-solid-region { position: absolute; left: 80px; top: 480px; width: 1280px; height: 96px; background: #df7226; }
    #controls { position: absolute; left: 80px; top: 620px; width: 1280px; display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; }
    button, input, a { min-height: 44px; font: inherit; }
    button:focus-visible, input:focus-visible, a:focus-visible { outline: 5px solid #7a21c7; outline-offset: 2px; }
    #overflow-surface { height: 76px; overflow: auto; border: 2px solid #15202b; white-space: nowrap; }
    #overflow-surface span { display: inline-block; width: 1800px; }
    #status { grid-column: 1 / -1; min-height: 32px; padding: 4px; background: #fff; }
    iframe { width: 100%; height: 76px; border: 2px solid #15202b; }
    dialog { border: 4px solid #15202b; width: 480px; }
    dialog::backdrop { background: rgb(0 0 0 / 45%); }
    @media (prefers-reduced-motion: reduce) { *, *::before, *::after { scroll-behavior: auto !important; animation: none !important; transition: none !important; } }
  </style>
</head>
<body>
  <main id="fixture" aria-labelledby="page-marker">
    <h1 id="page-marker">${P158_SYNTHETIC_PAGE_MARKER}</h1>
    <img id="pixel-marker" src="/pixel-marker.png" width="960" height="320" alt="Deterministic solid pixel marker">
    <div id="secondary-solid-region" aria-label="Secondary deterministic solid pixel region"></div>
    <section id="controls" aria-label="Synthetic interaction surfaces">
      <button id="focus-action" type="button">Focus action</button>
      <button id="modal-action" type="button">Open prompt-like dialog</button>
      <button id="popup-action" type="button">Open popup</button>
      <a id="redirect-action" href="/redirect">Redirect action</a>
      <form id="safe-form" action="/form-result" method="get">
        <label for="safe-text">Synthetic text</label>
        <input id="safe-text" name="safe-text" value="fixture-value" autocomplete="off">
        <button type="submit">Submit form</button>
      </form>
      <a id="error-action" href="/error-action">Error action</a>
      <button id="websocket-action" type="button">Connect WebSocket</button>
      <button id="reconnect-action" type="button">Reconnect WebSocket</button>
      <iframe id="fixture-frame" title="Synthetic iframe" src="/frame"></iframe>
      <div id="overflow-surface" tabindex="0"><span>Deterministic horizontal overflow surface for keyboard and viewport checks.</span></div>
      <output id="status" role="status" aria-live="polite">ready</output>
    </section>
    <dialog id="prompt-like" aria-labelledby="dialog-title">
      <h2 id="dialog-title">Synthetic prompt-like dialog</h2>
      <label for="dialog-input">Public synthetic value</label>
      <input id="dialog-input" value="fixture-value" autocomplete="off">
      <button id="dialog-accept" type="button">Accept</button>
      <button id="dialog-cancel" type="button">Cancel</button>
    </dialog>
  </main>
  <script>
    const status = document.getElementById('status');
    const dialog = document.getElementById('prompt-like');
    let socket;
    let modalReturnTarget;
    const connect = () => {
      if (socket) socket.close();
      const scheme = location.protocol === 'https:' ? 'wss:' : 'ws:';
      socket = new WebSocket(scheme + '//' + location.host + '/ws');
      socket.addEventListener('open', () => { status.value = 'websocket-open'; });
      socket.addEventListener('message', (event) => { status.value = event.data; });
      socket.addEventListener('close', () => { status.value = 'websocket-closed'; });
      socket.addEventListener('error', () => { status.value = 'websocket-error'; });
    };
    document.getElementById('focus-action').addEventListener('click', (event) => { status.value = 'focus-action'; event.currentTarget.focus(); });
    document.getElementById('modal-action').addEventListener('click', (event) => { modalReturnTarget = event.currentTarget; dialog.showModal(); document.getElementById('dialog-input').focus(); });
    for (const id of ['dialog-accept', 'dialog-cancel']) document.getElementById(id).addEventListener('click', () => { dialog.close(); status.value = id; modalReturnTarget.focus(); });
    document.getElementById('popup-action').addEventListener('click', () => { window.open('/popup', 'p158-synthetic-popup', 'width=640,height=480'); });
    document.getElementById('websocket-action').addEventListener('click', connect);
    document.getElementById('reconnect-action').addEventListener('click', connect);
  </script>
</body>
</html>`;
}

const FRAME_HTML = `<!doctype html><html lang="en"><body><button id="frame-action">Synthetic iframe action</button><output id="frame-status">ready</output><script>document.getElementById('frame-action').onclick=()=>{document.getElementById('frame-status').value='activated'}</script></body></html>`;
const POPUP_HTML = `<!doctype html><html lang="en"><title>P158 synthetic popup</title><body><h1>P158 synthetic popup</h1><button id="popup-close" onclick="window.close()">Close popup</button></body></html>`;

export function buildSyntheticVisualAttestation({ sourceBytes = readFileSync(SCRIPT_PATH) } = {}) {
  const html = Buffer.from(buildSyntheticFixtureHtml());
  const pixel = buildPixelMarkerPng();
  const receipt = {
    schemaVersion: 'agent-browser.p158-synthetic-visual-attestation.v1',
    planId: 'P158',
    fixtureId: P158_SYNTHETIC_FIXTURE_ID,
    syntheticOnly: true,
    forbiddenPrivateFieldsExcluded: true,
    source: { path: SCRIPT_RELATIVE_PATH, sha256: sha256(sourceBytes) },
    viewport: P158_SYNTHETIC_VIEWPORT,
    pageMarker: P158_SYNTHETIC_PAGE_MARKER,
    loopback: {
      host: P158_SYNTHETIC_FIXTURE_HOST,
      port: P158_SYNTHETIC_FIXTURE_PORT,
      origin: `http://${P158_SYNTHETIC_FIXTURE_HOST}:${P158_SYNTHETIC_FIXTURE_PORT}`,
    },
    documentSha256: sha256(html),
    pixelRegions: [{
      ...P158_PIXEL_REGION,
      rgba: [...P158_PIXEL_REGION.rgba],
      assetSha256: sha256(pixel.bytes),
      rawRgbaSha256: pixel.rawRgbaSha256,
      captureRule: 'exact-region-pixels-at-1440x1000',
    }],
    endpoints: ['/fixture', '/frame', '/popup', '/form-result', '/redirect', '/error-action', '/pixel-marker.png', '/manifest.json', '/healthz', '/ws'],
    redaction: {
      policy: 'synthetic-allowlist-only',
      credentialsCaptured: false,
      cookiesCaptured: false,
      requestBodiesCaptured: false,
      externalContentLoaded: false,
    },
  };
  return { ...receipt, redactionReceiptSha256: sha256(canonicalJson(receipt)) };
}

function assertAbsolutePath(value, label) {
  if (typeof value !== 'string' || !value.startsWith('/')) throw new TypeError(`${label} must be an absolute path`);
}

function systemdEscape(value) {
  if (/[ \n\r]/u.test(value)) throw new TypeError('systemd value contains a forbidden character');
  return value.replaceAll('\\', '\\\\').replaceAll('"', '\\"');
}

export function buildSystemdUserLaunchPlan({
  nodeExecutable = process.execPath,
  scriptPath = SCRIPT_PATH,
  workingDirectory = resolve(dirname(SCRIPT_PATH), '..'),
} = {}) {
  assertAbsolutePath(nodeExecutable, 'nodeExecutable');
  assertAbsolutePath(scriptPath, 'scriptPath');
  assertAbsolutePath(workingDirectory, 'workingDirectory');
  const unitName = 'agent-browser-p158-synthetic-visual-fixture.service';
  const unit = `[Unit]\nDescription=Plan 0158 synthetic visual fixture\nAfter=network.target\n\n[Service]\nType=simple\nWorkingDirectory=${systemdEscape(workingDirectory)}\nExecStart=${systemdEscape(nodeExecutable)} ${systemdEscape(scriptPath)} serve\nRestart=no\nNoNewPrivileges=true\nPrivateTmp=true\nProtectSystem=strict\nProtectHome=read-only\n\n[Install]\nWantedBy=default.target\n`;
  return {
    schemaVersion: 'agent-browser.p158-systemd-user-launch-plan.v1',
    planId: 'P158',
    fixtureId: P158_SYNTHETIC_FIXTURE_ID,
    apply: false,
    scope: 'systemd-user',
    unitName,
    destination: `%h/.config/systemd/user/${unitName}`,
    bind: { host: P158_SYNTHETIC_FIXTURE_HOST, port: P158_SYNTHETIC_FIXTURE_PORT },
    unit,
    unitSha256: sha256(unit),
    activationSteps: ['install the reviewed unit text', 'systemctl --user daemon-reload', `systemctl --user enable --now ${unitName}`],
  };
}

function send(response, status, contentType, body, extraHeaders = {}) {
  const bytes = Buffer.isBuffer(body) ? body : Buffer.from(body);
  response.writeHead(status, {
    'Cache-Control': 'no-store',
    'Content-Length': bytes.length,
    'Content-Security-Policy': "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self'; frame-src 'self'; connect-src 'self' ws: wss:; form-action 'self'; base-uri 'none'",
    'Content-Type': contentType,
    'X-Content-Type-Options': 'nosniff',
    'X-P158-Synthetic-Fixture': P158_SYNTHETIC_FIXTURE_ID,
    ...extraHeaders,
  });
  response.end(bytes);
}

function websocketAccept(key) {
  return createHash('sha1').update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`).digest('base64');
}

function websocketTextFrame(text) {
  const payload = Buffer.from(text);
  if (payload.length >= 126) throw new RangeError('fixture WebSocket message is too long');
  return Buffer.concat([Buffer.from([0x81, payload.length]), payload]);
}

export function createSyntheticVisualFixtureServer({
  host = P158_SYNTHETIC_FIXTURE_HOST,
  port = P158_SYNTHETIC_FIXTURE_PORT,
  allowEphemeralTestPort = false,
} = {}) {
  if (host !== P158_SYNTHETIC_FIXTURE_HOST) throw new TypeError('synthetic fixture must bind to explicit IPv4 loopback');
  if (!Number.isInteger(port) || port < 0 || port > 65535 || (port === 0 && !allowEphemeralTestPort)) {
    throw new TypeError('synthetic fixture requires the fixed development port; port zero is test-only');
  }
  if (port !== P158_SYNTHETIC_FIXTURE_PORT && !(port === 0 && allowEphemeralTestPort)) {
    throw new TypeError(`synthetic fixture port must be ${P158_SYNTHETIC_FIXTURE_PORT}`);
  }

  const html = buildSyntheticFixtureHtml();
  const pixel = buildPixelMarkerPng();
  const upgradedSockets = new Set();
  const server = createServer((request, response) => {
    const requestUrl = new URL(request.url ?? '/', `http://${P158_SYNTHETIC_FIXTURE_HOST}`);
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      send(response, 405, 'application/json; charset=utf-8', JSON.stringify({ error: 'method_not_allowed', synthetic: true }));
      return;
    }
    if (requestUrl.pathname === '/' || requestUrl.pathname === '/fixture') send(response, 200, 'text/html; charset=utf-8', html);
    else if (requestUrl.pathname === '/frame') send(response, 200, 'text/html; charset=utf-8', FRAME_HTML);
    else if (requestUrl.pathname === '/popup') send(response, 200, 'text/html; charset=utf-8', POPUP_HTML);
    else if (requestUrl.pathname === '/form-result') send(response, 200, 'text/html; charset=utf-8', `<!doctype html><title>Form result</title><output id="form-result">${requestUrl.searchParams.has('safe-text') ? 'submitted' : 'missing'}</output>`);
    else if (requestUrl.pathname === '/redirect') send(response, 302, 'text/plain; charset=utf-8', 'synthetic redirect', { Location: '/fixture?via=redirect' });
    else if (requestUrl.pathname === '/error-action') send(response, 503, 'application/json; charset=utf-8', JSON.stringify({ error: 'synthetic_error_action', synthetic: true }));
    else if (requestUrl.pathname === '/pixel-marker.png') send(response, 200, 'image/png', pixel.bytes);
    else if (requestUrl.pathname === '/manifest.json') send(response, 200, 'application/json; charset=utf-8', `${canonicalJson(buildSyntheticVisualAttestation())}\n`);
    else if (requestUrl.pathname === '/healthz') send(response, 200, 'application/json; charset=utf-8', JSON.stringify({ fixtureId: P158_SYNTHETIC_FIXTURE_ID, state: 'ready', synthetic: true }));
    else send(response, 404, 'application/json; charset=utf-8', JSON.stringify({ error: 'not_found', synthetic: true }));
  });

  server.on('upgrade', (request, socket) => {
    if (request.url !== '/ws' || request.headers.upgrade?.toLowerCase() !== 'websocket' || typeof request.headers['sec-websocket-key'] !== 'string') {
      socket.end('HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n');
      return;
    }
    upgradedSockets.add(socket);
    socket.once('close', () => upgradedSockets.delete(socket));
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${websocketAccept(request.headers['sec-websocket-key'])}\r\n\r\n`);
    socket.write(websocketTextFrame(FIXED_WS_MESSAGE));
  });

  return {
    server,
    listen: () => new Promise((resolveListen, reject) => {
      server.once('error', reject);
      server.listen(port, host, () => {
        server.off('error', reject);
        resolveListen(server.address());
      });
    }),
    close: () => new Promise((resolveClose, reject) => {
      for (const socket of upgradedSockets) socket.destroy();
      server.close((error) => error ? reject(error) : resolveClose());
    }),
  };
}

async function main() {
  const command = process.argv[2] ?? 'serve';
  if (command === 'attest') {
    process.stdout.write(`${canonicalJson(buildSyntheticVisualAttestation())}\n`);
    return;
  }
  if (command === 'plan-systemd-user') {
    process.stdout.write(`${canonicalJson(buildSystemdUserLaunchPlan())}\n`);
    return;
  }
  if (command !== 'serve') throw new Error(`unknown command: ${command}`);
  const fixture = createSyntheticVisualFixtureServer();
  await fixture.listen();
  process.stdout.write(`P158 synthetic fixture ready at http://${P158_SYNTHETIC_FIXTURE_HOST}:${P158_SYNTHETIC_FIXTURE_PORT}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  main().catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
