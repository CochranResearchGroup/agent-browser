const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'workspace-startup-'));
  let server, browser, browserServer;
  const owned = new Map();
  const capture = () => {
    const rows = [];
    for (const name of fs.readdirSync("/proc")) {
      if (!/^\d+$/.test(name)) continue;
      try { const fields = fs.readFileSync(`/proc/${name}/stat`, "utf8").split(") ").at(-1).split(" "); rows.push({ pid: Number(name), parent: Number(fields[1]), start: fields[19] }); } catch {}
    }
    const ids = new Set([browserServer.process().pid]);
    for (let i = 0; i < rows.length; i++) for (const row of rows) if (ids.has(row.parent)) ids.add(row.pid);
    for (const row of rows) if (ids.has(row.pid)) owned.set(`${row.pid}:${row.start}`, row);
  };
  try {
    await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/workspace-startup.tsx'), artifacts);
    server = http.createServer((request, response) => {
      response.setHeader('Content-Type', request.url === '/bundle.js' ? 'application/javascript' : 'text/html');
      response.end(request.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browserServer = await chromium.launchServer({ headless: true, executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome' });
    browser = await chromium.connect(browserServer.wsEndpoint());
    for (const mode of ['ready', 'unknown', 'failed', 'empty']) {
      const context = await browser.newContext();
      try {
        const page = await context.newPage();
        const reads = [], errors = [];
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/api/**', async route => {
          if (!route.request().url().includes('/api/service/status')) return route.fulfill({ json: { success: true, data: {} } });
          reads.push(Date.now());
          if (mode === 'failed') return route.fulfill({ status: 503, json: { success: false } });
          const state = mode === 'ready' && reads.length >= 3 ? 'live' : 'unknown';
          const workspace = { schemaVersion: 'agent-browser.managed-workspace.v1', source: 'managed-owner', observedAtMs: Date.now(), observationAgeMs: 0,
            profiles: { p: { id: 'p', kind: 'disposable' } },
            browsers: { b: { id: 'b', profileId: 'p', pid: 42, presence: { state }, activeSessionIds: ['s'], state: 'reported' } },
            sessions: { s: { id: 's', name: 'synthetic', profileId: 'p', browserId: 'b', createdAtMs: 1, lastActivityAtMs: 1, expiresAtMs: 0, currentTabId: 't' } },
            tabs: mode === 'empty' ? {} : { t: { id: 't', targetId: 'target', browserId: 'b', sessionId: 's', createdAtMs: 1, presence: { state }, url: 'about:blank', title: 'Synthetic task' } } };
          return route.fulfill({ json: { success: true, data: { browserRuntime: { state: 'available', workspace } } } });
        });
        await page.goto(`http://127.0.0.1:${server.address().port}`);
        if (mode === 'ready') {
          await page.locator('.native-workspace-tab').waitFor({ timeout: 2000 });
          assert.equal(reads.length, 3, 'catch-up must bypass the one-second shared response cache');
          assert.ok(reads[2] - reads[0] < 1500, 'live owner evidence must populate without a three-second wait');
        } else {
          await page.waitForTimeout(1600);
          assert.equal(reads.length, mode === 'unknown' ? 4 : 1, 'catch-up must exhaust its bound or stop on failure/empty');
          assert.equal(await page.locator('.native-workspace-tab').count(), 0, 'unknown observations never invent live rows');
        }
        capture();
        assert.deepEqual(errors, []);
        console.log(JSON.stringify({ mode, requests: reads.length, elapsedMs: reads.at(-1) - reads[0] }));
      } finally { await context.close(); }
    }
  } finally {
    if (browserServer) capture();
    await browser?.close();
    await browserServer?.close();
    for (const row of owned.values()) {
      let current;
      try { current = fs.readFileSync(`/proc/${row.pid}/stat`, 'utf8').split(') ').at(-1).split(' ')[19]; } catch {}
      assert.notEqual(current, row.start, 'owned fixture browser generation must be absent after close');
    }
    console.log(JSON.stringify({ observedProcessGenerations: owned.size, retained: 0 }));
    if (server) await new Promise(resolve => server.close(resolve));
    fs.rmSync(artifacts, { recursive: true, force: true });
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
