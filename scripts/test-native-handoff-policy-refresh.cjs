const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');

(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'p280-policy-refresh-'));
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/native-handoff-policy-refresh.tsx'), artifacts);
  const reads = [];
  let deadline = 1110;
  let hold = null;
  let notifyHeld;
  const server = http.createServer(async (req, res) => {
    if (req.url.startsWith('/api/service/')) {
      res.setHeader('Content-Type', 'application/json');
      if (req.url.startsWith('/api/service/status?')) {
        const observed = deadline;
        reads.push(req.headers);
        if (hold) { notifyHeld?.(); await hold; }
        return res.end(JSON.stringify({ success: true, data: {
          service_state: { browsers: {}, sessions: {}, tabs: {}, jobs: {} },
          browserRuntime: { workspace: { browsers: {}, sessions: {}, tabs: {}, handoffs: {
            'synthetic-link': { handoffId: 'synthetic-link', expiresAtMs: observed },
          } } },
        } }));
      }
      return res.end(JSON.stringify({ success: true, data: { resources: [] } }));
    }
    res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html');
    res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  let release;
  try {
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const expiry = page.getByLabel('Reported expiry');
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.waitForFunction(() => document.querySelector('output')?.textContent === '1110');
    assert.equal(reads.length, 1);
    assert.equal(reads[0]['x-agent-browser-revalidate'], undefined, 'ordinary initial polling retains shared cache policy');
    deadline = 2220;
    await page.getByRole('button', { name: 'Refresh handoffs' }).click();
    await page.waitForFunction(() => document.querySelector('output')?.textContent === '2220');
    assert.equal(reads.length, 2, 'explicit refresh cannot reuse the warm ten-second snapshot');
    assert.equal(reads[1]['x-agent-browser-revalidate'], '1');
    assert.equal(await expiry.innerText(), '2220');

    hold = new Promise(resolve => { release = resolve; });
    deadline = 3330;
    const heldAccepted = new Promise(resolve => { notifyHeld = resolve; });
    await page.reload();
    await heldAccepted;
    await page.waitForFunction(() => document.querySelector('[role=status]')?.textContent === 'Loading');
    assert.equal(reads.length, 3);
    deadline = 4440;
    await page.getByRole('button', { name: 'Refresh handoffs' }).click();
    assert.equal(reads.length, 3, 'manual refresh waits for the earlier poll rather than racing its publication');
    hold = null;
    release(); release = null;
    await page.waitForFunction(() => document.querySelector('output')?.textContent === '4440');
    assert.equal(reads.length, 4, 'a refresh requested during a poll must run after it settles');
    assert.equal(reads[3]['x-agent-browser-revalidate'], '1');
    assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'receipt.json'), JSON.stringify({ complete: true, deadlines: [1110, 2220, 4440], statusReads: reads.length, seam: 'real workspace hook and document coordinator; synthetic owner served over actual HTTP' }, null, 2));
    console.log(JSON.stringify({ success: true, artifacts }));
  } finally {
    release?.();
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
