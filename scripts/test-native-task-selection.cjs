const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'native-task-selection-'));
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/native-task-selection.tsx'), artifacts);
  const requests = [];
  let hold, accepted, denyNext = false, denyCode = null, denyStatus = 200, viewerMode = 'ready';
  const accessServer = http.createServer((req, res) => { res.setHeader('Content-Type', 'text/html'); res.end('<body>Synthetic access page</body>'); });
  await new Promise(resolve => accessServer.listen(0, '127.0.0.1', resolve));
  const server = http.createServer(async (req, res) => {
    if (req.url === '/api/service/request') {
      let raw = ''; for await (const part of req) raw += part;
      const command = JSON.parse(raw); requests.push(command);
      if (hold) { accepted?.(); await hold; }
      res.setHeader('Content-Type', 'application/json');
      if (denyCode) { const code = denyCode; denyCode = null; res.statusCode = denyStatus; return res.end(JSON.stringify({ success: false, error: 'PRIVATE-OPERATOR-ERROR', failure: {
        schemaVersion: 'agent-browser.service-failure-recourse.v1', code, axis: 'profile_access', phase: 'ingress_validation',
        effectState: 'no_effect', retryDisposition: 'inspect_before_retry', recommendedAction: 'inspect_operator_focus_authority', reuseAllowed: false,
      } })); }
      if (denyNext) { denyNext = false; return res.end(JSON.stringify({ success: false, error: 'operator_focus_browser_unavailable' })); }
      res.end(JSON.stringify({ success: true, data: { handoffId: command.params.targetId,
        handoffUrl: '/remote-view/' + command.params.targetId, operatorVisible: { state: 'ready' } } }));
    } else if (req.url.startsWith('/remote-view/')) {
      if (viewerMode === 'access') {
        res.writeHead(302, { Location: `http://127.0.0.1:${accessServer.address().port}/sign-in` }); return res.end();
      }
      res.setHeader('Content-Type', 'text/html');
      if (viewerMode === 'page') { res.statusCode = 503; return res.end('<body>PRIVATE-NATIVE-ERROR Service Unavailable</body>'); }
      res.end('<body id="viewer" data-workspace="true">Synthetic Native boundary</body>');
    } else {
      res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html');
      res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
    }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser, release;
  try {
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage(); const errors = []; page.on('pageerror', e => errors.push(e.message));
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.waitForFunction(() => document.querySelector('iframe')?.getAttribute('src')?.endsWith('/alpha'));
    assert.deepEqual(requests.map(x => x.action), ['remote_view_open'], 'one native handoff must perform the authoritative focus; do not pre-focus the same selection');
    assert.equal(requests[0].params.maximize, true, 'combined handoff must preserve native maximize');
    await page.evaluate(() => { window.verifiedNativeFrame = document.querySelector('iframe'); });
    await page.getByRole('button', { name: 'Expire census', exact: true }).click();
    assert.equal(await page.locator('iframe').count(), 1, 'aging descriptive census must not tear down a verified Native connection');
    assert(await page.evaluate(() => window.verifiedNativeFrame === document.querySelector('iframe')), 'retain the exact iframe node while census freshness is unknown');
    await page.getByRole('button', { name: 'Fresh census', exact: true }).click();
    assert.equal(requests.length, 1, 'freshness recovery must not repeat an already-authorized handoff');

    await page.getByRole('button', { name: 'Expire census', exact: true }).click();
    await page.getByRole('button', { name: 'Beta', exact: true }).click();
    assert.equal(await page.locator('iframe').count(), 0, 'stale selection cannot retain another task viewer');
    assert.equal(requests.length, 1, 'stale selection cannot authorize another handoff');
    await page.getByRole('button', { name: 'Alpha', exact: true }).click();
    await page.getByRole('button', { name: 'Fresh census', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('iframe')?.getAttribute('src')?.endsWith('/alpha'));

    hold = new Promise(resolve => { release = resolve; });
    const pending = new Promise(resolve => { accepted = resolve; });
    await page.getByRole('button', { name: 'Beta', exact: true }).click(); await pending;
    await page.getByRole('button', { name: 'Expire census', exact: true }).click();
    await page.getByRole('button', { name: 'Fresh census', exact: true }).click();

    await page.getByRole('button', { name: 'Alpha', exact: true }).click();
    hold = null; release(); release = null;
    await page.waitForFunction(() => document.querySelector('iframe')?.getAttribute('src')?.endsWith('/alpha'));
    assert.deepEqual(requests.map(x => x.params.targetId), ['alpha', 'alpha', 'beta', 'alpha'], 'latest selection must execute after earlier in-flight native effects');

    await page.getByRole('button', { name: 'Confirm absence', exact: true }).click();
    assert.equal(await page.locator('iframe').count(), 0, 'a current confirmed absence must clear the viewer');
    await page.getByRole('button', { name: 'Fresh census', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('iframe')?.getAttribute('src')?.endsWith('/alpha'));
    denyNext = true;
    await page.getByRole('button', { name: 'Beta', exact: true }).click();
    await page.locator('[data-native-task-state=failed]').waitFor();
    assert.equal(await page.locator('iframe').count(), 0, 'unknown presence cannot become a viewer after authoritative readiness fails');
    assert.equal(await page.locator('[data-native-task-state=failed]').getAttribute('data-native-task-failure'), 'handoff');
    assert.equal(await page.getByRole('link', { name: 'Open Remote View' }).count(), 0, 'failed preparation has no fabricated handoff');
    assert.deepEqual(errors, []);
    const beforeStale = requests.length;
    const stalePage = await browser.newPage();
    await stalePage.goto(`http://127.0.0.1:${server.address().port}/?stale=1`);
    await stalePage.locator('[data-native-task-state=unavailable]').waitFor();
    assert.equal(requests.length, beforeStale, 'initial stale metadata cannot authorize a handoff');
    assert.equal(await stalePage.locator('iframe').count(), 0);
    await stalePage.getByRole('button', { name: 'Fresh census', exact: true }).click();
    await stalePage.waitForFunction(() => document.querySelector('iframe')?.getAttribute('src')?.endsWith('/alpha'));
    assert.equal(requests.length, beforeStale + 1);
    await stalePage.close();
    for (const [code, status] of [['operator_focus_profile_policy_denied', 400], ['operator_focus_authority_required', 200]]) {
      denyCode = code; denyStatus = status;
      await page.goto(`http://127.0.0.1:${server.address().port}`);
      const denied = page.locator('[data-native-task-failure="operator-access"]');
      await denied.getByRole('alert').waitFor();
      assert((await denied.innerText()).includes('Check dashboard and profile access'));
      assert.equal((await denied.innerText()).includes('PRIVATE-OPERATOR-ERROR'), false);
      assert.equal(await page.locator('iframe').count(), 0);
      assert.equal(await page.getByRole('link', { name: 'Open Remote View' }).count(), 0);
      const count = requests.length;
      await page.waitForTimeout(100);
      assert.equal(requests.length, count, 'operator policy refusal must not replay focus');
    }
    for (const [mode, reason, message] of [
      ['page', 'viewer-page', 'The Remote View desktop page is unavailable'],
      ['access', 'viewer-access', 'Check Remote View sign-in or access'],
    ]) {
      viewerMode = mode;
      await page.goto(`http://127.0.0.1:${server.address().port}`);
      const failure = page.locator('[data-native-task-state=failed]');
      await failure.waitFor();
      assert.equal(await failure.getAttribute('data-native-task-failure'), reason,
        'page and access failures must not collapse into a handoff failure');
      assert((await failure.innerText()).includes(message));
      assert.equal((await failure.innerText()).includes('PRIVATE-NATIVE-ERROR'), false);
      assert.equal(await failure.getByRole('link', { name: 'Open Remote View' }).getAttribute('href'),
        `http://127.0.0.1:${server.address().port}/remote-view/alpha`, 'retain only the opaque handoff');
      const count = requests.length;
      await page.waitForTimeout(100);
      assert.equal(requests.length, count, 'failed embedding must not automatically replay focus');
      await failure.getByRole('button', { name: 'Refresh browsers', exact: true }).click();
      assert.equal(await page.getByLabel('Refresh count').textContent(), '1');
      assert.equal(requests.length, count, 'inventory refresh alone must not replay focus');
      viewerMode = 'ready';
      await failure.getByRole('button', { name: 'Retry native desktop' }).click();
      await page.frameLocator('iframe').getByText('Synthetic Native boundary').waitFor();
      assert.equal(requests.length, count + 1, 'one explicit retry prepares one handoff');
      assert.equal(await page.locator('[data-native-task-state=failed]').count(), 0);
    }
    assert.deepEqual(errors, [], 'failure and recovery paths must not raise page errors');
    console.log(JSON.stringify({ success: true, cases: ['single focus through handoff', 'maximize', 'same frame through census expiry', 'initial stale metadata blocked', 'changed stale selection clears viewer', 'confirmed absence clears viewer', 'latest selection after held request', 'unknown selection denied without iframe', 'readable 503 page is distinct', 'cross-origin access stays unverified', 'explicit retry and refresh', 'producer errors redacted', 'typed operator policy remains distinct'] }));
  } finally { release?.(); if (browser) await browser.close(); await new Promise(resolve => server.close(resolve)); await new Promise(resolve => accessServer.close(resolve)); }
})().catch(e => { console.error(e); process.exitCode = 1; });
