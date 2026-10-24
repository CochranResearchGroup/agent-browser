#!/usr/bin/env node
/** Explicit owned Native acceptance. Required absolute P276_BINARY, P276_CHROME,
 * P276_EVIDENCE paths. Never installs, retries effects or borrows baseline desktops. */
import assert from 'node:assert/strict';
import { createServer, request as httpRequest } from 'node:http';
import { createConnection } from 'node:net';
import { spawn, execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { randomUUID, createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, writeFile, readdir, realpath, stat, readlink } from 'node:fs/promises';
import { isAbsolute, join, resolve } from 'node:path';
import { once } from 'node:events';
import { tmpdir } from 'node:os';
import { appProcessOwned, retainViewerProcess } from './hosted-process-ownership.mjs';
import { readFreshCleanupIdentity } from './hosted-fixture-cleanup.mjs';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
const repo = resolve(new URL('..', import.meta.url).pathname);
const required = name => { const value = process.env[name]; assert(value && isAbsolute(value), `${name} requires an absolute path`); return value; };
const binary = await realpath(required('P276_BINARY')), chrome = await realpath(required('P276_CHROME'));
const evidence = required('P276_EVIDENCE');
await mkdir(evidence, { mode: 0o700 }); // Never overwrite an earlier run.
const save = (name, value) => writeFile(join(evidence, name), JSON.stringify(value, null, 2), { mode: 0o600 });
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const root = join(evidence, 'runtime'), sockets = await mkdtemp(join(tmpdir(), 'p276-s-')), profile = join(root, 'synthetic-profile');
for (const path of [root, profile]) await mkdir(path, { mode: 0o700 });
const marker = `p276-${randomUUID()}`, token = randomUUID();
const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith('AGENT_BROWSER_') && !key.startsWith('XDG_') && !key.startsWith('P276_')));
Object.assign(env, { HOME: root, AGENT_BROWSER_HOME: join(root, '.agent-browser'), AGENT_BROWSER_SOCKET_DIR: sockets,
  AGENT_BROWSER_RUNTIME_HOST: '1', AGENT_BROWSER_SESSION: 'default', AGENT_BROWSER_HEADED: '1',
  AGENT_BROWSER_EXECUTABLE_PATH: chrome, AGENT_BROWSER_ARGS: '--no-sandbox,--disable-background-networking',
  AGENT_BROWSER_DASHBOARD_AUTH_DIR: join(root, 'operator-auth'), AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY: 'disabled',
  AGENT_BROWSER_REMOTE_VIEW_APPLICATION: 'agent-browser', AGENT_BROWSER_REMOTE_VIEW_POOL: 'main',
  AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT: '1', AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN: 'https://remote-view.ecochran.dyndns.org',
  AGENT_BROWSER_PUBLIC_ORIGIN: 'https://agent-browser.ecochran.dyndns.org',
  XDG_RUNTIME_DIR: join(root, 'xdg-runtime'), XDG_CONFIG_HOME: join(root, 'config'), XDG_CACHE_HOME: join(root, 'cache'), XDG_DATA_HOME: join(root, 'data') });
for (const path of [env.AGENT_BROWSER_HOME, env.XDG_RUNTIME_DIR, env.XDG_CONFIG_HOME, env.XDG_CACHE_HOME, env.XDG_DATA_HOME]) await mkdir(path, { mode: 0o700 });
await writeFile(join(sockets, 'runtime-host.token'), token, { mode: 0o600 });
const retained = new Map(), requests = [], errors = [], opened = [];
let assignment, observation, baseline, launch, host, broker, server, viewerServer, viewer, cdp, cookie = '', backendPort;
let step = 'build-fixture', passed = false, uncertain = false;
let hostProof, viewerProof, providerProof;
const hostProcesses = new Map(), viewerProcesses = new Map();
const retain = row => { if (row) retained.set(`${row.pid}:${row.start}`, row); };
async function kernel(pid) {
  const fields = (await readFile(`/proc/${pid}/stat`, 'utf8')).split(')').at(-1).trim().split(/\s+/);
  return { pid: Number(pid), uid: (await stat(`/proc/${pid}`)).uid, start: fields[19], parentPid: Number(fields[1]), processGroup: Number(fields[2]), session: Number(fields[3]) };
}
const fresh = pid => readFreshCleanupIdentity(Number(pid), { identity: pid => kernel(pid), executable: pid => readlink(`/proc/${pid}/exe`) });
const cleanup = { errors: [], released: false };
async function identity(pid) { return fresh(Number(pid)); }
async function census() {
  for (const name of await readdir('/proc')) if (/^\d+$/.test(name)) {
    let p; try { p = await kernel(Number(name)); } catch (error) { if (['ENOENT', 'ESRCH'].includes(error.code)) continue; throw error; }
    if (hostProof && appProcessOwned(p, hostProof, hostProcesses)) { const row = await fresh(p.pid); if (row) { hostProcesses.set(row.pid, row); retain(row); } }
    if (viewerProof && await retainViewerProcess(p, { root: viewerProof, observed: viewerProcesses, readRoot: fresh, readKernel: kernel })) retain({ ...p, executable: await readlink(`/proc/${p.pid}/exe`) });
  }
  for (const browser of [cdp, viewer]) if (browser?.isConnected()) {
    const session = await browser.newBrowserCDPSession(); const result = await session.send('SystemInfo.getProcessInfo'); await session.detach();
    for (const process of result.processInfo) { const row = await fresh(process.id); retain(row); }
  }
  await save('retained-processes.json', [...retained.values()]);
}
async function native(request) {
  const mutates = ['acquire', 'activate', 'register_native_handoff_observation', 'release'].includes(request.operation);
  if (providerProof) { const current = await fresh(providerProof.pid); assert(current && current.start === providerProof.start && current.executable === providerProof.executable, 'Reviewed Native provider changed'); }
  if (mutates) await save(`intent-${request.operation}-${randomUUID()}.json`, { providerProof, request, state: 'dispatching_once' });
  try {
    const response = await fetch('http://127.0.0.1:19096/v1/consumer', { method: 'POST', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ application: 'agent-browser', request }), redirect: 'error', signal: AbortSignal.timeout(45000) });
    const body = await response.json();
    if (mutates) await save(`response-${request.operation}-${randomUUID()}.json`, { status: response.status, body });
    assert(response.ok, `Native ${request.operation} returned ${response.status}`);
    assert(body && typeof body === 'object');
    if (request.operation === 'acquire') assert(typeof body.assignmentId === 'string' && typeof body.desktopId === 'string' && Number.isSafeInteger(body.generation) && body.state === 'active', 'Unqualified acquisition response');
    return body;
  } catch (error) { if (mutates) { uncertain = true; await save('uncertain-native-effect.json', { request, providerProof, code: error.code ?? error.name }); } throw error; }
}
const exact = operation => ({ operation, assignment_id: assignment.assignmentId, expected_generation: assignment.generation,
  expected_viewing_generation: observation.target.viewingGeneration });
async function raw(lane, action, params = {}) {
  const socket = createConnection(join(sockets, 'runtime-host.sock'));
  const result = await new Promise((resolve, reject) => {
    let buffer = '', dispatched = false; const timer = setTimeout(() => { socket.destroy(); uncertain = true; reject(Error(`Uncertain ${action} timeout`)); }, 45000);
    socket.on('error', error => { clearTimeout(timer); if (dispatched) uncertain = true; reject(error); });
    socket.on('connect', () => { dispatched = true; socket.write(JSON.stringify({ id: randomUUID(), action, _agentBrowserAuthToken: token,
      _agentBrowserRuntimeLane: lane, sessionName: lane, runtimeProfile: 'p276-synthetic', profile,
      serviceName: 'P276SyntheticAcceptance', agentName: 'Codex', taskName: `Synthetic Focus ${lane}`, ...params }) + '\n'); });
    socket.on('data', data => { buffer += data; if (buffer.includes('\n')) { clearTimeout(timer); socket.end(); try { resolve(JSON.parse(buffer.split('\n')[0])); } catch (error) { uncertain = true; reject(error); } } });
  });
  await save(`last-${action}.json`, result); assert(result.success, `Owner ${action} failed: ${JSON.stringify(result)}`); await census(); return result;
}
async function until(predicate, label) { const end = Date.now() + 30000; while (!await predicate()) { assert(Date.now() < end, label); await sleep(50); } }
function foreground() {
  const xenv = { ...env, ...launch.environment };
  const window = execFileSync('xdotool', ['getactivewindow'], { env: xenv, encoding: 'utf8' }).trim();
  return { window, pid: Number(execFileSync('xdotool', ['getwindowpid', window], { env: xenv, encoding: 'utf8' }).trim()),
    title: execFileSync('xdotool', ['getwindowname', window], { env: xenv, encoding: 'utf8' }).trim() };
}
try {
  await buildWorkspaceViewportFixture(repo, join(repo, 'scripts/fixtures/native-selection-focus.tsx'), evidence);
  await save('inputs.json', { binary, binarySha256: createHash('sha256').update(await readFile(binary)).digest('hex'), chrome,
    chromeSha256: createHash('sha256').update(await readFile(chrome)).digest('hex'), source: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim() });
  step = 'pin-provider';
  const providerPid = Number(execFileSync('systemctl', ['--user', 'show', 'remote-view-main-5146a1bafcc4-control.service', '-p', 'MainPID', '--value'], { encoding: 'utf8' }).trim());
  providerProof = await fresh(providerPid); assert(providerProof && providerProof.executable.includes('/remote-view/'));
  const listener = execFileSync('ss', ['-ltnp', 'sport = :19096'], { encoding: 'utf8' }); assert(listener.includes(`pid=${providerPid},`));
  await save('runtime-custody.json', { operationId: marker, authority: 'User okay go for issue68 remaining synthetic Native Focus acceptance', effect: 'One new assignment and private candidate runtime only', state: 'preflight' });
  await save('provider-pin.json', { ...providerProof, sha256: createHash('sha256').update(await readFile(providerProof.executable)).digest('hex'), listener });
  step = 'baseline'; baseline = await native({ operation: 'inventory' }); await save('baseline.json', baseline);
  step = 'acquire-new-assignment';
  assignment = await native({ operation: 'acquire', pool_name: 'main', idempotency_key: marker });
  await save('acquired.json', assignment);
  assert(!baseline.assignments.some(a => a.assignmentId === assignment.assignmentId), 'Cannot borrow a baseline assignment');
  observation = await native({ operation: 'observe_assignment', assignment_id: assignment.assignmentId, expected_generation: assignment.generation });
  launch = await native(exact('launch_environment')); await save('native-observation.json', observation);
  Object.assign(env, launch.environment, { AGENT_BROWSER_SESSION_DISPLAY: launch.environment.DISPLAY });
  step = 'scoped-broker';
  const allowed = new Set(['observe_assignment', 'launch_environment', 'windows', 'activate', 'events', 'observe_idle', 'read_idle_retirement', 'register_native_handoff_observation']);
  const correlations = new Set();
  broker = createServer(async (req, res) => {
    try {
      assert(req.method === 'POST' && req.url === '/v1/consumer'); let text = '';
      for await (const part of req) { text += part; assert(text.length < 65536); }
      const envelope = JSON.parse(text), request = envelope.request; assert.equal(envelope.application, 'agent-browser');
      if (request.operation !== 'inventory' && request.operation !== 'observe_native_handoff_use') {
        assert(allowed.has(request.operation)); assert.equal(request.assignment_id, assignment.assignmentId); assert.equal(request.expected_generation, assignment.generation);
        if ('expected_viewing_generation' in request) assert.equal(request.expected_viewing_generation, observation.target.viewingGeneration);
      } else if (request.operation === 'observe_native_handoff_use') assert(correlations.has(request.correlation_id));
      const result = await native(request);
      if (request.operation === 'inventory') {
        result.assignments = result.assignments.filter(a => a.assignmentId === assignment.assignmentId);
        result.desktops = result.desktops.filter(d => d.desktopId === assignment.desktopId);
        result.pools = result.pools.map(pool => ({ ...pool, desktopMembers: pool.desktopMembers.filter(id => id === assignment.desktopId) }));
      }
      if (request.operation === 'register_native_handoff_observation') correlations.add(result.correlationId ?? result.observation?.correlationId);
      requests.push({ operation: request.operation, success: true }); await save('broker-requests.json', requests);
      res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify(result));
    } catch (error) { errors.push({ step: 'broker', message: error.message }); res.writeHead(502); res.end(JSON.stringify({ code: 'fixture_scope_refused', message: 'Inspect fixture evidence', retryable: false })); }
  });
  await new Promise(resolve => broker.listen(0, '127.0.0.1', resolve)); env.AGENT_BROWSER_REMOTE_VIEW_ORIGIN = `http://127.0.0.1:${broker.address().port}`;
  step = 'start-isolated-host';
  host = spawn(binary, [], { env: { ...env, AGENT_BROWSER_RUNTIME_HOST_PROCESS: '1', AGENT_BROWSER_DAEMON_AUTH_TOKEN: token }, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const hostLog = data => { void writeFile(join(evidence, 'host.log'), data, { flag: 'a', mode: 0o600 }).catch(error => { cleanup.errors.push({ step: 'host-log', code: error.code }); }); };
  host.stdout.on('data', hostLog); host.stderr.on('data', hostLog);
  hostProof = await identity(host.pid); assert(hostProof); retain(hostProof); await census();
  await until(async () => { assert(host.exitCode === null); try { return JSON.parse(await readFile(join(sockets, 'runtime-host.json'))).pid === host.pid; } catch { return false; } }, 'Host did not publish');
  step = 'open-owned-tabs';
  for (const lane of ['A', 'B']) {
    await raw(lane, 'browser_session_open', { profileId: 'p276-synthetic' }); opened.push(lane);
    await raw(lane, 'navigate', { url: `data:text/html,<title>P276 Focus ${lane}</title><input id=draft value=kept-${lane}><p>Synthetic ${lane}</p>` });
  }
  const state = (await raw('A', 'browser_session_status')).data;
  await save('owner-state.json', state);
  const browser = Object.values(state.browsers)[0]; assert(browser);
  step = 'owner-observer'; cdp = await chromium.connectOverCDP(browser.cdpEndpoint);
  const pages = cdp.contexts().flatMap(context => context.pages());
  const before = await Promise.all(pages.map(async page => ({ title: await page.title(), draft: await page.locator('#draft').inputValue() })));
  const browserProof = await identity(browser.pid); assert(browserProof); retain(browserProof); await census();
  backendPort = Number((await readFile(join(sockets, 'default.stream'), 'utf8')).trim());
  const backend = `http://127.0.0.1:${backendPort}`;
  step = 'private-dashboard-login'; await fetch(backend + '/api/dashboard-auth/status');
  const credentials = await readFile(join(env.AGENT_BROWSER_DASHBOARD_AUTH_DIR, 'dashboard-auth.env'), 'utf8');
  const password = credentials.split('\n').find(line => line.startsWith('AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD=')).split('=').slice(1).join('=').replace(/^"|"$/g, '');
  const logged = await fetch(backend + '/api/dashboard-auth/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ username: 'admin', password }) });
  assert.equal(logged.status, 200); cookie = logged.headers.get('set-cookie').split(';')[0];
  server = createServer(async (req, res) => {
    if (req.url === '/' || req.url === '/bundle.js') { res.writeHead(200, { 'content-type': req.url === '/' ? 'text/html' : 'application/javascript' }); res.end(req.url === '/' ? '<div id="root"></div><script src="/bundle.js"></script>' : await readFile(join(evidence, 'bundle.js'))); return; }
    const proxy = httpRequest({ hostname: '127.0.0.1', port: backendPort, method: req.method, path: req.url,
      headers: { ...req.headers, host: `127.0.0.1:${backendPort}`, origin: backend, cookie } }, reply => { res.writeHead(reply.statusCode, reply.headers); reply.pipe(res); });
    proxy.on('error', () => { res.writeHead(502); res.end(); }); req.pipe(proxy);
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  step = 'actual-selection-ui';
  viewerServer = await chromium.launchServer({ executablePath: chrome, headless: true, args: [`--${marker}`, '--disable-background-networking'] });
  viewerProof = await identity(viewerServer.process().pid); assert(viewerProof); retain(viewerProof);
  viewer = await chromium.connect(viewerServer.wsEndpoint()); const page = await viewer.newPage(); page.setDefaultTimeout(15000);
  page.on('pageerror', error => errors.push({ step: 'page', message: error.message }));
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const scenes = [];
  for (const lane of ['A', 'B', 'A']) {
    await page.getByRole('button', { name: `Inspect P276 Focus ${lane}`, exact: true }).click();
    await page.getByRole('button', { name: 'Focus', exact: true }).click();
    await page.getByText('Focus request accepted for the selected tab.', { exact: true }).waitFor();
    const scene = foreground(); assert(scene.title.includes(`P276 Focus ${lane}`), 'Independent native title mismatch'); assert.equal(scene.pid, browser.pid);
    scenes.push({ lane, scene }); await save('focus-scenes.json', scenes); await census();
  }
  const after = await Promise.all(pages.map(async page => ({ title: await page.title(), draft: await page.locator('#draft').inputValue() })));
  assert.deepEqual(after, before); assert.equal((await identity(browser.pid)).start, browserProof.start); assert.deepEqual(errors, []);
  await save('qualification.json', { passed: true, scenes, peersPreserved: true, browserGenerationPreserved: true, before, after });
  passed = true;
} catch (error) { await save('failure.json', { step, message: error.message, uncertain }); process.exitCode = 1; }
finally {
  async function clean(name, action) { let timer; try { await Promise.race([action(), new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Bounded cleanup deadline')), 50000); })]); } catch (error) { cleanup.errors.push({ step: name, message: error.message }); process.exitCode = 1; } finally { clearTimeout(timer); } }
  await clean('capture-processes', census);
  await clean('close-ui-client', async () => { await viewer?.close(); });
  await clean('close-ui-server', async () => { await viewerServer?.close(); });
  for (const lane of opened) await clean(`close-task-${lane}`, () => raw(lane, 'browser_session_close', { leaveOpen: false }));
  await clean('close-observer', async () => { await cdp?.close(); });
  await clean('stop-host', async () => { if (host && host.exitCode === null) { const stopped = once(host, 'exit'); assert.equal((await identity(host.pid))?.start, hostProof.start); host.kill('SIGTERM'); await Promise.race([stopped, sleep(15000).then(() => { throw Error('Owned host still live'); })]); } });
  await clean('process-absence', async () => {
    await until(async () => { for (const p of retained.values()) if ((await identity(p.pid))?.start === p.start) return false; return true; }, 'Fixture process generation remains');
    const references = [];
    for (const name of await readdir('/proc')) if (/^\d+$/.test(name)) try { const cmd = await readFile(`/proc/${name}/cmdline`, 'utf8'); if (cmd.includes(profile) || cmd.includes(marker)) references.push(name); } catch (error) { if (!['ENOENT', 'ESRCH', 'EACCES'].includes(error.code)) throw error; }
    assert.deepEqual(references, []); cleanup.processesClear = true;
  });
  if (assignment && !baseline.assignments.some(a => a.assignmentId === assignment.assignmentId) && cleanup.processesClear && !uncertain && cleanup.errors.length === 0) await clean('release-owned-assignment', async () => {
    const windows = await native(exact('windows')); assert.equal(windows.windows.length, 0);
    await native({ operation: 'release', assignment_id: assignment.assignmentId, expected_generation: assignment.generation, idempotency_key: marker + '-release',
      cleanup: { schemaVersion: 1, assignmentId: assignment.assignmentId, lifecycleGeneration: assignment.generation, applicationReferencesClear: true, pendingRecoveryClear: true, foregroundLeasesClear: true, cleanupTasksClear: true } });
    const inventory = await native({ operation: 'inventory' }); assert(!inventory.assignments.some(a => a.assignmentId === assignment.assignmentId && a.state === 'active'));
    const baselineActive = baseline.assignments.filter(a => a.state === 'active');
    assert(baselineActive.every(a => inventory.assignments.some(b => ['assignmentId', 'desktopId', 'generation', 'state'].every(key => a[key] === b[key])))); cleanup.released = true; cleanup.baselineAssignmentsPreserved = true;
  });
  for (const listener of [server, broker]) if (listener) await clean('close-listener', async () => { listener.closeAllConnections(); await new Promise(resolve => listener.close(resolve)); });
  await save('cleanup.json', { ...cleanup, passed, uncertain, retained: [...retained.values()] });
  console.log(JSON.stringify({ passed, released: cleanup.released, errors: cleanup.errors, evidence }));
}
