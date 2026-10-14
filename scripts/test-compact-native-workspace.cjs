const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'p237-compact-workspace-'));
  // Use the existing TSX harness and the real workspace CSS below.
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/compact-native-workspace.tsx'), artifacts);
  const server = http.createServer((req, res) => {
    res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : req.url === '/native.css' ? 'text/css' : 'text/html');
    if (req.url === '/native.css') return res.end('body { margin: 0; } button,input { font: inherit; color: inherit; border: 0; background: transparent; } ' + fs.readFileSync(path.join(root, 'packages/dashboard/src/app/globals.css'), 'utf8').split('/* Native workspace keeps navigation compact and the desktop unobstructed. */')[1]);
    res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<link rel="stylesheet" href="/native.css"><div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  try {
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage();
    let ready = true;
    let frameAllowed = true;
    const requests = [];
    await page.route("**/api/service/request", async route => {
      const body = route.request().postDataJSON(); requests.push(body);
      if (body.action === "view_focus") return route.fulfill({ json: { success: true, data: { focused: true } } });
      return route.fulfill({ json: { success: true, data: { handoffId: "task-link", handoffUrl: `http://127.0.0.1:${server.address().port}/remote-view/task-link`, operatorVisible: { state: ready ? "ready" : "pending" } } } });
    });
    await page.route("**/remote-view/task-link", route => route.fulfill({ contentType: "text/html",
      headers: { 'Content-Security-Policy': frameAllowed ? "frame-ancestors 'self'" : "frame-ancestors 'none'" },
      body: '<body id="viewer" data-workspace="true"><p>Synthetic native viewer transport</p></body>' }));
    const errors = []; page.on('pageerror', e => errors.push(e.message));
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).waitFor();
    const rail = page.getByRole('complementary', { name: 'Browsers and tabs' });
    assert.equal(Math.round((await rail.boundingBox()).width), 224);
    const resize = page.getByRole('separator', { name: 'Browser rail width' });
    await resize.focus(); await page.keyboard.press('ArrowRight');
    assert.equal(await resize.getAttribute('aria-valuenow'), '232');
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).click();
    const selected = new URL(page.url());
    assert.equal(selected.searchParams.get('browser'), 'alice-browser');
    assert.equal(selected.searchParams.get('tab'), 'bob');
    assert.equal(selected.searchParams.get('session'), 'bob-session');
    assert.equal(selected.searchParams.get('view'), 'workspace:control');
    await page.getByTitle('Native Remote View desktop', { exact: true }).waitFor();
    await page.frameLocator('iframe[title="Native Remote View desktop"]').getByText('Synthetic native viewer transport', { exact: true }).waitFor();
    await page.getByRole('searchbox').fill('');
    const aliceFrame = page.waitForResponse('**/remote-view/task-link');
    await page.getByRole('button', { name: 'Alice invoice', exact: true }).click();
    await aliceFrame;
    frameAllowed = false;
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'native desktop' }).waitFor({ timeout: 2000 });
    assert.equal(await page.getByRole('link', { name: 'Open Remote View' }).getAttribute('href'), `http://127.0.0.1:${server.address().port}/remote-view/task-link`);
    frameAllowed = true;
    await page.getByRole('button', { name: 'Retry native desktop' }).click();
    await page.getByTitle('Native Remote View desktop', { exact: true }).waitFor();
    assert.ok(requests.some(request => request.action === 'view_focus' && request.params.operatorFocus === true));
    assert.ok(requests.some(request => request.action === 'remote_view_open'));
    await page.reload();
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).waitFor();
    assert.equal(await page.getByRole('button', { name: 'Bob invoice', exact: true }).getAttribute('aria-current'), 'page');
    assert.equal(Math.round((await rail.boundingBox()).width), 232, 'rail width must survive reload');
    assert.equal(await page.getByRole('button', { name: 'Closed invoice', exact: true }).count(), 0);
    await page.getByRole('searchbox', { name: 'Search browsers and tabs' }).fill('Alice');
    assert.equal(await page.getByRole('button', { name: 'Bob invoice', exact: true }).count(), 0);
    await page.getByRole('button', { name: 'Alice invoice', exact: true }).waitFor();
    ready = false;
    await page.getByRole('button', { name: 'Alice invoice', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'native desktop' }).waitFor();
    assert.equal(await page.getByTitle('Native Remote View desktop', { exact: true }).count(), 0, 'unready handoff must not become a viewer iframe');
    ready = true;
    await page.getByRole('button', { name: 'Retry native desktop' }).click();
    await page.getByTitle('Native Remote View desktop', { exact: true }).waitFor();
    await page.getByRole('searchbox').fill('no-such-task');
    await page.getByRole('status').filter({ hasText: 'No matching browsers or tabs' }).waitFor({ timeout: 1500 });
    await page.getByRole('button', { name: 'Collapse browser rail' }).click();
    assert.equal(await page.getByRole('searchbox').count(), 0);
    await page.getByRole('region', { name: 'Native desktop' }).waitFor();
    await page.getByRole('button', { name: 'Show browser rail' }).click();
    await page.getByRole('searchbox').waitFor();
    await page.getByRole('searchbox').fill('');
    await page.screenshot({ path: path.join(artifacts, 'workspace-desktop.png') });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: path.join(artifacts, 'workspace-mobile.png') });
    assert.ok((await page.getByRole('region', { name: 'Native desktop' }).boundingBox()).width > 0);
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ success: true, artifacts, seam: 'actual compact component in Chromium; synthetic inventory and viewer boundary' }));
  } finally {
    if (browser) {
      const session = await browser.newBrowserCDPSession();
      const { processInfo } = await session.send('SystemInfo.getProcessInfo');
      await session.detach();
      const identities = processInfo.flatMap(process => {
        try { const raw = fs.readFileSync(`/proc/${process.id}/stat`, 'utf8'); return [{ pid: process.id, start: raw.slice(raw.lastIndexOf(')') + 2).split(' ')[19] }]; }
        catch { return []; }
      });
      await browser.close();
      const remaining = identities.filter(identity => {
        try { const raw = fs.readFileSync(`/proc/${identity.pid}/stat`, 'utf8'); const fields = raw.slice(raw.lastIndexOf(')') + 2).split(' '); return fields[0] !== 'Z' && fields[19] === identity.start; }
        catch { return false; }
      });
      fs.writeFileSync(path.join(artifacts, 'cleanup.json'), JSON.stringify({ identities, remaining }, null, 2));
      assert.deepEqual(remaining, [], 'task browser processes remain after close');
    }
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
