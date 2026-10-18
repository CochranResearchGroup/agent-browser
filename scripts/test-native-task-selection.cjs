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
  let hold, accepted, denyNext = false;
  const server = http.createServer(async (req, res) => {
    if (req.url === '/api/service/request') {
      let raw = ''; for await (const part of req) raw += part;
      const command = JSON.parse(raw); requests.push(command);
      if (hold) { accepted?.(); await hold; }
      res.setHeader('Content-Type', 'application/json');
      if (denyNext) { denyNext = false; return res.end(JSON.stringify({ success: false, error: 'operator_focus_browser_unavailable' })); }
      res.end(JSON.stringify({ success: true, data: { handoffId: command.params.targetId,
        handoffUrl: '/remote-view/' + command.params.targetId, operatorVisible: { state: 'ready' } } }));
    } else if (req.url.startsWith('/remote-view/')) {
      res.setHeader('Content-Type', 'text/html'); res.end('<body id="viewer" data-workspace="true">Synthetic Native boundary</body>');
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
    hold = new Promise(resolve => { release = resolve; });
    const pending = new Promise(resolve => { accepted = resolve; });
    await page.getByRole('button', { name: 'Beta', exact: true }).click(); await pending;
    await page.getByRole('button', { name: 'Alpha', exact: true }).click();
    hold = null; release(); release = null;
    await page.waitForFunction(() => document.querySelector('iframe')?.getAttribute('src')?.endsWith('/alpha'));
    assert.deepEqual(requests.map(x => x.params.targetId), ['alpha', 'beta', 'alpha'], 'latest selection must execute after earlier in-flight native effects');
    denyNext = true;
    await page.getByRole('button', { name: 'Beta', exact: true }).click();
    await page.locator('[data-native-task-state=failed]').waitFor();
    assert.equal(await page.locator('iframe').count(), 0, 'unknown presence cannot become a viewer after authoritative readiness fails');
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ success: true, cases: ['single focus through handoff', 'maximize', 'latest selection after held request', 'unknown selection denied without iframe'] }));
  } finally { release?.(); if (browser) await browser.close(); await new Promise(resolve => server.close(resolve)); }
})().catch(e => { console.error(e); process.exitCode = 1; });
