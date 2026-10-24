#!/usr/bin/env node
/** Mounted public inspection/review workflow. All HTTP APIs are intercepted;
 * real components, Jotai Activity and opt-in browser persistence remain intact. */
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { randomUUID } = require('node:crypto');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
const root = path.resolve(__dirname, '..');
const args = process.argv.slice(2);
function option(name) { const value = args[args.indexOf(name) + 1]; assert(args.includes(name) && value && path.isAbsolute(value), `${name} requires an absolute path`); return value; }
const artifacts = option('--artifact-dir');
const chrome = args.includes('--bundle-only') ? null : option('--chrome');
const packetHash = 'a'.repeat(64);
function identity(pid) {
  try { const stat = fs.readFileSync(`/proc/${pid}/stat`, 'utf8'); return { pid, start: stat.slice(stat.lastIndexOf(')') + 2).split(' ')[19] }; }
  catch (error) { if (['ENOENT', 'ESRCH'].includes(error.code)) return null; throw error; }
}
(async () => {
  fs.mkdirSync(artifacts, { recursive: true });
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/inspection-activity.tsx'), artifacts);
  if (args.includes('--bundle-only')) return;
  const server = http.createServer((req, res) => { res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html'); res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>'); });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const marker = `--p275-inspection-${randomUUID()}`;
  fs.writeFileSync(path.join(artifacts, 'launch.json'), JSON.stringify({ chrome, marker }));
  let browserServer, browser, page;
  const retained = new Map();
  const errors = [], unexpected = [], inspections = [];
  async function retainBrowserProcesses() {
    const cdp = await browser.newBrowserCDPSession(); const { processInfo } = await cdp.send('SystemInfo.getProcessInfo'); await cdp.detach();
    for (const p of processInfo) { const observed = identity(p.id); if (observed) retained.set(p.id, observed); }
    fs.writeFileSync(path.join(artifacts, 'retained-identities.json'), JSON.stringify([...retained.values()]));
  }
  try {
    browserServer = await chromium.launchServer({ executablePath: chrome, headless: true, args: ['--disable-background-networking', marker] });
    const observed = identity(browserServer.process().pid); assert(observed); retained.set(observed.pid, observed);
    fs.writeFileSync(path.join(artifacts, 'browser-root.json'), JSON.stringify(observed));
    browser = await chromium.connect(browserServer.wsEndpoint()); await retainBrowserProcesses();
    page = await browser.newPage(); page.setDefaultTimeout(5000);
    page.on('pageerror', error => errors.push(error.message));
    const intercept = async route => {
      const request = route.request(), url = new URL(request.url());
      if (url.origin === origin && ['/', '/bundle.js'].includes(url.pathname)) return route.continue();
      if (url.origin !== origin) { unexpected.push(url.origin + url.pathname); return route.abort(); }
      if (url.pathname === '/api/sessions') return route.fulfill({ json: [{ session: 'fixture-session-A', port: 9223, engine: 'chrome' }, { session: 'fixture-session-B', port: 9224, engine: 'chrome' }] });
      if (/^\/api\/sessions\/\d+\/tabs$/.test(url.pathname) || url.pathname === '/api/tabs' || url.pathname === '/api/session-tabs') return route.fulfill({ json: [] });
      if (url.pathname === '/api/chat/status') return route.fulfill({ json: { enabled: true } });
      if (url.pathname === '/api/models') return route.fulfill({ json: { data: [] } });
      if (url.pathname === '/api/app-intelligence/inspect-workspace') {
        const input = request.postDataJSON(); inspections.push(input);
        const observation = { version: 'codex-workspace-observation.v1', provider: 'codex-app-server', runId: 'inspection-run-A', threadId: 'synthetic-thread-A', createdAt: new Date().toISOString(), workspaceId: input.packet.workspace.id, summary: 'SYNTHETIC-PRIVATE-OBSERVATION-PROSE', detectedState: 'ready', confidence: 'high', blockers: [{ severity: 'info', summary: 'SYNTHETIC-PRIVATE-BLOCKER-PROSE', evidenceIds: ['workspace.summary'] }], risks: [], suggestedNextInspections: [], unsupportedActions: [] };
        return route.fulfill({ json: { success: true, data: { observation, ledger: { runId: observation.runId, contextPacketHash: packetHash, observationPath: '/private/synthetic-observation.json', eventLogPath: '/private/synthetic-events.json' } } } });
      }
      unexpected.push(url.pathname); return route.abort();
    };
    await page.route('**/*', intercept);
    await page.goto(origin);
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    const feed = page.getByRole('region', { name: 'Activity feed', exact: true });
    const chat = page.getByRole('region', { name: 'Inspection Chat', exact: true });
    await feed.getByRole('button', { name: 'Off', exact: true }).click();
    await feed.getByRole('button', { name: 'On', exact: true }).waitFor();
    await chat.getByRole('button', { name: 'Inspect selected workspace', exact: true }).click();
    await chat.getByText('SYNTHETIC-PRIVATE-OBSERVATION-PROSE', { exact: true }).waitFor();
    assert.equal(inspections.length, 1); assert.deepEqual(inspections[0].packet.selection, { workspaceId: 'browser:browser-A', browserId: 'browser-A', sessionId: 'session-A', tabId: 'tab-A', profileId: 'profile-A', jobId: null });
    assert(!(await feed.innerText()).includes('inspection-run-A'), 'Inspection alone must not record an Activity review');
    fs.writeFileSync(path.join(artifacts, 'before-review.json'), JSON.stringify({ inspectedScope: inspections[0].packet.selection, noAutomaticReview: true }));
    // First expected baseline failure: current inspection has no review control.
    await chat.getByRole('button', { name: 'Record as accepted', exact: true }).click();
    const accepted = () => feed.getByRole('button', { name: /accepted/i });
    await accepted().waitFor(); assert.equal(await accepted().count(), 1, 'One accepted record');
    await accepted().click();
    let text = await feed.innerText();
    for (const expected of ['inspection-run-A', packetHash, 'workspace.summary', 'browser:browser-A', 'session-A', 'tab-A', 'profile-A']) assert(text.includes(expected), `Activity review exposes ${expected}`);
    for (const excluded of ['SYNTHETIC-PRIVATE-OBSERVATION-PROSE', 'SYNTHETIC-PRIVATE-BLOCKER-PROSE', '/private/synthetic-observation.json', '/private/synthetic-events.json', 'Inspect selected workspace']) assert(!text.includes(excluded), `Review metadata excludes ${excluded}`);
    await page.reload();
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    await accepted().waitFor(); assert.equal(await accepted().count(), 1, 'Opt-in persistence restores the review once');
    await accepted().click(); text = await feed.innerText();
    for (const expected of ['inspection-run-A', packetHash, 'workspace.summary', 'browser:browser-A', 'session-A', 'tab-A', 'profile-A']) assert(text.includes(expected), `Reload preserves ${expected}`);
    assert.equal(inspections.length, 1, 'Reload must not repeat inspection or review'); assert.deepEqual(unexpected, []); assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'accept-reload.json'), JSON.stringify({ passed: true, inspectedScope: inspections[0].packet.selection, restoredReviews: 1 }));
    console.log('accept-and-reload: passed');

    // Independent public workflow: corrections append; repeating the current
    // verdict through an ordinary double-click must not append another record.
    await page.close(); page = await browser.newPage(); page.setDefaultTimeout(5000);
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', intercept); await page.goto(origin);
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    const correctionFeed = page.getByRole('region', { name: 'Activity feed', exact: true });
    const correctionChat = page.getByRole('region', { name: 'Inspection Chat', exact: true });
    await correctionFeed.getByRole('button', { name: 'Off', exact: true }).click();
    await correctionChat.getByRole('button', { name: 'Inspect selected workspace', exact: true }).click();
    await correctionChat.getByText('SYNTHETIC-PRIVATE-OBSERVATION-PROSE', { exact: true }).waitFor();
    const acceptReview = correctionChat.getByRole('button', { name: 'Record as accepted', exact: true });
    const rejectReview = correctionChat.getByRole('button', { name: 'Record as rejected', exact: true });
    const acceptedRows = correctionFeed.getByRole('button', { name: /accepted/i });
    const rejectedRows = correctionFeed.getByRole('button', { name: /rejected/i });
    await acceptReview.dblclick();
    assert.equal(await acceptedRows.count(), 1, 'Repeated accepted intent records once');
    assert(await acceptReview.isDisabled(), 'Latest accepted verdict cannot be recorded again');
    await rejectReview.dblclick();
    assert.equal(await acceptedRows.count(), 1, 'Reject preserves original acceptance');
    assert.equal(await rejectedRows.count(), 1, 'Repeated rejected intent records once');
    assert(await rejectReview.isDisabled());
    await acceptReview.dblclick();
    assert.equal(await acceptedRows.count(), 2, 'Acceptance after rejection appends a new correction');
    assert.equal(await rejectedRows.count(), 1, 'Earlier rejection remains');
    const reviewOrder = () => correctionFeed.getByRole('button', { name: /(?:accepted|rejected) inspection review/i }).allTextContents();
    assert.deepEqual((await reviewOrder()).map(value => /accepted/i.test(value) ? 'accepted' : 'rejected'), ['accepted', 'rejected', 'accepted']);
    await page.reload();
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    await acceptedRows.first().waitFor();
    assert.equal(await acceptedRows.count(), 2, 'Both separate accepted decisions survive reload once');
    assert.equal(await rejectedRows.count(), 1, 'Rejected correction survives reload once');
    assert.deepEqual((await reviewOrder()).map(value => /accepted/i.test(value) ? 'accepted' : 'rejected'), ['accepted', 'rejected', 'accepted']);
    assert.equal(inspections.length, 2, 'Corrections and reload never invoke another inspection');
    assert.deepEqual(unexpected, []); assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'corrections.json'), JSON.stringify({ passed: true, accepted: 2, rejected: 1, repeatedVerdictsDeduplicated: true }));
    console.log('corrections-and-repeated-intent: passed');
  } finally {
    if (page && !page.isClosed()) fs.writeFileSync(path.join(artifacts, 'final-dom.txt'), await page.locator('body').innerText().catch(() => 'unavailable'));
    fs.writeFileSync(path.join(artifacts, 'requests.json'), JSON.stringify({ inspections, unexpected, errors }, null, 2));
    try { if (browser) { await retainBrowserProcesses(); await browser.close(); } }
    finally {
      if (browserServer) await browserServer.close();
      await new Promise(resolve => server.close(resolve));
      const remaining = [...retained.values()].filter(p => identity(p.pid)?.start === p.start), markerReferences = [], denied = [];
      for (const pid of fs.readdirSync('/proc').filter(x => /^\d+$/.test(x))) try { if (fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8').includes(marker)) markerReferences.push(pid); } catch (error) { if (!['ENOENT', 'ESRCH'].includes(error.code)) denied.push({ pid, code: error.code }); }
      fs.writeFileSync(path.join(artifacts, 'cleanup.json'), JSON.stringify({ retained: [...retained.values()], remaining, markerReferences, denied }, null, 2));
      assert.deepEqual(remaining, [], 'Retained fixture kernel generations must be absent'); assert.deepEqual(markerReferences, []);
    }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
