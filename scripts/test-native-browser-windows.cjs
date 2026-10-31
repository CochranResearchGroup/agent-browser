const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'native-browser-windows-'));
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/native-browser-windows.tsx'), artifacts);
  let generation = 7;
  let unavailable = false;
  let failure = null;
  const effects = [];
  const serviceRequests = [];
  let releaseFirst;
  const first = new Promise(resolve => { releaseFirst = resolve; });
  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url, 'http://fixture');
    if (url.pathname.startsWith('/api/service')) serviceRequests.push(url.pathname);
    if (url.pathname === '/api/v1/desktops/browser-windows') {
      res.setHeader('Content-Type', 'application/json');
      if (failure === 'html') { res.setHeader('Content-Type', 'text/html'); return res.end('<html>Sign in</html>'); }
      if (failure) { res.statusCode = failure; return res.end('{}'); }
      if (unavailable) { res.statusCode = 503; return res.end('{}'); }
      return res.end(JSON.stringify({ schemaVersion: 1, source: 'remote-view-native-windows', observedAtMs: Date.now(), observationAgeMs: 0, unverifiedDesktopCount: 0,
        desktops: [{ desktopId: '11111111-1111-4111-8111-111111111111', slot: 2, generation, displayName: 'Research', application: 'research',
          windows: [{ id: 31, pid: 500, title: 'Alpha browser', class: 'Chromium' }, { id: 32, pid: 501, title: 'Beta browser', class: 'Firefox' }] },
        { desktopId: '22222222-2222-4222-8222-222222222222', slot: 3, generation: 1, displayName: 'Empty desktop', windows: [] }] }));
    }
    if (url.pathname === '/native-test-focus') {
      let body = ''; for await (const chunk of req) body += chunk;
      const effect = JSON.parse(body); effects.push(effect);
      if (effects.length === 1) await first;
      res.setHeader('Content-Type', 'application/json'); return res.end('{}');
    }
    if (url.pathname === '/2') {
      res.setHeader('Content-Type', 'text/html'); res.setHeader('Content-Security-Policy', "frame-ancestors 'self'");
      return res.end(`<body id="viewer" data-workspace="true"><p>Synthetic native boundary</p><script>window.RemoteViewWorkspace={isConnected:()=>true,activateWindow:async(generation,windowId)=>{if(generation!==${generation})throw Error('stale_generation');await fetch('/native-test-focus',{method:'POST',body:JSON.stringify({generation,windowId})})}};</script></body>`);
    }
    res.setHeader('Content-Type', url.pathname === '/bundle.js' ? 'application/javascript' : url.pathname === '/native.css' ? 'text/css' : 'text/html');
    if (url.pathname === '/bundle.js') return res.end(fs.readFileSync(path.join(artifacts, 'bundle.js')));
    if (url.pathname === '/native.css') return res.end('body{margin:0}button,input{font:inherit}' + fs.readFileSync(path.join(root, 'packages/dashboard/src/app/globals.css'), 'utf8').split('/* Native workspace keeps navigation compact and the desktop unobstructed. */')[1]);
    res.end('<link rel="stylesheet" href="/native.css"><div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', headless: true });
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${server.address().port}`, { waitUntil: 'domcontentloaded' });
    await page.getByRole('button', { name: 'Alpha browser', exact: true }).waitFor();
    assert.equal(await page.getByText('Empty desktop', { exact: true }).count(), 0);
    assert.equal(await page.locator('.native-workspace-browser').count(), 1);
    await page.getByRole('button', { name: 'Alpha browser', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('iframe')?.contentWindow?.RemoteViewWorkspace?.isConnected());
    await page.frameLocator('iframe').locator('body').evaluate(body => { body.dataset.continuity = 'keep'; });
    const focusDeadline = Date.now() + 3000;
    while (!effects.length && Date.now() < focusDeadline) await new Promise(resolve => setTimeout(resolve, 20));
    assert.ok(effects.length, "native window focus was not requested");
    await page.getByRole('button', { name: 'Beta browser', exact: true }).click();
    releaseFirst();
    await page.locator('[data-native-window-state="opened"]').waitFor();
    assert.deepEqual(effects, [{ generation: 7, windowId: 31 }, { generation: 7, windowId: 32 }]);
    assert.equal(await page.frameLocator('iframe').locator('body').getAttribute('data-continuity'), 'keep', 'same-desktop selection must preserve the native connection');
    assert.equal(await page.getByRole('button', { name: 'Beta browser', exact: true }).getAttribute('aria-current'), 'page');
    generation = 8;
    await page.getByRole('button', { name: 'Refresh browsers', exact: true }).click();
    await page.getByText('This browser window could not be verified. Refresh browsers to check again.', { exact: true }).waitFor();
    assert.equal(await page.locator('iframe').count(), 0, 'new generation must invalidate the prior exact selection');
    assert.equal(effects.length, 2, 'refresh must never activate a replacement window');
    await page.getByRole('button', { name: 'Alpha browser', exact: true }).click();
    await page.locator('[data-native-window-state="opened"]').waitFor();
    assert.deepEqual(effects[2], { generation: 8, windowId: 31 });
    const selectedUrl = page.url();
    for (const [status, message] of [
      [401, 'Sign in to Remote View to load browser windows. Your dashboard sign-in does not sign you in to Remote View. After signing in, refresh browsers.'],
      [403, 'Remote View denied access to browser windows. Check your account permissions before refreshing browsers.'],
      ['html', 'Remote View returned a web page instead of browser windows. Check Remote View sign-in and access, then refresh browsers.'],
    ]) {
      failure = status;
      await page.getByRole('button', { name: 'Refresh browsers', exact: true }).click();
      await page.getByText(message, { exact: true }).waitFor({ timeout: 3000 });
      assert.equal(await page.getByText('Browser presence could not be verified. Refresh browsers to check again.', { exact: true }).count(), 0);
      assert.equal(await page.locator('iframe').count(), 0);
      if (status === 401) {
        await page.getByRole('button', { name: 'Collapse browser rail', exact: true }).click();
        assert.equal(await page.getByRole('status').filter({ hasText: message }).isVisible(), true, 'sign-in guidance remains visible with a collapsed rail');
        await page.getByRole('button', { name: 'Show browser rail', exact: true }).click();
      }
      assert.equal(page.url(), selectedUrl, 'access failures must preserve requested selection');
    }
    await page.route('**/api/v1/desktops/browser-windows?*', route => route.abort('failed'));
    await page.getByRole('button', { name: 'Refresh browsers', exact: true }).click();
    await page.getByText('Could not load Remote View browser windows. Check Remote View sign-in and access, or the service connection, then refresh browsers.', { exact: true }).waitFor({ timeout: 3000 });
    await page.unroute('**/api/v1/desktops/browser-windows?*');
    failure = null;
    await page.getByRole('button', { name: 'Refresh browsers', exact: true }).click();
    await page.getByRole('button', { name: 'Alpha browser', exact: true }).waitFor();
    await page.getByRole('status').filter({ hasText: 'Remote View denied access' }).waitFor({ state: 'detached' });
    unavailable = true;
    await page.getByRole('button', { name: 'Refresh browsers', exact: true }).click();
    await page.getByText('Remote View browser inventory is unavailable. Managed tasks remain listed separately.', { exact: true }).waitFor();
    assert.equal(await page.locator('iframe').count(), 0);
    assert.deepEqual(serviceRequests, [], 'native observation must not create managed tasks or invoke full diagnostics');
    console.log(JSON.stringify({ success: true, effects: effects.length, source: 'real native-window components and hooks; synthetic provider boundary', artifacts }));
  } finally { releaseFirst(); await browser.close(); await new Promise(resolve => server.close(resolve)); }
})().catch(error => { console.error(error); process.exitCode = 1; });
