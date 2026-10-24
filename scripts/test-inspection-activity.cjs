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
  let holdNextInspection = false, heldInspection = null;
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
        const scope = input.packet.selection.browserId === 'browser-B' ? 'B' : 'A';
        const observation = { version: 'codex-workspace-observation.v1', provider: 'codex-app-server', runId: `inspection-run-${scope}`, threadId: `synthetic-thread-${scope}`, createdAt: new Date(Date.now() - (holdNextInspection ? 86_400_000 : 0)).toISOString(), workspaceId: input.packet.workspace.id, summary: 'SYNTHETIC-PRIVATE-OBSERVATION-PROSE', detectedState: 'ready', confidence: 'high', blockers: [{ severity: 'info', summary: 'SYNTHETIC-PRIVATE-BLOCKER-PROSE', evidenceIds: ['workspace.summary'] }], risks: [], suggestedNextInspections: [], unsupportedActions: [] };
        const reply = { success: true, data: { observation, ledger: { runId: observation.runId, contextPacketHash: scope === 'A' ? packetHash : 'b'.repeat(64), observationPath: '/private/synthetic-observation.json', eventLogPath: '/private/synthetic-events.json' } } };
        if (holdNextInspection) { holdNextInspection = false; heldInspection = { route, input, reply }; return; }
        return route.fulfill({ json: reply });
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

    // Recording must not silently enable the existing opt-in persistence switch.
    await page.close(); page = await browser.newPage(); page.setDefaultTimeout(5000);
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', intercept); await page.goto(origin);
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    const volatileFeed = page.getByRole('region', { name: 'Activity feed', exact: true });
    const volatileChat = page.getByRole('region', { name: 'Inspection Chat', exact: true });
    await volatileFeed.getByRole('button', { name: 'Off', exact: true }).waitFor();
    await volatileChat.getByRole('button', { name: 'Inspect selected workspace', exact: true }).click();
    await volatileChat.getByText('SYNTHETIC-PRIVATE-OBSERVATION-PROSE', { exact: true }).waitFor();
    await volatileChat.getByRole('button', { name: 'Record as accepted', exact: true }).click();
    await volatileFeed.getByRole('button', { name: /accepted/i }).waitFor();
    await volatileChat.getByText('Recorded in Activity for this session. Persistence is off.', { exact: true }).waitFor();
    assert.equal(await volatileFeed.getByRole('button', { name: 'Off', exact: true }).count(), 1);
    await page.reload();
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    await volatileFeed.getByText('Waiting for events...', { exact: true }).waitFor();
    assert.equal(await volatileFeed.getByRole('button', { name: /accepted/i }).count(), 0, 'Persistence-off review is not restored');
    assert.equal(await volatileFeed.getByRole('button', { name: 'Off', exact: true }).count(), 1);
    assert.equal(inspections.length, 3); assert.deepEqual(unexpected, []); assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'persist-off.json'), JSON.stringify({ passed: true, recordedInSession: true, restoredReviews: 0, persistRemainsOff: true }));
    console.log('persist-off-remains-off: passed');

    // Fail the real browser storage boundary after opt-in, without replacing
    // Activity's store or persistence collaborators.
    await page.close(); page = await browser.newPage(); page.setDefaultTimeout(5000);
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', intercept); await page.goto(origin);
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    const failedFeed = page.getByRole('region', { name: 'Activity feed', exact: true });
    const failedChat = page.getByRole('region', { name: 'Inspection Chat', exact: true });
    await failedFeed.getByRole('button', { name: 'Off', exact: true }).click();
    await page.evaluate(() => { Storage.prototype.setItem = function () { throw new DOMException('Synthetic storage write failure', 'QuotaExceededError'); }; });
    await failedChat.getByRole('button', { name: 'Inspect selected workspace', exact: true }).click();
    await failedChat.getByText('SYNTHETIC-PRIVATE-OBSERVATION-PROSE', { exact: true }).waitFor();
    await failedChat.getByRole('button', { name: 'Record as accepted', exact: true }).click();
    await failedFeed.getByRole('button', { name: /accepted/i }).waitFor();
    await failedChat.getByText('Recorded in Activity, but browser storage is unavailable. It may be lost on reload.', { exact: true }).waitFor();
    assert(!(await failedChat.innerText()).includes('Recorded in Activity and saved for reload.'));
    await page.reload();
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    await failedFeed.getByText('Waiting for events...', { exact: true }).waitFor();
    assert.equal(await failedFeed.getByRole('button', { name: /accepted/i }).count(), 0, 'Failed storage does not restore the memory-only review');
    assert.equal(inspections.length, 4); assert.deepEqual(unexpected, []); assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'storage-failure.json'), JSON.stringify({ passed: true, memoryRecord: true, durabilityClaim: false, restoredReviews: 0 }));
    console.log('storage-failure-is-truthful: passed');

    // A response may arrive after the operator has selected a different task.
    // Return to exact A to record its historical judgment, then isolate B/Clear.
    await page.close(); page = await browser.newPage(); page.setDefaultTimeout(5000);
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', intercept); await page.goto(origin);
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    const scopedFeed = page.getByRole('region', { name: 'Activity feed', exact: true });
    const scopedChat = page.getByRole('region', { name: 'Inspection Chat', exact: true });
    await scopedFeed.getByRole('button', { name: 'Off', exact: true }).click();
    holdNextInspection = true;
    await scopedChat.getByRole('button', { name: 'Inspect selected workspace', exact: true }).click();
    const deadline = Date.now() + 5000;
    while (!heldInspection && Date.now() < deadline) await page.waitForTimeout(20);
    assert(heldInspection, 'A inspection is held at the external HTTP boundary');
    await page.getByRole('button', { name: 'Select workspace B', exact: true }).click();
    await page.getByLabel('Active session').getByText('fixture-session-B', { exact: true }).waitFor();
    const originalObservedAt = heldInspection.reply.data.observation.createdAt;
    await heldInspection.route.fulfill({ json: heldInspection.reply });
    await scopedChat.getByText('SYNTHETIC-PRIVATE-OBSERVATION-PROSE', { exact: true }).waitFor();
    const aAccept = scopedChat.getByRole('button', { name: 'Record as accepted', exact: true }).first();
    assert(await aAccept.isDisabled(), 'Late A response cannot be reviewed against B');
    assert(!(await scopedFeed.innerText()).includes('inspection-run-A'), 'Late A never auto-attaches to B Activity');
    await page.getByRole('button', { name: 'Select workspace A', exact: true }).click();
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    await aAccept.click();
    const aReview = scopedFeed.getByRole('button', { name: /accepted/i });
    await aReview.waitFor(); await aReview.click();
    let scopedText = await scopedFeed.innerText();
    for (const expected of ['inspection-run-A', 'browser:browser-A', 'session-A', 'tab-A', 'profile-A', originalObservedAt]) assert(scopedText.includes(expected), `Historical A retains ${expected}`);
    assert(!(scopedText.includes('browser:browser-B')), 'A review has no B scope');
    await page.getByRole('button', { name: 'Select workspace B', exact: true }).click();
    await page.getByLabel('Active session').getByText('fixture-session-B', { exact: true }).waitFor();
    await scopedFeed.getByText('Waiting for events...', { exact: true }).waitFor();
    await scopedChat.getByRole('button', { name: 'Inspect selected workspace', exact: true }).click();
    await scopedChat.getByText('packet bbbbbbbbbbbb', { exact: true }).waitFor();
    await scopedChat.getByRole('button', { name: 'Record as accepted', exact: true }).last().click();
    const bReview = scopedFeed.getByRole('button', { name: /accepted/i });
    await bReview.waitFor(); assert.equal(await bReview.count(), 1, 'Only B review appears in B session');
    await bReview.click(); scopedText = await scopedFeed.innerText();
    for (const expected of ['inspection-run-B', 'browser:browser-B', 'session-B', 'tab-B', 'profile-B']) assert(scopedText.includes(expected), `B retains ${expected}`);
    assert(!scopedText.includes('inspection-run-A'), 'A review stays isolated');
    await scopedFeed.getByRole('button', { name: 'Clear activity', exact: true }).click();
    await scopedFeed.getByText('Waiting for events...', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Select workspace A', exact: true }).click();
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    await scopedFeed.getByRole('button', { name: /accepted/i }).waitFor();
    assert.equal(await scopedFeed.getByRole('button', { name: /accepted/i }).count(), 1, 'Clear B preserves A');
    await page.reload();
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    await scopedFeed.getByRole('button', { name: /accepted/i }).waitFor();
    assert.equal(await scopedFeed.getByRole('button', { name: /accepted/i }).count(), 1, 'A survives reload after clearing B');
    await page.getByRole('button', { name: 'Select workspace B', exact: true }).click();
    await page.getByLabel('Active session').getByText('fixture-session-B', { exact: true }).waitFor();
    await scopedFeed.getByText('Waiting for events...', { exact: true }).waitFor();
    assert.equal(inspections.length, 6); assert.deepEqual(unexpected, []); assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'late-scope-and-clear.json'), JSON.stringify({ passed: true, historicalAObservedAt: originalObservedAt, lateARecordedOnlyOnA: true, bClearPreservesA: true }));
    console.log('late-A-session-isolation-and-clear: passed');

    // Exercise the real WebSocket-to-streamEvents owner. A later stream update
    // replaces its own event array without erasing the separate local review.
    await page.close(); page = await browser.newPage(); page.setDefaultTimeout(5000);
    page.on('pageerror', error => errors.push(error.message));
    let stream = null;
    await page.routeWebSocket('**/*', socket => {
      const address = new URL(socket.url());
      if (address.protocol !== 'ws:' || address.hostname !== 'localhost' || address.port !== '9223' || address.pathname !== '/') { unexpected.push(socket.url()); socket.close(); return; }
      stream = socket;
    });
    await page.route('**/*', intercept); await page.goto(origin);
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    const streamFeed = page.getByRole('region', { name: 'Activity feed', exact: true });
    const streamChat = page.getByRole('region', { name: 'Inspection Chat', exact: true });
    await streamFeed.getByRole('button', { name: 'Off', exact: true }).click();
    await page.getByRole('button', { name: 'Connect synthetic stream', exact: true }).click();
    const streamDeadline = Date.now() + 5000;
    while (!stream && Date.now() < streamDeadline) await page.waitForTimeout(20);
    assert(stream, 'Synthetic WebSocket is intercepted before connecting');
    const streamResult = suffix => JSON.stringify({ type: 'result', id: `stream-${suffix}`, action: `synthetic.stream.${suffix}`, success: true, data: { fixture: true }, duration_ms: 1, timestamp: Date.now() });
    stream.send(streamResult('before'));
    await streamFeed.getByRole('button', { name: /synthetic.stream.before/ }).waitFor();
    await streamChat.getByRole('button', { name: 'Inspect selected workspace', exact: true }).click();
    await streamChat.getByText('SYNTHETIC-PRIVATE-OBSERVATION-PROSE', { exact: true }).waitFor();
    await streamChat.getByRole('button', { name: 'Record as accepted', exact: true }).click();
    await streamFeed.getByRole('button', { name: /accepted/i }).waitFor();
    stream.send(streamResult('after'));
    await streamFeed.getByRole('button', { name: /synthetic.stream.after/ }).waitFor();
    assert.equal(await streamFeed.getByRole('button', { name: /accepted/i }).count(), 1, 'Stream replacement preserves one local review');
    await streamFeed.getByRole('button', { name: /accepted/i }).click();
    let streamText = await streamFeed.innerText();
    for (const expected of ['inspection-run-A', packetHash, 'browser:browser-A', 'session-A', 'tab-A', 'profile-A']) assert(streamText.includes(expected), `Stream update preserves ${expected}`);
    await page.reload();
    await page.getByLabel('Active session').getByText('fixture-session-A', { exact: true }).waitFor();
    await streamFeed.getByRole('button', { name: /accepted/i }).waitFor();
    assert.equal(await streamFeed.getByRole('button', { name: /accepted/i }).count(), 1, 'Review restores once after stream activity');
    assert.equal(inspections.length, 7); assert.deepEqual(unexpected, []); assert.deepEqual(errors, []);
    fs.writeFileSync(path.join(artifacts, 'stream-replacement.json'), JSON.stringify({ passed: true, actualStreamHook: true, interceptedWebSockets: 1, preservedReviews: 1 }));
    console.log('websocket-stream-replacement-preserves-review: passed');
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
