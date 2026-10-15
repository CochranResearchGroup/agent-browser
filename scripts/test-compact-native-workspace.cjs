const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const crypto = require('node:crypto');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'p237-compact-workspace-'));
  // Use the existing TSX harness and the real workspace CSS below.
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/compact-native-workspace.tsx'), artifacts);
  const server = http.createServer((req, res) => {
    res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : req.url === '/native.css' ? 'text/css' : 'text/html');
    if (req.url === '/native.css') return res.end('body { margin: 0; } button,input { font: inherit; color: inherit; border: 0; background: transparent; } ' + fs.readFileSync(path.join(root, 'packages/dashboard/src/app/globals.css'), 'utf8').split('/* Native workspace keeps navigation compact and the desktop unobstructed. */')[1]);
    res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<link rel="stylesheet" href="/native.css"><div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  let releaseHeldFocus = null;
  try {
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage();
    let ready = true;
    let focusRejection = null;
    let focusHold = null;
    let specificHandoffs = false;
    let frameAllowed = true;
    const requests = [];
    const lateSelections = [];
    await page.route("**/api/service/request", async route => {
      const body = route.request().postDataJSON(); requests.push(body);
      if (body.action === "view_focus") {
        let rejection = focusRejection;
        if (focusHold?.targetId === body.params.targetId) {
          const held = focusHold;
          focusHold = null;
          held.started();
          rejection = await held.reply;
        }
        if (rejection) return route.fulfill({ status: rejection.status, json: {
          success: false,
          error: 'operator_focus_profile_policy_denied: SYNTHETIC-PRIVATE-DETAIL https://provider.invalid/internal',
          failure: rejection.legacyOnly ? undefined : {
            schemaVersion: 'agent-browser.service-failure-recourse.v1',
            code: rejection.code, axis: 'profile_access', phase: 'ingress_validation',
            effectState: 'no_effect', retryDisposition: 'inspect_before_retry',
            recommendedAction: 'inspect_operator_focus_authority', reuseAllowed: false,
          },
        } });
        return route.fulfill({ json: { success: true, data: { focused: true } } });
      }
      const handoffId = specificHandoffs ? `task-link-${body.params.targetId}` : 'task-link';
      return route.fulfill({ json: { success: true, data: { handoffId, handoffUrl: `http://127.0.0.1:${server.address().port}/remote-view/${handoffId}`, operatorVisible: { state: ready ? "ready" : "pending" } } } });
    });
    await page.route("**/remote-view/task-link*", route => route.fulfill({ contentType: "text/html",
      headers: { 'Content-Security-Policy': frameAllowed ? "frame-ancestors 'self'" : "frame-ancestors 'none'" },
      body: '<body id="viewer" data-workspace="true"><p>Synthetic native viewer transport</p></body>' }));
    const errors = []; page.on('pageerror', e => errors.push(e.message));
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).waitFor();
    const actions = page.getByRole('button', { name: 'Tab actions for Alice invoice', exact: true });
    await actions.waitFor({ timeout: 1500 });
    await actions.focus();
    await page.keyboard.press('Enter');
    await page.getByRole('menuitem', { name: 'Properties', exact: true }).press('Enter');
    const properties = page.getByRole('dialog', { name: 'Tab properties', exact: true });
    await properties.waitFor();
    for (const value of ['Synthetic billing', 'Ada worker', 'Review operator', 'Review invoice',
      'Check supplied invoice totals', 'shared-profile', 'desktop-a', 'https://example.test/alice', 'Alice invoice']) {
      await properties.getByText(value, { exact: true }).waitFor();
    }
    await page.screenshot({ path: path.join(artifacts, 'properties-desktop.png') });
    await page.setViewportSize({ width: 390, height: 760 });
    const dialogBounds = await properties.boundingBox();
    await page.screenshot({ path: path.join(artifacts, 'properties-narrow.png') });
    assert.ok(dialogBounds.x >= 0 && dialogBounds.x + dialogBounds.width <= 390, 'Properties must fit a narrow screen');
    assert.ok(dialogBounds.y >= 0 && dialogBounds.y + dialogBounds.height <= 760, 'Properties must fit the visible height');
    await page.setViewportSize({ width: 1280, height: 720 });
    assert.equal(requests.length, 0, 'reading Properties must not focus, navigate or prepare a handoff');
    await page.keyboard.press('Escape');
    await properties.waitFor({ state: 'hidden' });
    await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Tab actions for Alice invoice', null, { timeout: 1500 });
    const bobContext = page.getByRole('button', { name: 'Bob invoice', exact: true });
    await bobContext.focus();
    await page.keyboard.press('Shift+F10');
    await page.getByRole('menuitem', { name: 'Properties', exact: true }).waitFor({ timeout: 1500 });
    await page.getByRole('menuitem', { name: 'Properties', exact: true }).press('Enter');
    await properties.waitFor();
    for (const field of ['Service', 'Agent', 'Human', 'Origin', 'Task', 'Purpose', 'Attribution', 'Desktop']) {
      assert.equal(await properties.locator('dt').filter({ hasText: new RegExp(`^${field}$`) }).locator('..').locator('dd').innerText(), 'Unknown');
    }
    assert.equal((await properties.innerText()).includes('Ada worker'), false, 'peer provenance must not fill missing task values');
    assert.equal(requests.length, 0);
    await page.keyboard.press('Escape');
    await properties.waitFor({ state: 'hidden' });
    await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Bob invoice', null, { timeout: 1500 });
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
    await bobContext.click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'Copy URL', exact: true }).waitFor({ timeout: 1500 });
    await page.getByRole('menuitem', { name: 'Copy URL', exact: true }).press('Enter');
    await page.getByRole('status').filter({ hasText: 'URL copied' }).waitFor();
    assert.equal(await page.evaluate(() => navigator.clipboard.readText()), 'https://example.test/bob');
    assert.equal(requests.length, 0, 'Copy URL must not navigate or prepare a handoff');
    await page.evaluate(() => {
      window.fixtureClipboardWrite = navigator.clipboard.writeText.bind(navigator.clipboard);
      navigator.clipboard.writeText = async () => { throw new DOMException('SYNTHETIC-PRIVATE-CLIPBOARD-DETAIL', 'NotAllowedError'); };
    });
    await bobContext.click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'Copy URL', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'Open Properties to copy it manually' }).waitFor();
    assert.equal((await page.locator('body').innerText()).includes('SYNTHETIC-PRIVATE-CLIPBOARD-DETAIL'), false);
    await page.evaluate(() => { navigator.clipboard.writeText = window.fixtureClipboardWrite; delete window.fixtureClipboardWrite; });
    await bobContext.click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'Copy URL', exact: true }).click();
    await page.getByRole('status').filter({ hasText: 'URL copied' }).waitFor();
    assert.equal(requests.length, 0);

    const rail = page.getByRole('complementary', { name: 'Browsers and tabs' });
    assert.equal(Math.round((await rail.boundingBox()).width), 224);
    const resize = page.getByRole('separator', { name: 'Browser rail width' });
    await resize.focus(); await page.keyboard.press('ArrowRight');
    assert.equal(await resize.getAttribute('aria-valuenow'), '232');
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).click();
    const selected = new URL(page.url());
    assert.equal(selected.searchParams.get('browser'), 'alice-browser');
    assert.equal(selected.searchParams.get('tab'), 'bob');
    assert.equal(selected.searchParams.get('session'), 'bob-session');
    assert.equal(selected.searchParams.get('view'), 'workspace:control');
    await page.getByTitle('Native Remote View desktop', { exact: true }).waitFor();
    await page.frameLocator('iframe[title="Native Remote View desktop"]').getByText('Synthetic native viewer transport', { exact: true }).waitFor();
    await page.getByRole('searchbox').fill('');
    const aliceFrame = page.waitForResponse('**/remote-view/task-link');
    await page.getByRole('button', { name: 'Alice invoice', exact: true }).click();
    await aliceFrame;
    frameAllowed = false;
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'native desktop' }).waitFor({ timeout: 2000 });
    assert.equal(await page.getByRole('link', { name: 'Open Remote View' }).getAttribute('href'), `http://127.0.0.1:${server.address().port}/remote-view/task-link`);
    frameAllowed = true;
    await page.getByRole('button', { name: 'Retry native desktop' }).click();
    await page.getByTitle('Native Remote View desktop', { exact: true }).waitFor();
    assert.ok(requests.some(request => request.action === 'view_focus' && request.params.operatorFocus === true));
    assert.ok(requests.some(request => request.action === 'remote_view_open'));
    await page.reload();
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).waitFor();
    assert.equal(await page.getByRole('button', { name: 'Bob invoice', exact: true }).getAttribute('aria-current'), 'page');
    assert.equal(Math.round((await rail.boundingBox()).width), 232, 'rail width must survive reload');
    assert.equal(await page.getByRole('button', { name: 'Closed invoice', exact: true }).count(), 0);
    await page.getByRole('searchbox', { name: 'Search browsers and tabs' }).fill('Alice');
    assert.equal(await page.getByRole('button', { name: 'Bob invoice', exact: true }).count(), 0);
    await page.getByRole('button', { name: 'Alice invoice', exact: true }).waitFor();
    ready = false;
    await page.getByRole('button', { name: 'Alice invoice', exact: true }).click();
    await page.getByRole('alert').filter({ hasText: 'native desktop' }).waitFor();
    assert.equal(await page.getByTitle('Native Remote View desktop', { exact: true }).count(), 0, 'unready handoff must not become a viewer iframe');
    ready = true;
    await page.getByRole('button', { name: 'Retry native desktop' }).click();
    await page.getByTitle('Native Remote View desktop', { exact: true }).waitFor();
    await page.getByRole('searchbox').fill('');
    for (const rejection of [
      { status: 400, code: 'operator_focus_profile_policy_denied', target: 'Bob invoice', access: true },
      { status: 200, code: 'operator_focus_authority_required', target: 'Alice invoice', access: true },
      { status: 200, code: 'future_focus_policy_error', target: 'Bob invoice', access: false },
      { status: 400, legacyOnly: true, target: 'Alice invoice', access: false },
    ]) {
      focusRejection = rejection;
      const beforeDeniedHandoffs = requests.filter(request => request.action === 'remote_view_open').length;
      const beforeDeniedFocus = requests.filter(request => request.action === 'view_focus').length;
      await page.getByRole('button', { name: rejection.target, exact: true }).click();
      const alert = page.getByRole('region', { name: 'Native desktop', exact: true }).getByRole('alert');
      await alert.filter({ hasText: rejection.access ? 'Check dashboard and profile access' : 'Refresh the task' }).waitFor({ timeout: 1500 });
      assert.equal((await alert.innerText()).includes('Check dashboard and profile access'), rejection.access);
      assert.equal(await page.getByTitle('Native Remote View desktop', { exact: true }).count(), 0);
      assert.equal(requests.filter(request => request.action === 'view_focus').length, beforeDeniedFocus + 1,
        'access rejection must not automatically repeat focus');
      assert.equal(requests.filter(request => request.action === 'remote_view_open').length, beforeDeniedHandoffs,
        'rejected focus must not request a handoff');
      assert.equal((await page.locator('body').innerText()).includes('SYNTHETIC-PRIVATE-DETAIL'), false);
      focusRejection = null;
      await page.getByRole('button', { name: 'Retry native desktop' }).click();
      await page.frameLocator('iframe[title="Native Remote View desktop"]').getByText('Synthetic native viewer transport', { exact: true }).waitFor();
    }
    specificHandoffs = true;
    for (const older of [
      { title: 'Bob invoice', targetId: 'target-bob', sameDesktop: true },
      { title: 'Carol report', targetId: 'target-carol', sameDesktop: false },
    ]) {
      for (const rejects of [false, true]) {
        assert.equal(new URL(page.url()).searchParams.get('tab'), 'alice');
        let announce;
        const started = new Promise(resolve => { announce = resolve; });
        const reply = new Promise(resolve => { releaseHeldFocus = resolve; });
        focusHold = { targetId: older.targetId, started: announce, reply };
        const beforeFocus = requests.filter(request => request.action === 'view_focus').length;
        const beforeHandoffs = requests.filter(request => request.action === 'remote_view_open').length;
        await page.getByRole('button', { name: older.title, exact: true }).click();
        await started;
        await page.getByRole('button', { name: 'Alice invoice', exact: true }).click();
        if (older.sameDesktop) {
          assert.equal(requests.filter(request => request.action === 'view_focus').length, beforeFocus + 1,
            'one desktop must serialize older and newer focus');
          assert.equal(await page.getByTitle('Native Remote View desktop', { exact: true }).count(), 0);
        } else {
          await page.frameLocator('iframe[title="Native Remote View desktop"]').getByText('Synthetic native viewer transport', { exact: true }).waitFor();
          await page.frameLocator('iframe[title="Native Remote View desktop"]').locator('body').evaluate(body => { body.dataset.continuity = 'keep-current-frame'; });
        }
        const oldResponse = page.waitForResponse(response => response.url().endsWith('/api/service/request')
          && response.request().postDataJSON().action === 'view_focus'
          && response.request().postDataJSON().params.targetId === older.targetId);
        releaseHeldFocus(rejects ? { status: 400, code: 'operator_focus_profile_policy_denied' } : null);
        releaseHeldFocus = null;
        await (await oldResponse).finished();
        await page.frameLocator('iframe[title="Native Remote View desktop"]').getByText('Synthetic native viewer transport', { exact: true }).waitFor();
        await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
        assert.ok((await page.getByTitle('Native Remote View desktop', { exact: true }).getAttribute('src')).endsWith('/remote-view/task-link-target-alice'));
        assert.equal(new URL(page.url()).searchParams.get('tab'), 'alice');
        assert.equal(await page.getByRole('region', { name: 'Native desktop', exact: true }).getByRole('alert').count(), 0);
        assert.deepEqual(requests.filter(request => request.action === 'view_focus').slice(beforeFocus).map(request => request.params.targetId),
          [older.targetId, 'target-alice']);
        assert.deepEqual(requests.filter(request => request.action === 'remote_view_open').slice(beforeHandoffs).map(request => request.params.targetId),
          ['target-alice'], 'obsolete focus must not prepare a handoff');
        if (!older.sameDesktop) {
          assert.equal(await page.frameLocator('iframe[title="Native Remote View desktop"]').locator('body').getAttribute('data-continuity'),
            'keep-current-frame', 'late work from another desktop must not remount the current viewer');
        }
        lateSelections.push({
          olderTarget: older.targetId,
          syntheticScope: older.sameDesktop ? 'same-desktop' : 'independent-desktop',
          olderReply: rejects ? 'access-rejection' : 'success',
          focusRequestTargets: requests.filter(request => request.action === 'view_focus').slice(beforeFocus).map(request => request.params.targetId),
          handoffRequestTargets: requests.filter(request => request.action === 'remote_view_open').slice(beforeHandoffs).map(request => request.params.targetId),
          selectedTab: new URL(page.url()).searchParams.get('tab'),
          viewerDocumentPreserved: older.sameDesktop ? null : await page.frameLocator('iframe[title="Native Remote View desktop"]').locator('body').getAttribute('data-continuity') === 'keep-current-frame',
        });
      }
    }
    fs.writeFileSync(path.join(artifacts, 'late-selections.json'), JSON.stringify({
      seam: 'production components in Chromium; synthetic inventory, desktop identities, service and viewer transport',
      bundleSha256: crypto.createHash('sha256').update(fs.readFileSync(path.join(artifacts, 'bundle.js'))).digest('hex'),
      cases: lateSelections,
    }, null, 2));
    specificHandoffs = false;
    await page.getByRole('searchbox').fill('no-such-task');
    await page.getByRole('status').filter({ hasText: 'No matching browsers or tabs' }).waitFor({ timeout: 1500 });
    await page.getByRole('button', { name: 'Collapse browser rail' }).click();
    assert.equal(await page.getByRole('searchbox').count(), 0);
    await page.getByRole('region', { name: 'Native desktop' }).waitFor();
    await page.getByRole('button', { name: 'Show browser rail' }).click();
    await page.getByRole('searchbox').waitFor();
    await page.getByRole('searchbox').fill('');
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).click();
    await page.frameLocator('iframe[title="Native Remote View desktop"]').getByText('Synthetic native viewer transport', { exact: true }).waitFor();
    const beforeMissing = requests.length;
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-remove-selected-tab')));
    const desktop = page.getByRole('region', { name: 'Native desktop', exact: true });
    await desktop.getByRole('alert').filter({ hasText: 'The selected tab cannot be shown' }).waitFor({ timeout: 1500 });
    assert.equal(await page.getByTitle('Native Remote View desktop', { exact: true }).count(), 0, 'missing selection must remove the stale frame');
    assert.equal(new URL(page.url()).searchParams.get('tab'), 'bob', 'missing target must not silently select a peer');
    assert.equal(await page.getByRole('button', { name: 'Bob invoice', exact: true }).count(), 0);
    assert.equal(await page.getByRole('button', { name: 'Alice invoice', exact: true }).count(), 1);
    assert.equal(requests.length, beforeMissing, 'missing inventory must issue no native selection or handoff request');
    await page.screenshot({ path: path.join(artifacts, 'workspace-missing-tab.png') });
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-pause-refresh')));
    await desktop.getByRole('button', { name: 'Refresh browsers', exact: true }).click();
    await desktop.getByRole('status').filter({ hasText: 'Refreshing selected tab' }).waitFor({ timeout: 1500 });
    assert.equal(await desktop.getByRole('alert').count(), 0, 'pending inventory refresh is not a new target failure');
    assert.equal(await desktop.getByRole('button', { name: 'Refreshing browsers', exact: true }).isDisabled(), true);
    assert.equal(requests.length, beforeMissing, 'held refresh must not select a peer or reopen the old frame');
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-finish-refresh')));
    await page.frameLocator('iframe[title="Native Remote View desktop"]').getByText('Synthetic native viewer transport', { exact: true }).waitFor();
    assert.equal(await page.getByRole('button', { name: 'Bob invoice', exact: true }).getAttribute('aria-current'), 'page');
    await page.screenshot({ path: path.join(artifacts, 'workspace-desktop.png') });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: path.join(artifacts, 'workspace-mobile.png') });
    assert.ok((await page.getByRole('region', { name: 'Native desktop' }).boundingBox()).width > 0);
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-remove-selected-tab')));
    await desktop.getByRole('alert').waitFor();
    const mobileRefresh = desktop.getByRole('button', { name: 'Refresh browsers', exact: true });
    const [desktopBounds, refreshBounds] = await Promise.all([desktop.boundingBox(), mobileRefresh.boundingBox()]);
    assert.ok(refreshBounds.x >= desktopBounds.x && refreshBounds.x + refreshBounds.width <= desktopBounds.x + desktopBounds.width,
      'mobile refresh control must fit inside the native workspace');
    await page.screenshot({ path: path.join(artifacts, 'workspace-missing-tab-mobile.png') });
    await mobileRefresh.focus();
    await page.keyboard.press('Enter');
    await page.frameLocator('iframe[title="Native Remote View desktop"]').getByText('Synthetic native viewer transport', { exact: true }).waitFor();
    assert.equal(new URL(page.url()).searchParams.get('tab'), 'bob');
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ success: true, artifacts, seam: 'actual compact component in Chromium; synthetic inventory and viewer boundary' }));
  } finally {
    releaseHeldFocus?.(null);
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
