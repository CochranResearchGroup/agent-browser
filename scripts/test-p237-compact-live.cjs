// Production dashboard assets and service calls against a task-owned source host.
// Only the native provider's browser viewer transport is synthetic here.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { chromium } = require('playwright');
const https = require('node:https');
const config = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
(async () => {
  process.env.TMPDIR = path.join(config.root, 'ui-temporary');
  fs.mkdirSync(process.env.TMPDIR, { recursive: true });
  let browser;
  let page;
  const requests = [];
  const errors = [];
  const viewerRequests = [];
  const viewerResponses = [];
  let nativeFrames = 0;
  try {
    browser = await chromium.launch({ executablePath: config.chrome, headless: true, timeout: 10000, args: ['--disable-background-networking', '--no-proxy-server', '--host-resolver-rules=MAP p237-native-viewer.invalid 127.0.0.1', '--p237-ui-ordering-fixture'] });
    const cdp = await browser.newBrowserCDPSession();
    const { processInfo } = await cdp.send('SystemInfo.getProcessInfo');
    await cdp.detach();
    const owner = processInfo.find(process => process.type === 'browser');
    const raw = fs.readFileSync(`/proc/${owner.id}/stat`, 'utf8');
    const fields = raw.slice(raw.lastIndexOf(')') + 2).split(' ');
    fs.writeFileSync(path.join(config.root, 'ui-selection-owned-session.json'), JSON.stringify({ pid: owner.id, start: fields[19], session: Number(fields[3]), profileRoot: process.env.TMPDIR }));
    const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true });
    const separator = config.cookie.indexOf('=');
    await context.addCookies([{ name: config.cookie.slice(0, separator), value: config.cookie.slice(separator + 1), url: config.origin }]);
    page = await context.newPage();
    page.setDefaultTimeout(15000);
    page.on('pageerror', error => errors.push(error.message));
    page.on('requestfailed', request => viewerRequests.push({ url: request.url(), failure: request.failure() }));
    page.on('response', response => {
      if (response.request().isNavigationRequest()) viewerRequests.push({ url: response.url(), status: response.status(), location: response.headers().location });
      const location = new URL(response.url());
      if (location.origin === config.providerOrigin && location.pathname === '/1' && response.status() === 200) {
        nativeFrames += 1;
        viewerResponses.push({ workspace: location.searchParams.get('workspace'), csp: response.headers()['content-security-policy'] });
      }
    });
    page.on('request', request => {
      if (new URL(request.url()).origin !== config.origin || request.method() !== 'POST') return;
      let body; try { body = request.postDataJSON(); } catch {}
      if (body?.action) requests.push({ action: body.action, targetId: body.params?.targetId ?? null });
    });
    const scene = () => {
      const active = execFileSync('xdotool', ['getactivewindow'], { encoding: 'utf8' }).trim();
      return { activeWindow: active, title: execFileSync('xdotool', ['getwindowname', active], { encoding: 'utf8' }).trim() };
    };
    await page.goto(`${config.origin}/?workspace-ui=compact&view=workspace:control&browser=${encodeURIComponent(config.browserId)}&tab=${encodeURIComponent(config.tabs[0].id)}`);
    await page.getByRole('searchbox', { name: 'Search browsers and tabs' }).waitFor();
    const observations = [];
    for (const tab of config.tabs) {
      await page.getByRole('button', { name: tab.label, exact: true }).click();
      await page.getByTitle('Native Remote View desktop', { exact: true }).waitFor();
      const handle = await page.getByTitle('Native Remote View desktop', { exact: true }).elementHandle();
      const frameContext = await handle.contentFrame();
      await frameContext.getByText('Task-owned native viewer transport boundary', { exact: true }).waitFor();
      const deadline = Date.now() + 10000;
      while (!scene().title.includes(tab.expectedTitle) && Date.now() < deadline) await page.waitForTimeout(25);
      const current = scene();
      assert.ok(current.title.includes(tab.expectedTitle), JSON.stringify({ current, tab, errors, text: await page.locator('body').innerText() }));
      assert.equal(new URL(page.url()).searchParams.get('tab'), tab.id);
      const frame = await page.getByTitle('Native Remote View desktop', { exact: true }).getAttribute('src');
      assert.match(new URL(frame, config.origin).pathname, /^\/remote-view\/.+/);
      const link = new URL(frame, config.origin);
      // Node does not use Chrome's task-only host resolver. Address the owned
      // local TLS listener directly without cookies or following the redirect.
      const standalone = await new Promise((resolve, reject) => {
        const request = https.get({ hostname: '127.0.0.1', port: link.port, path: link.pathname,
          headers: { Host: link.host, 'Sec-Fetch-Dest': 'document' }, rejectUnauthorized: false }, response => {
          response.resume();
          response.on('end', () => resolve({ status: response.statusCode, location: response.headers.location }));
        });
        request.on('error', reject);
        request.setTimeout(15000, () => request.destroy(new Error('standalone locator timeout')));
      });
      assert.equal(standalone.status, 303);
      assert.equal(new URL(standalone.location).searchParams.get('workspace'), null, 'ordinary handoff changed to embedded mode');
      assert.deepEqual(scene(), current, 'read-only standalone handoff changed the native selection');
      observations.push({ tabId: tab.id, scene: current, durableFrame: true });
    }
    // A normal dashboard reload must restore auth and the exact selected task.
    await page.reload();
    await page.getByRole('searchbox', { name: 'Search browsers and tabs' }).waitFor();
    await page.getByTitle('Native Remote View desktop', { exact: true }).waitFor();
    assert.equal(await page.getByRole('button', { name: config.tabs.at(-1).label, exact: true }).getAttribute('aria-current'), 'page');
    assert.ok(nativeFrames > 0, 'the durable frame never reached the native provider viewer boundary');
    assert.ok(viewerResponses.every(reply => reply.workspace === 'true' && reply.csp === "frame-ancestors 'self'"), 'native workspace framing policy was bypassed');
    assert.deepEqual(errors, []);
    await page.screenshot({ path: path.join(config.artifacts, 'compact-production-desktop.png') });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: path.join(config.artifacts, 'compact-production-mobile.png') });
    fs.writeFileSync(path.join(config.artifacts, 'compact-production-ui.json'), JSON.stringify({ observations, requests, errors, nativeFrames, viewerResponses, viewerTransport: 'synthetic provider GET boundary; native pixels and first use unqualified' }, null, 2));
    console.log('compact production dashboard native-selection workflow: passed');
  } catch (error) {
    if (page && !page.isClosed()) {
      const frames = [];
      for (const frame of page.frames()) {
        frames.push({ url: frame.url(), text: await frame.locator('body').innerText({ timeout: 1000 }).catch(() => 'unavailable') });
      }
      fs.writeFileSync(path.join(config.artifacts, 'compact-production-ui-failure.json'), JSON.stringify({ message: error.message, requests, errors, nativeFrames, frames, viewerRequests, providerOrigin: config.providerOrigin }, null, 2));
      await page.screenshot({ path: path.join(config.artifacts, 'compact-production-failure.png') }).catch(() => {});
    }
    throw error;
  } finally {
    if (browser) await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
