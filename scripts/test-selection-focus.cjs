#!/usr/bin/env node
/** Actual mounted Selection component, synthetic intercepted service responses.
 * No Native/provider desktop is contacted or changed. */
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const http = require('node:http');
const { randomUUID } = require('node:crypto');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
const root = path.resolve(__dirname, '..');
const args = process.argv.slice(2);
function option(name) { const value = args[args.indexOf(name) + 1]; assert(args.includes(name) && value && path.isAbsolute(value), `${name} requires an absolute path`); return value; }
const artifacts = option('--artifact-dir');
const chrome = args.includes('--bundle-only') ? null : option('--chrome');
const statIdentity = pid => {
  try { const fields = fs.readFileSync(`/proc/${pid}/stat`, 'utf8').split(')').slice(1).join(')').trim().split(/\s+/); return { pid, start: fields[19] }; }
  catch (error) { if (error.code === 'ENOENT' || error.code === 'ESRCH') return null; throw error; }
};
(async () => {
  fs.mkdirSync(artifacts, { recursive: true });
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/selection-focus.tsx'), artifacts);
  if (args.includes('--bundle-only')) return;
  const server = http.createServer((req, res) => { res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html'); res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>'); });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const marker = `--p274-selection-${randomUUID()}`;
  fs.writeFileSync(path.join(artifacts, 'launch-marker.json'), JSON.stringify({ marker, chrome }));
  let browser, browserServer;
  const identities = new Map();
  async function scenario(mode) {
    const page = await browser.newPage(); const requests = [], held = [], errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const response = body => ({ success: true, data: { handoffId: 'synthetic-link', handoffUrl: origin + '/remote-view/synthetic-link', operatorVisible: { state: 'ready' } } });
    await page.route('**/*', route => {
      const url = new URL(route.request().url());
      if (url.origin === origin && ['/', '/bundle.js'].includes(url.pathname)) return route.continue();
      if (url.origin !== origin || url.pathname !== '/api/service/request') { errors.push('Unexpected fixture request'); return route.abort(); }
      const body = route.request().postDataJSON(); requests.push(body);
      if (mode !== 'refusals' && body.params.targetId === 'target-B') { held.push(route); return; }
      return route.fulfill({ json: response(body) });
    });
    const focus = () => page.getByRole('button', { name: 'Focus', exact: true });
    const until = async (predicate, label) => { const end = Date.now() + 5000; while (!predicate() && Date.now() < end) await page.waitForTimeout(20); assert(predicate(), label); };
    try {
      page.setDefaultTimeout(5000);
      await page.goto(origin);
      if (mode === 'refusals') {
        assert(!(await focus().isDisabled()), 'A current owned secondary tab must offer Focus');
        for (const invalid of ['missing', 'foreign', 'stale', 'unknown', 'absent', 'wrong-profile']) {
          await page.getByRole('button', { name: `Select ${invalid}`, exact: true }).click();
          assert(await focus().isDisabled(), `${invalid} must refuse focus`);
          assert.match(await focus().getAttribute('title'), /current owned browser tab/);
        }
        assert.equal(requests.length, 0);
        await page.getByRole('button', { name: 'Select B', exact: true }).click(); await focus().click();
        await page.getByText('Focus request accepted for the selected tab.', { exact: true }).waitFor();
        assert.deepEqual(requests.map(request => request.params.targetId), ['target-B']);
      } else {
        await focus().click(); await until(() => held.length === 1, 'B held');
        await page.locator('[data-workspace-focus-pending="true"]').waitFor(); assert(await focus().isDisabled());
        if (mode === 'failure') {
          await held[0].fulfill({ status: 403, json: { success: false, error: 'PRIVATE-PROVIDER-DETAIL' } });
          await page.getByText('Focus request could not be confirmed. Refresh the selected workspace before trying again.', { exact: true }).waitFor();
          assert(!(await page.locator('body').innerText()).includes('PRIVATE-PROVIDER-DETAIL'));
        } else {
          await page.getByRole('button', { name: 'Select A', exact: true }).click(); await focus().click();
          assert.equal(requests.length, 1, 'same-display focus requests serialize');
          if (mode === 'queued-expiry') await page.evaluate(() => { const expired = Date.now() + 31_000; Date.now = () => expired; });
          if (mode === 'queued-unmount') await page.evaluate(() => window.dispose());
          if (mode === 'queued-stale') await page.getByRole('button', { name: 'Select B', exact: true }).click();
          await held[0].fulfill({ status: 403, json: { success: false, error: 'OLD-SELECTION-FAILURE' } });
          if (mode === 'selection-race') {
            await until(() => requests.length === 2, 'A follows B');
            await page.getByText('Focus request accepted for the selected tab.', { exact: true }).waitFor();
            assert.deepEqual(requests.map(request => request.params.targetId), ['target-B', 'target-A']);
          } else { await page.waitForTimeout(150); assert.equal(requests.length, 1, 'stale or unmounted queued selection sends no request'); }
          assert(!(await page.locator('body').innerText()).includes('OLD-SELECTION-FAILURE'));
        }
      }
      for (const body of requests) { assert.equal(body.action, 'remote_view_open'); assert.equal(body.browserId, 'browser'); assert.equal(body.sessionName, 'owned-task'); assert.equal(body.params.displayAllocationId, 'desktop'); assert(!('targetId' in body), 'Target belongs in the existing action params only'); }
      assert.deepEqual(errors, []); fs.writeFileSync(path.join(artifacts, mode + '.json'), JSON.stringify({ requests, errors }, null, 2));
      console.log(`${mode}: passed`);
    } finally { fs.writeFileSync(path.join(artifacts, mode + '-final.txt'), await page.locator('body').innerText()); await page.close(); }
  }
  try {
    browserServer = await chromium.launchServer({ executablePath: chrome, headless: true, args: ['--disable-background-networking', marker] });
    const process = browserServer.process(); const rootIdentity = statIdentity(process.pid); assert(rootIdentity); identities.set(process.pid, rootIdentity);
    fs.writeFileSync(path.join(artifacts, 'browser-root.json'), JSON.stringify(rootIdentity));
    browser = await chromium.connect(browserServer.wsEndpoint());
    const initialCdp = await browser.newBrowserCDPSession(); const initial = await initialCdp.send('SystemInfo.getProcessInfo'); await initialCdp.detach();
    for (const p of initial.processInfo) { const identity = statIdentity(p.id); if (identity) identities.set(p.id, identity); }
    fs.writeFileSync(path.join(artifacts, 'initial-identities.json'), JSON.stringify([...identities.values()]));
    for (const mode of ['refusals', 'failure', 'selection-race', 'queued-stale', 'queued-expiry', 'queued-unmount']) await scenario(mode);
  } finally {
    try {
      if (browser) { const cdp = await browser.newBrowserCDPSession(); const result = await cdp.send('SystemInfo.getProcessInfo'); await cdp.detach(); for (const p of result.processInfo) { const identity = statIdentity(p.id); if (identity) identities.set(p.id, identity); } await browser.close(); }
    } finally {
      if (browserServer) await browserServer.close();
      await new Promise(resolve => server.close(resolve));
      const remaining = [...identities.values()].filter(identity => statIdentity(identity.pid)?.start === identity.start);
      const markerReferences = [], denied = [];
      for (const pid of fs.readdirSync('/proc').filter(x => /^\d+$/.test(x))) try { if (fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').includes(marker)) markerReferences.push(pid); } catch (error) { if (!['ENOENT', 'ESRCH'].includes(error.code)) denied.push({ pid, code: error.code }); }
      fs.writeFileSync(path.join(artifacts, 'cleanup.json'), JSON.stringify({ identities: [...identities.values()], remaining, markerReferences, denied }, null, 2));
      assert.deepEqual(remaining, [], 'All retained fixture kernel generations must be absent'); assert.deepEqual(markerReferences, []);
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
