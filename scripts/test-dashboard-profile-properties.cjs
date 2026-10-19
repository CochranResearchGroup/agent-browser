const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'dashboard-profile-properties-'));
  let browser; let server;
  try {
    await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/profile-properties.tsx'), artifacts);
    server = http.createServer((req, res) => {
      res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : req.url === '/style.css' ? 'text/css' : 'text/html');
      res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : req.url === '/style.css' ? fs.readdirSync(path.join(root, 'packages/dashboard/.next/static/chunks')).filter(name => name.endsWith('.css')).map(name => fs.readFileSync(path.join(root, 'packages/dashboard/.next/static/chunks', name))).join('\n') : '<link rel="stylesheet" href="/style.css"><div id="root"></div><script src="/bundle.js"></script>');
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    browser = await chromium.launch({ executablePath: '/opt/google/chrome/chrome', headless: true });
    const page = await browser.newPage();
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    const requests = []; let fail = false;
    await page.route('**/api/dashboard/profile-properties', route => {
      const request = route.request(); const body = request.postDataJSON(); requests.push(body);
      assert.equal(request.method(), 'PATCH'); assert.equal(body.profileId, 'stable-profile');
      return fail ? route.fulfill({ status: 409, json: { success: false } }) : route.fulfill({ json: { success: true, data: { profile: { id: body.profileId, name: body.name, kind: body.promote ? 'named' : body.expectedKind } } } });
    });
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    const actions = page.getByRole('button', { name: 'Tab actions for Research' });
    await actions.click(); await page.getByRole('menuitem', { name: 'Properties', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Tab properties' }); await dialog.waitFor();
    await dialog.getByText('Original task attribution was not recorded', { exact: false }).waitFor();
    const creation = await dialog.locator('dl div').filter({ has: page.locator('dt', { hasText: /^Created$/ }) }).locator('dd').textContent();
    assert.notEqual(creation, 'Unknown');
    assert.equal(await dialog.locator('dt').filter({ hasText: /^Service$/ }).count(), 0, 'missing attribution is explained once instead of an Unknown wall');
    await dialog.getByText('Throwaway', { exact: false }).first().waitFor();
    assert.equal(requests.length, 0);
    await dialog.getByLabel('Profile name').fill('Research archive');
    await dialog.getByRole('button', { name: 'Save name', exact: true }).click();
    await dialog.getByText('Profile name saved.', { exact: true }).waitFor();
    assert.equal(requests[0].promote, false);
    await dialog.getByRole('button', { name: 'Make durable', exact: true }).click();
    await dialog.getByText('Profile is now durable.', { exact: false }).waitFor();
    assert.equal(requests[1].expectedName, 'Research archive');
    assert.equal(await dialog.getByRole('button', { name: 'Make durable', exact: true }).count(), 0);
    fail = true; await dialog.getByLabel('Profile name').fill('Another name');
    await dialog.getByRole('button', { name: 'Save name', exact: true }).click();
    await dialog.getByText('Profile change could not be confirmed.', { exact: false }).waitFor();
    assert.equal(requests[2].expectedKind, 'named');
    await page.keyboard.press('Escape'); await dialog.waitFor({ state: 'hidden' });
    await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Tab actions for Research');
    await actions.click(); await page.getByRole('menuitem', { name: 'Properties', exact: true }).click();
    await dialog.waitFor();
    assert.equal(await dialog.getByLabel('Profile name').inputValue(), 'Research archive', 'last confirmed receipt survives an older inventory snapshot');
    assert.equal(await dialog.getByRole('button', { name: 'Make durable', exact: true }).count(), 0);
    await page.setViewportSize({ width: 390, height: 760 });
    const bounds = await dialog.boundingBox();
    assert(bounds.x >= 0 && bounds.x + bounds.width <= 390, 'Properties fits narrow screens');
    assert.deepEqual(errors, []);
    console.log('Profile Properties mounted checks passed: creation, absent attribution, rename, promotion, conflict and focus.');
  } finally {
    await browser?.close(); if (server) await new Promise(resolve => server.close(resolve));
    fs.rmSync(artifacts, { recursive: true, force: true });
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
