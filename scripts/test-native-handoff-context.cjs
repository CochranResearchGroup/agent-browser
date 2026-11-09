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
    if (req.url === '/styles.css') {
      res.setHeader('Content-Type', 'text/css');
      const chunks = path.join(root, 'packages/dashboard/out/_next/static/chunks');
      return res.end(fs.readdirSync(chunks).filter(name => name.endsWith('.css')).map(name => fs.readFileSync(path.join(chunks, name), 'utf8')).join('\n'));
    }
    res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html');
    res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><link rel="stylesheet" href="/styles.css"><div id="root"></div><script src="/bundle.js"></script>');
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
    assert.ok(text.includes('qbo-example'));
    assert.ok(!text.includes('Session:') && !text.includes('Tab retention:') && !text.includes('Creator not recorded'), 'technical context stays out of compact rows');
    await missing.getByRole('button', { name: 'Inspect', exact: true }).click();
    const details = table.locator('.native-handoff-details');
    assert.ok((await details.innerText()).includes('qbo-operator'));
    assert.ok((await details.innerText()).includes('Default policy'));
    await missing.getByRole('button', { name: 'Inspect', exact: true }).click();
    const search = page.getByRole('searchbox', { name: 'Search handoffs' });
    await search.pressSequentially('AcCoUnTs');
    assert.deepEqual(await table.locator('tbody tr strong').allTextContents(), ['Open accounts']);
    await search.fill('  CODEX example-session  ');
    assert.deepEqual(await table.locator('tbody tr strong').allTextContents(), ['Open accounts'], 'search includes collapsed creator/session context and trims whitespace');
    await search.fill('missing-labels');
    assert.deepEqual(await table.locator('tbody tr strong').allTextContents(), ['qbo-example'], 'link IDs remain searchable');
    await search.fill('not-a-match');
    await page.getByText('No matching handoffs. Try another search or show inactive links.').waitFor();
    assert.equal(await table.count(), 0);
    await search.fill('archived');
    await page.getByText('No matching handoffs. Try another search or show inactive links.').waitFor();
    await page.getByRole('checkbox', { name: /Show inactive/ }).check();
    assert.deepEqual(await table.locator('tbody tr strong').allTextContents(), ['archived-profile']);
    await page.getByRole('checkbox', { name: /Show inactive/ }).uncheck();
    await page.getByRole('button', { name: 'Clear search' }).click();
    assert.equal(await search.inputValue(), '');
    assert.equal(await search.evaluate(node => node === document.activeElement), true);
    assert.equal(await table.locator('tbody tr strong').count(), 3);
    await page.getByRole('button', { name: 'Sort by task name, ascending' }).click();
    assert.deepEqual(await table.locator('tbody tr strong').allTextContents(), ['Browser handoff', 'Open accounts', 'qbo-example']);
    await page.screenshot({ path: path.join(artifacts, 'desktop.png') });
    for (const width of [390, 768]) {
      await page.setViewportSize({ width, height: 844 });
      const dialog = page.getByRole('dialog');
      const box = await dialog.boundingBox();
      assert.ok(box.x >= 0 && box.x + box.width <= width, `dialog fits ${width}px viewport`);
      assert.equal(await dialog.evaluate(node => node.scrollWidth <= node.clientWidth), true, 'modal has no horizontal overflow');
      assert.ok((await table.locator('thead').boundingBox()).height < 80, 'sort labels must not collapse into vertical letter stacks');
      await page.screenshot({ path: path.join(artifacts, `viewport-${width}.png`) });
    }
    assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'receipt.json'), JSON.stringify({ success: true, cases: ['compact rows', 'inspect full context', 'instant search', 'hidden metadata search', 'link ID search', 'empty results', 'clear and focus', 'sort fallback titles', 'inactive search composition', 'responsive layout'] }, null, 2));
    console.log(JSON.stringify({ success: true, artifacts }));
  } finally {
    if (browser) await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
