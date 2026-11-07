const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');

(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'native-tab-continuity-'));
  let browser;
  const requests = [];
  let viewerLoads = 0;
  const server = http.createServer(async (req, res) => {
    if (req.url === '/api/service/request') {
      let body = ''; for await (const part of req) body += part;
      const request = JSON.parse(body); requests.push(request);
      res.setHeader('Content-Type', 'application/json');
      res.end(JSON.stringify({ success: true, data: {
        handoffId: request.params.targetId, handoffUrl: '/remote-view/' + request.params.targetId,
        operatorVisible: { state: 'ready' },
      } }));
    } else if (req.url.startsWith('/remote-view/')) {
      viewerLoads++;
      res.setHeader('Content-Type', 'text/html');
      res.end('<body id="viewer" data-workspace="true">Synthetic Native viewer</body>');
    } else {
      res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html');
      res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
    }
  });
  try {
    await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/native-task-selection.tsx'), artifacts);
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.frameLocator('iframe').getByText('Synthetic Native viewer').waitFor();
    await page.evaluate(() => { window.originalViewer = document.querySelector('iframe'); window.originalDocument = window.originalViewer.contentDocument; });
    const focused = page.waitForResponse(response => response.url().endsWith('/api/service/request')
      && response.request().postDataJSON()?.params?.targetId === 'beta');
    await page.getByRole('button', { name: 'Beta', exact: true }).click();
    await focused;
    await page.waitForFunction(() => document.querySelector('[data-native-task-state="opened"]'));
    assert(await page.evaluate(() => window.originalViewer === document.querySelector('iframe') && window.originalDocument === window.originalViewer.contentDocument),
      'switching tabs on the same desktop must preserve the connected viewer node and document');
    assert.equal(viewerLoads, 1, 'same-desktop tab switch must not navigate or reload Remote View');
    assert.equal(requests.at(-1).params.targetId, 'beta', 'retaining the viewer must still focus the selected tab');
    console.log(JSON.stringify({ success: true, viewerLoads, focusedTarget: requests.at(-1).params.targetId }));
  } finally {
    await browser?.close();
    await new Promise(resolve => server.close(resolve));
    fs.rmSync(artifacts, { recursive: true, force: true });
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
