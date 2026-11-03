const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'p524-auth-gate-'));
  let browser;
  const server = http.createServer((req, res) => {
    res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html');
    res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(scratch, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
  });
  try {
    await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/workspace-auth-gate.tsx'), scratch);
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage();
    let status = 401;
    let redirected = false;
    await page.route('**/api/dashboard-auth/status', route => route.fulfill(redirected
      ? { status: 200, contentType: 'text/html', body: '<html>Auth portal</html>' }
      : { status, contentType: 'application/json', body: JSON.stringify({ authenticated: status === 200, user: status === 200 ? { username: 'reader', displayName: 'Reader', role: 'observer' } : null }) }));
    const url = `http://127.0.0.1:${server.address().port}/?browser=selected-task`;
    await page.goto(url);
    await page.getByText('Sign in through the authenticated workspace URL', { exact: false }).waitFor();
    assert.equal(await page.locator('input[type=password]').count(), 0);
    status = 403;
    await page.getByRole('button', { name: 'Retry workspace access' }).click();
    await page.getByText('Access denied.', { exact: false }).waitFor();
    assert.equal(page.url(), url);
    redirected = true;
    await page.getByRole('button', { name: 'Retry workspace access' }).click();
    await page.getByText('Workspace sign-in redirect:', { exact: false }).waitFor();
    redirected = false;
    status = 503;
    await page.getByRole('button', { name: 'Retry workspace access' }).click();
    await page.getByText('HTTP 503', { exact: false }).waitFor();
    status = 200;
    await page.getByRole('button', { name: 'Retry workspace access' }).click();
    await page.getByText('Selected browser retained for reader').waitFor();
    assert.equal(page.url(), url);
    await page.reload();
    await page.getByText('Selected browser retained for reader').waitFor();
    console.log('Mounted workspace ingress gate: sign-in, denied, redirect, unavailable and existing session passed');
  } finally {
    if (browser) await browser.close();
    server.close();
    fs.rmSync(scratch, { recursive: true, force: true });
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
