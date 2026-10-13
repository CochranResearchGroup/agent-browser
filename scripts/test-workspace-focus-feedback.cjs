#!/usr/bin/env node
// Browser-visible feedback tests with deterministic synthetic HTTP delays.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
const { chromium } = require('playwright');

const root = path.resolve(__dirname, '..');
const args = process.argv.slice(2);
function argument(name) {
  const value = args[args.indexOf(name) + 1];
  assert.ok(args.includes(name) && value && path.isAbsolute(value), `${name} requires an absolute path`);
  return value;
}
const chrome = args.includes('--bundle-only') ? null : argument('--chrome');
const artifacts = argument('--artifact-dir');
fs.mkdirSync(artifacts, { recursive: true });


async function scenario(browser, origin, mode) {
  const page = await browser.newPage();
  const held = [];
  const requests = [];
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', async route => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.origin === origin && ['/', '/bundle.js'].includes(url.pathname)) return route.continue();
    let body;
    try { body = request.postDataJSON(); } catch {}
    if (body?.action === 'view_focus') {
      requests.push(body.params.targetId);
      if (body.params.targetId === 'target-alice') { held.push(route); return; }
    }
    return route.fulfill({ json: { success: true, data: {} } });
  });
  async function waitFor(predicate, description) {
    const deadline = Date.now() + 5000;
    while (!predicate() && Date.now() < deadline) await new Promise(resolve => setTimeout(resolve, 25));
    assert.ok(predicate(), `${description}: ${JSON.stringify({ requests, errors })}`);
  }
  const accepted = () => page.getByText('Queued view focus and maximize before opening the workspace viewport.', { exact: true }).waitFor({ timeout: 5000 });
  try {
    await page.goto(`${origin}/?view=workspace:control&browser=browser-a&tab=alice`);
    await waitFor(() => held.length === 1, 'Alice request');
    if (mode === 'queued-unmount') {
      await page.evaluate(() => { window.select('bob'); });
      await page.waitForTimeout(100);
      await page.evaluate(() => window.dispose());
      await held[0].fulfill({ json: { success: true, data: {} } });
      await page.waitForTimeout(150);
      assert.deepEqual(requests, ['target-alice'], 'an unmounted viewport must not start a queued focus request');
    } else if (mode === 'same-selection-refresh') {
      await page.evaluate(() => window.select('alice'));
      await page.waitForTimeout(100);
      await held[0].fulfill({ json: { success: true, data: {} } });
      await accepted();
      assert.equal(requests.length, 1, 'refreshing the same selection must preserve its pending request');
    } else {
      await page.evaluate(() => window.select('bob', 'browser-b'));
      await waitFor(() => requests.includes('target-bob'), 'Bob request');
      await accepted();
      if (mode === 'reselection') {
        await page.evaluate(() => window.select('alice'));
      }
      await held[0].fulfill({ json: { success: false, error: 'Old Alice rejection' } });
      if (mode === 'reselection') await waitFor(() => held.length === 2, 'new Alice request');
      await page.waitForTimeout(100);
      assert.ok(!(await page.locator('body').innerText()).includes('Old Alice rejection'), 'an older request must not replace current selection feedback');
      if (mode === 'reselection') {
        await held[1].fulfill({ json: { success: true, data: {} } });
        await accepted();
      }
    }
    assert.deepEqual(errors, [], 'the actual component must render without browser errors');
  } finally {
    fs.writeFileSync(path.join(artifacts, `${mode}.json`), JSON.stringify({ requests, errors, text: await page.locator('body').innerText() }, null, 2));
    await page.close();
  }
}

(async () => {
  const entry = args.includes('--entry') ? argument('--entry') : path.join(root, 'scripts/fixtures/workspace-focus-feedback.tsx');
  await buildWorkspaceViewportFixture(root, entry, artifacts);
  if (args.includes('--bundle-only')) return;
  const server = http.createServer((request, response) => {
    const bundle = request.url === '/bundle.js';
    response.setHeader('Content-Type', bundle ? 'application/javascript' : 'text/html; charset=utf-8');
    response.end(bundle ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  try {
    browser = await chromium.launch({ executablePath: chrome, headless: true, args: ['--disable-background-networking', '--p237-focus-feedback-fixture'] });
    for (const mode of ['changed-selection', 'same-selection-refresh', 'reselection', 'queued-unmount']) {
      await scenario(browser, `http://127.0.0.1:${server.address().port}`, mode);
      console.log(`${mode}: passed`);
    }
  } finally {
    try {
      if (browser) {
        let identities = [];
        try {
          const session = await browser.newBrowserCDPSession();
          const { processInfo } = await session.send('SystemInfo.getProcessInfo');
          await session.detach();
          identities = processInfo.flatMap(process => {
            try {
              const stat = fs.readFileSync(`/proc/${process.id}/stat`, 'utf8');
              return [{ pid: process.id, start: stat.slice(stat.lastIndexOf(')') + 2).split(' ')[19] }];
            } catch { return []; }
          });
        } finally {
          await browser.close();
        }
        const remaining = identities.filter(identity => {
          try {
            const stat = fs.readFileSync(`/proc/${identity.pid}/stat`, 'utf8');
            const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
            return fields[0] !== 'Z' && fields[19] === identity.start;
          } catch { return false; }
        });
        fs.writeFileSync(path.join(artifacts, 'cleanup.json'), JSON.stringify({ identities, remaining }, null, 2));
        assert.deepEqual(remaining, [], 'task browser processes must exit');
      }
    } finally {
      await new Promise(resolve => server.close(resolve));
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
