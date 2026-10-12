#!/usr/bin/env node
/** Synthetic MCP-to-native-viewer acceptance against an explicitly prepared private provider. */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { createServer as httpServer, request as httpRequest } from 'node:http';
import { createServer as httpsServer } from 'node:https';
import { connect } from 'node:net';
import { randomUUID, createHash } from 'node:crypto';
import { chromium } from 'playwright';
import { createSmokeContext, createMcpStdioClient } from './smoke-utils.js';
const root = process.env.AGENT_BROWSER_SMOKE_NATIVE_FIXTURE_HOME;
assert(root, 'Pin the private synthetic native provider fixture home');
assert(
  readFileSync(`${root}/config.toml`, 'utf8').includes('instance = "p238fixture"'),
  'Use the dedicated synthetic provider'
);
const binary = process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
assert(binary, 'Pin the candidate executable');
const chrome = process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(chrome, 'Pin the reviewed Linux browser executable');
process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD = binary;
const context = createSmokeContext({ prefix: 'p238-native-task-', sessionPrefix: 'p238' });
const env = Object.fromEntries(
  Object.entries(context.env).filter(
    ([k]) => !k.startsWith('AGENT_BROWSER_') && !k.startsWith('XDG_')
  )
);
Object.assign(env, {
  AGENT_BROWSER_HOME: context.agentHome,
  AGENT_BROWSER_SOCKET_DIR: context.socketDir,
  AGENT_BROWSER_RUNTIME_HOST: '1',
  AGENT_BROWSER_EXECUTABLE_PATH: chrome,
  AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD: binary,
  AGENT_BROWSER_PUBLIC_ORIGIN: 'https://browser.p238.test:55440',
  AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN: 'https://native.p238.test:55440',
  AGENT_BROWSER_REMOTE_VIEW_ORIGIN: 'http://127.0.0.1:55444',
  AGENT_BROWSER_REMOTE_VIEW_POOL: 'p238',
  AGENT_BROWSER_REMOTE_VIEW_APPLICATION: 'agent-browser',
  AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT: '1',
  XDG_RUNTIME_DIR: `${context.tempHome}/runtime`,
  XDG_CONFIG_HOME: `${context.tempHome}/config`,
  XDG_DATA_HOME: `${context.tempHome}/data`,
});
mkdirSync(env.XDG_RUNTIME_DIR, { mode: 0o700 });
context.env = env;
const cookie = `p238_session=${randomUUID()}`;
const secret = readFileSync(`${root}/ingress.secret`, 'utf8').trim();
const authenticated = (h) =>
  h.cookie
    ?.split(';')
    .map((x) => x.trim())
    .includes(cookie);
const auth = httpServer((req, res) => {
  if (!authenticated(req.headers)) {
    res.writeHead(401);
    res.end();
    return;
  }
  res.writeHead(204, { 'Remote-User': 'p238-operator' });
  res.end();
});
function headers(req, native) {
  const h = { ...req.headers };
  for (const key of [
    'remote-user',
    'x-remote-view-proxy-secret',
    'x-forwarded-user',
    'x-auth-request-user',
    'x-remote-user',
  ])
    delete h[key];
  if (native) {
    h['remote-user'] = 'p238-operator';
    h['x-remote-view-proxy-secret'] = secret;
  }
  return h;
}
const edge = httpsServer(
  { key: readFileSync(`${root}/tls.key`), cert: readFileSync(`${root}/tls.crt`) },
  (req, res) => {
    const native = req.headers.host === 'native.p238.test:55440';
    if (native && !authenticated(req.headers)) {
      res.writeHead(401);
      res.end('Native fixture authentication required');
      return;
    }
    const upstream = httpRequest(
      {
        hostname: '127.0.0.1',
        port: native ? 55441 : 55445,
        path: req.url,
        method: req.method,
        headers: headers(req, native),
      },
      (reply) => {
        res.writeHead(reply.statusCode, reply.headers);
        reply.pipe(res);
      }
    );
    upstream.on('error', () => {
      res.writeHead(502);
      res.end('Fixture upstream unavailable');
    });
    req.pipe(upstream);
  }
);
edge.on('upgrade', (req, socket, head) => {
  if (req.headers.host !== 'native.p238.test:55440' || !authenticated(req.headers)) {
    socket.end('HTTP/1.1 401 Unauthorized\r\nConnection: close\r\n\r\n');
    return;
  }
  const upstream = connect(55441, '127.0.0.1', () => {
    const lines = Object.entries(headers(req, true))
      .map(([k, v]) => `${k}: ${v}`)
      .join('\r\n');
    upstream.write(`${req.method} ${req.url} HTTP/1.1\r\n${lines}\r\n\r\n`);
    if (head.length) upstream.write(head);
    socket.pipe(upstream);
    upstream.pipe(socket);
  });
  upstream.on('error', () => socket.destroy());
  socket.on('error', () => upstream.destroy());
});
const listen = (server, port) =>
  new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', resolve);
  });
const processes = () =>
  readdirSync('/proc')
    .filter((p) => /^\d+$/.test(p) && Number(p) !== process.pid)
    .filter((p) => {
      try {
        return (
          readFileSync(`/proc/${p}/environ`)
            .toString()
            .split('\0')
            .includes(`AGENT_BROWSER_HOME=${context.agentHome}`) ||
          readFileSync(`/proc/${p}/cmdline`)
            .toString()
            .replaceAll('\0', ' ')
            .includes(`--user-data-dir=${context.agentHome}/`)
        );
      } catch {
        return false;
      }
    })
    .map(Number);
let client, backend, viewer, task, peer;
const evidence = {
  candidateExecutable: binary,
  candidateSha256: createHash('sha256').update(readFileSync(binary)).digest('hex'),
  scope:
    'isolated native viewer fixture; local trusted proxy authentication, not production Authelia',
};
const call = async (name, args) => {
  const r = await client.send('tools/call', { name, arguments: args });
  const v = JSON.parse(r.content.find((c) => c.type === 'text').text);
  assert.equal(v.success, true, JSON.stringify(v));
  return v.data;
};
try {
  await listen(auth, 55446);
  await listen(edge, 55440);
  backend = spawn(binary, [], {
    env: {
      ...env,
      AGENT_BROWSER_DASHBOARD: '1',
      AGENT_BROWSER_DASHBOARD_BACKEND_ONLY: '1',
      AGENT_BROWSER_DASHBOARD_PORT: '55445',
    },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  backend.stderr.on('data', (b) => writeFileSync(`${root}/backend-stderr.log`, b, { flag: 'a' }));
  client = createMcpStdioClient({ context, args: ['mcp', 'serve'], onFatal: () => {} });
  await client.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'p238-native-fixture', version: '0' },
  });
  client.notify('notifications/initialized');
  task = await call('browser_open', {
    profile: 'native-fixture',
    profileMode: 'persistent',
    taskKey: 'alice',
    url: 'data:text/html,<title>P238 native input</title><textarea aria-label="Draft" id="draft" style="width:100vw;height:100vh;font-size:32px;box-sizing:border-box"></textarea>',
  });
  evidence.task = task;
  console.log('Native browser task opened');
  peer = await call('browser_open', {
    profile: 'native-fixture',
    profileMode: 'persistent',
    taskKey: 'bob',
    url: 'data:text/html,<title>P238 peer</title><input id="draft" aria-label="Draft">',
  });
  assert.equal(peer.browserId, task.browserId);
  assert.notEqual(peer.tabId, task.tabId);
  await call('browser_fill', {
    taskId: peer.taskId,
    selector: '#draft',
    value: 'Bob untouched draft',
  });
  evidence.peer = peer;
  const handoff = await call('browser_handoff', { taskId: task.taskId });
  assert.equal(handoff.operatorVisible.state, 'ready');
  assert(handoff.handoffUrl.startsWith('https://browser.p238.test:55440/remote-view/'));
  evidence.handoff = handoff;
  console.log('Complete ready handoff returned');
  viewer = await chromium.launch({
    executablePath: chrome,
    headless: true,
    env,
    args: [
      '--no-sandbox',
      '--no-proxy-server',
      '--host-resolver-rules=MAP browser.p238.test 127.0.0.1, MAP native.p238.test 127.0.0.1',
    ],
  });
  const viewing = await viewer.newContext({
    ignoreHTTPSErrors: true,
    viewport: { width: 1400, height: 1000 },
  });
  await viewing.addCookies([
    {
      name: 'p238_session',
      value: cookie.split('=')[1],
      domain: 'native.p238.test',
      path: '/',
      secure: true,
      httpOnly: true,
    },
  ]);
  const page = await viewing.newPage();
  page.on('pageerror', (e) => console.error('Native viewer script error:', e.message));
  page.on('response', (r) => {
    if (r.status() >= 400)
      console.error('Native HTTP failure', new URL(r.url()).pathname, r.status());
  });
  page.on('requestfailed', (r) =>
    console.error('Native request failed', new URL(r.url()).pathname, r.failure()?.errorText)
  );
  await page.goto(handoff.handoffUrl, { waitUntil: 'domcontentloaded', timeout: 30000 });
  assert.equal(new URL(page.url()).hostname, 'native.p238.test');
  evidence.viewerUrl = page.url();
  try {
    await page.waitForFunction(
      () =>
        document.querySelector('#connection-status')?.dataset.state === 'connected' ||
        document.querySelector('#connection-status')?.textContent?.trim() === 'Connected',
      {},
      { timeout: 30000 }
    );
  } catch (e) {
    console.error('Native viewer state', await page.locator('body').innerText());
    await page.screenshot({ path: `${root}/synthetic-viewer-failure.png` });
    throw e;
  }
  console.log('Native viewer connected');
  const surface = page.locator('#desktop-surface');
  const box = await surface.boundingBox();
  assert(box);
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await page.keyboard.type('P238 human viewer input');
  await page.waitForTimeout(800);
  const continued = await call('browser_continue', { taskId: task.taskId });
  assert.equal(continued.taskId, task.taskId);
  assert.equal(continued.browserId, task.browserId);
  assert.equal(continued.tabId, task.tabId);
  assert.equal(continued.url, task.url);
  const snapshot = await call('browser_snapshot', { taskId: task.taskId });
  assert(
    JSON.stringify(snapshot).includes('P238 human viewer input'),
    'native viewer input must survive continuation'
  );
  evidence.continued = continued;
  evidence.inputConfirmed = true;
  console.log('Native viewer input survived same-task continuation');
  const peerPage = await call('browser_snapshot', { taskId: peer.taskId });
  assert(JSON.stringify(peerPage).includes('Bob untouched draft'));
  await call('browser_close', { taskId: task.taskId });
  task = null;
  const survivingPeer = await call('browser_snapshot', { taskId: peer.taskId });
  assert(JSON.stringify(survivingPeer).includes('Bob untouched draft'));
  evidence.peerPreserved = true;
  console.log('Native handoff and Alice close preserve Bob');
} finally {
  for (const ownedTask of [task, peer])
    if (ownedTask && client) {
      try {
        await call('browser_close', { taskId: ownedTask.taskId });
      } catch (e) {
        console.error('Scoped task close:', e.message);
      }
    }
  await viewer?.close();
  client?.close();
  backend?.kill('SIGTERM');
  edge.closeAllConnections();
  auth.closeAllConnections();
  await Promise.all([new Promise((r) => edge.close(r)), new Promise((r) => auth.close(r))]);
  for (const pid of processes()) {
    try {
      process.kill(pid, 'SIGTERM');
    } catch {}
  }
  const deadline = Date.now() + 5000;
  while (processes().length && Date.now() < deadline) await new Promise((r) => setTimeout(r, 50));
  evidence.ownedProcessResidue = processes();
  writeFileSync(`${root}/acceptance-evidence.json`, JSON.stringify(evidence, null, 2), {
    mode: 0o600,
  });
  assert.equal(evidence.ownedProcessResidue.length, 0);
  context.cleanupTempHome();
  console.log('Task fixture residue: zero');
}
