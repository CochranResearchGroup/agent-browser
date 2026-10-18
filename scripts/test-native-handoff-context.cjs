const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');

(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'handoff-context-'));
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/native-handoff-context.tsx'), artifacts);
  const server = http.createServer((req, res) => {
    res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html');
    res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  try {
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    const table = page.getByRole('table', { name: 'Retained handoffs' });
    await table.waitFor();
    const missing = table.getByRole('row').filter({ has: page.locator('a[href="/remote-view/missing-labels"]') });
    assert.equal(await missing.locator('strong').innerText(), 'qbo-example', 'missing creator context must still identify the browser work');
    const text = await missing.innerText();
    assert.ok(text.includes('Profile: qbo-example') && text.includes('Session: qbo-operator'));
    assert.ok(text.includes('Creator not recorded'));
    assert.ok(text.includes('Tab retention: Default policy'));
    assert.ok(text.includes('Task: Retained') && text.includes('Awaiting confirmed use'));
    assert.ok(!text.includes('Unknown') && !text.includes('· supplied'));
    assert.ok(!text.includes('Human:') && !text.includes('Agent:') && !text.includes('Service:'));
    const attributed = table.getByRole('row').filter({ hasText: 'example-session' });
    assert.equal(await attributed.locator('strong').innerText(), 'Open accounts');
    assert.ok((await attributed.innerText()).includes('Agent: Codex'));
    assert.ok(!(await attributed.innerText()).includes('Human:'));
    await page.getByRole('button', { name: 'Sort by task name, ascending' }).click();
    assert.deepEqual(await table.locator('tbody tr strong').allTextContents(), ['Browser handoff', 'Open accounts', 'qbo-example']);
    assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'receipt.json'), JSON.stringify({ success: true, cases: ['missing identity with job only', 'supplied agent', 'legacy record', 'sort fallback titles'] }, null, 2));
    console.log(JSON.stringify({ success: true, artifacts }));
  } finally {
    if (browser) await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
