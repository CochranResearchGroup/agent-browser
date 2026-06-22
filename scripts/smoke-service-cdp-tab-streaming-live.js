#!/usr/bin/env node

import { createHash, randomBytes } from 'node:crypto';
import { request } from 'node:http';
import { createConnection } from 'node:net';

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-cdp-tab-stream-',
  sessionPrefix: 'cdp-tab-stream',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'CdpTabStreamingSmoke';
const agentName = 'smoke-agent';
const browserId = `session:${session}`;
const pageA = smokePageUrl('CDP Stream A', '#d7f5ff', '#003044');
const pageB = smokePageUrl('CDP Stream B', '#ffe8cf', '#482100');
const timeout = setTimeout(() => {
  fail('Timed out waiting for CDP tab streaming live smoke to complete');
}, 180000);

let streamPort;
let ws;

async function cleanup() {
  clearTimeout(timeout);
  try {
    ws?.close();
  } catch {
    // Best effort only.
  }
  await closeSession(context);
  if (process.env.AGENT_BROWSER_SMOKE_PRESERVE === '1') {
    console.error(`Preserved smoke temp home: ${context.tempHome}`);
  } else {
    context.cleanupTempHome();
  }
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
}

function smokePageUrl(title, background, foreground) {
  const html = [
    '<!doctype html>',
    '<html>',
    `<head><title>${title}</title>`,
    '<style>',
    `html,body{margin:0;width:100%;height:100%;background:${background};color:${foreground};font:48px sans-serif;}`,
    'main{display:grid;place-items:center;min-height:100vh;}',
    '</style></head>',
    `<body><main><h1>${title}</h1></main></body>`,
    '</html>',
  ].join('');
  return `data:text/html;charset=utf-8,${encodeURIComponent(html)}`;
}

function frameHash(frame) {
  return createHash('sha256').update(frame.data || '').digest('hex');
}

async function serviceRequest(action, params, taskName) {
  let response;
  try {
    response = await httpJsonWithTimeout(streamPort, 'POST', '/api/service/request', {
      action,
      serviceName,
      agentName,
      taskName,
      params,
      jobTimeoutMs: 60000,
    }, 90000);
  } catch (err) {
    throw new Error(`${action} request '${taskName}' failed: ${err.message}`);
  }
  assert(response.success === true, `${action} failed: ${JSON.stringify(response)}`);
  return response;
}

function httpJsonWithTimeout(port, method, path, body, timeoutMs) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = request(
      {
        host: '127.0.0.1',
        port,
        method,
        path,
        headers: rawBody
          ? {
              'content-type': 'application/json',
              'content-length': Buffer.byteLength(rawBody),
            }
          : undefined,
      },
      (res) => {
        let text = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          text += chunk;
        });
        res.on('end', () => {
          try {
            const parsed = JSON.parse(text);
            if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
              reject(new Error(`HTTP ${method} ${path} returned ${res.statusCode}: ${text}`));
              return;
            }
            resolve(parsed);
          } catch (err) {
            reject(new Error(`Failed to parse HTTP ${method} ${path}: ${err.message}\n${text}`));
          }
        });
      },
    );
    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`HTTP ${method} ${path} timed out after ${timeoutMs}ms`));
    });
    req.on('error', reject);
    if (rawBody) req.write(rawBody);
    req.end();
  });
}

async function serviceStatus(label) {
  const result = await runCli(context, ['--json', '--session', session, 'service', 'status'], 60000);
  const status = parseJsonOutput(result.stdout, label);
  assert(status.success === true, `${label} failed: ${result.stdout}${result.stderr}`);
  return status.data?.service_state;
}

async function waitForServiceTab(title, label, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
  const encodedTitle = encodeURIComponent(title);
  while (Date.now() < deadline) {
    const state = await serviceStatus(label);
    const tab = Object.values(state?.tabs ?? {}).find((candidate) =>
      candidate.title === title || String(candidate.url || '').includes(encodedTitle),
    );
    if (tab?.targetId) return { state, tab };
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  const state = await serviceStatus(`${label} timeout diagnostics`);
  throw new Error(
    `Timed out waiting for service tab '${title}'. ` +
    `tabs=${JSON.stringify(state?.tabs ?? {})} browsers=${JSON.stringify(state?.browsers ?? {})}`,
  );
}

function primaryCdpStream(browser) {
  return browser?.viewStreams?.find((stream) => stream.provider === 'cdp_screencast') ?? null;
}

async function activeUrl() {
  const response = await httpJson(streamPort, 'GET', '/api/browser/url');
  assert(response.success === true, `active URL request failed: ${JSON.stringify(response)}`);
  return response.data?.url || response.url;
}

async function focusTab(tab, index, taskName) {
  const response = await serviceRequest('view_focus', {
    targetId: tab.targetId,
    index,
    maximize: false,
    sessionName: session,
  }, taskName);
  assert(
    response.data?.tabSwitch?.targetId === tab.targetId,
    `view_focus did not report target switch: ${JSON.stringify(response)}`,
  );
  return response;
}

class SmokeWebSocket {
  constructor(port) {
    this.port = port;
    this.buffer = Buffer.alloc(0);
    this.messages = [];
    this.waiters = [];
    this.socket = null;
  }

  connect() {
    return new Promise((resolve, reject) => {
      const key = randomBytes(16).toString('base64');
      const socket = createConnection({ host: '127.0.0.1', port: this.port });
      this.socket = socket;
      let handshake = Buffer.alloc(0);
      const timeout = setTimeout(() => {
        socket.destroy();
        reject(new Error('WebSocket handshake timed out'));
      }, 10000);
      socket.on('connect', () => {
        socket.write([
          'GET / HTTP/1.1',
          `Host: 127.0.0.1:${this.port}`,
          'Upgrade: websocket',
          'Connection: Upgrade',
          `Sec-WebSocket-Key: ${key}`,
          'Sec-WebSocket-Version: 13',
          '\r\n',
        ].join('\r\n'));
      });
      socket.on('data', (chunk) => {
        if (handshake !== null) {
          handshake = Buffer.concat([handshake, chunk]);
          const end = handshake.indexOf('\r\n\r\n');
          if (end === -1) return;
          const header = handshake.slice(0, end).toString('utf8');
          if (!header.startsWith('HTTP/1.1 101')) {
            clearTimeout(timeout);
            reject(new Error(`WebSocket handshake failed: ${header}`));
            socket.destroy();
            return;
          }
          this.buffer = handshake.slice(end + 4);
          handshake = null;
          clearTimeout(timeout);
          this.readFrames(Buffer.alloc(0));
          resolve();
          return;
        }
        this.readFrames(chunk);
      });
      socket.on('error', reject);
    });
  }

  readFrames(chunk) {
    if (chunk.length) this.buffer = Buffer.concat([this.buffer, chunk]);
    while (this.buffer.length >= 2) {
      const first = this.buffer[0];
      const second = this.buffer[1];
      const opcode = first & 0x0f;
      let offset = 2;
      let length = second & 0x7f;
      if (length === 126) {
        if (this.buffer.length < 4) return;
        length = this.buffer.readUInt16BE(2);
        offset = 4;
      } else if (length === 127) {
        if (this.buffer.length < 10) return;
        const raw = this.buffer.readBigUInt64BE(2);
        if (raw > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('WebSocket frame too large');
        length = Number(raw);
        offset = 10;
      }
      const masked = Boolean(second & 0x80);
      const maskLength = masked ? 4 : 0;
      if (this.buffer.length < offset + maskLength + length) return;
      let payload = this.buffer.slice(offset + maskLength, offset + maskLength + length);
      if (masked) {
        const mask = this.buffer.slice(offset, offset + 4);
        payload = Buffer.from(payload.map((byte, index) => byte ^ mask[index % 4]));
      }
      this.buffer = this.buffer.slice(offset + maskLength + length);
      if (opcode === 1) this.pushMessage(JSON.parse(payload.toString('utf8')));
      if (opcode === 8) this.close();
    }
  }

  pushMessage(message) {
    const waiter = this.waiters.shift();
    if (waiter) {
      waiter.resolve(message);
    } else {
      this.messages.push(message);
    }
  }

  nextMessage(timeoutMs = 10000) {
    if (this.messages.length) return Promise.resolve(this.messages.shift());
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.waiters = this.waiters.filter((waiter) => waiter.resolve !== resolve);
        reject(new Error('Timed out waiting for WebSocket message'));
      }, timeoutMs);
      this.waiters.push({
        resolve: (value) => {
          clearTimeout(timeout);
          resolve(value);
        },
      });
    });
  }

  async nextFrame(label, previousHash = null, timeoutMs = 30000) {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      const message = await this.nextMessage(Math.max(1000, deadline - Date.now()));
      if (message.type !== 'frame') continue;
      const hash = frameHash(message);
      if (!previousHash || hash !== previousHash) return { frame: message, hash };
    }
    throw new Error(`Timed out waiting for distinct frame after ${label}`);
  }

  close() {
    this.socket?.end();
  }
}

try {
  streamPort = await ensureStreamPort(context, 120000);

  await serviceRequest('navigate', {
    headless: true,
    url: pageA,
    waitUntil: 'load',
  }, 'openPageA');

  const afterPageA = await waitForServiceTab('CDP Stream A', 'after page A launch');
  const browser = afterPageA.state?.browsers?.[browserId];
  assert(browser, `Service state missing browser ${browserId}: ${JSON.stringify(afterPageA.state)}`);
  assert(browser.host === 'local_headless', `Expected local_headless browser: ${JSON.stringify(browser)}`);
  assert(browser.health === 'ready', `Browser is not ready: ${JSON.stringify(browser)}`);
  const stream = primaryCdpStream(browser);
  assert(stream, `Browser did not advertise cdp_screencast stream: ${JSON.stringify(browser)}`);
  assert(stream.controlInput === 'cdp_input', `CDP stream is not controllable: ${JSON.stringify(stream)}`);
  assert(stream.url === `http://127.0.0.1:${streamPort}/`, `CDP stream URL mismatch: ${JSON.stringify(stream)}`);
  assert(stream.readiness?.state === 'ready', `CDP stream readiness mismatch: ${JSON.stringify(stream)}`);

  ws = new SmokeWebSocket(streamPort);
  await ws.connect();
  const pageAFrame = await ws.nextFrame('page A');

  await serviceRequest('tab_new', {
    url: pageB,
    waitUntil: 'load',
  }, 'openPageB');
  const afterPageB = await waitForServiceTab('CDP Stream B', 'after page B launch');
  const tabA = afterPageB.state.tabs[afterPageA.tab.id] ?? afterPageA.tab;
  const tabB = afterPageB.tab;
  const tabEntries = Object.values(afterPageB.state.tabs ?? {}).filter((tab) => tab.browserId === browserId);
  const tabAIndex = tabEntries.findIndex((tab) => tab.id === tabA.id);
  const tabBIndex = tabEntries.findIndex((tab) => tab.id === tabB.id);
  assert(tabAIndex >= 0 && tabBIndex >= 0, `Could not resolve tab indexes: ${JSON.stringify(tabEntries)}`);

  await focusTab(tabB, tabBIndex, 'focusPageB');
  assert((await activeUrl()).includes('CDP%20Stream%20B'), 'Active URL did not switch to page B');
  const pageBFrame = await ws.nextFrame('page B focus', pageAFrame.hash);

  await focusTab(tabA, tabAIndex, 'focusPageA');
  assert((await activeUrl()).includes('CDP%20Stream%20A'), 'Active URL did not switch back to page A');
  const pageASecondFrame = await ws.nextFrame('page A refocus', pageBFrame.hash);

  assert(pageAFrame.hash !== pageBFrame.hash, 'Page B focus did not produce a distinct stream frame');
  assert(pageBFrame.hash !== pageASecondFrame.hash, 'Page A refocus did not produce a distinct stream frame');

  await cleanup();
  console.log(`Service CDP tab streaming live smoke passed (${browserId}, stream ${streamPort})`);
} catch (err) {
  await fail(err.stack || err.message);
}
