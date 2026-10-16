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
  let releaseHeldClose = null;
  let releaseHeldShare = null;
  let releaseHeldDelete = null;
  try {
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    // Keep the supplied epoch-time assertion independent of the host timezone.
    const page = await browser.newPage({ timezoneId: "UTC" });
    let closeDenied = false;
    let shareFailure = false;
    let policyFailure = false;
    let policyWrongTab = false;
    let deleteWrongId = false;
    let deleteHold = null;
    let shareHold = null;
    let closeHold = null;
    let ready = true;
    let focusRejection = null;
    let focusHold = null;
    let specificHandoffs = false;
    let frameAllowed = true;
    const requests = [];
    const lateSelections = [];
    await page.route("**/api/service/request", async route => {
      const body = route.request().postDataJSON(); requests.push(body);
      if (body.action === 'remote_view_open' && body.taskName === 'Share browser task') {
        if (shareFailure) return route.fulfill({ status: 403, json: { success: false, error: 'PRIVATE-SHARE-DIAGNOSTIC https://provider.invalid/internal' } });
        if (shareHold) await shareHold;
      }
      if (body.action === 'service_remote_view_handoff_resolve') {
        if (body.params.handoffOperation === 'delete') {
          if (deleteHold) await deleteHold;
          if (deleteWrongId) return route.fulfill({ json: { success: true, data: { handoffId: 'peer-link', status: 'deleted' } } });
          await page.evaluate(id => {
            const deleted = JSON.parse(sessionStorage.getItem('fixture-deleted-handoffs') || '[]');
            sessionStorage.setItem('fixture-deleted-handoffs', JSON.stringify([...deleted, id]));
          }, body.params.handoffId);
          return route.fulfill({ json: { success: true, data: { handoffId: body.params.handoffId, status: 'deleted' } } });
        }
        if (policyFailure) return route.fulfill({ status: 403, json: { success: false, error: 'PRIVATE-POLICY-DETAIL' } });
        const data = body.params.handoffOperation === 'set_tab_lifetime'
          ? { handoffId: body.params.handoffId, tabId: policyWrongTab ? 'peer-tab' : 'expired-original', status: 'tab_lifetime_updated', tabLifetime: { inactivityMs: body.params.tabInactivityMs || null, pressureInactivityMs: 90000 } }
          : body.params.handoffOperation === 'prepare_human'
            ? { handoffId: body.params.handoffId, tabId: policyWrongTab ? 'peer-tab' : 'expired-original', status: 'pending_human', pendingHuman: { handoffId: body.params.handoffId, requestedAtMs: 1000, expiresAtMs: body.params.pendingHumanRetentionMs === 0 ? null : 1000 + body.params.pendingHumanRetentionMs } }
          : { handoffId: body.params.handoffId, status: 'link_lifetime_updated', expiresAtMs: body.params.ttlMs === 0 ? null : Date.now() + body.params.ttlMs };
        return route.fulfill({ json: { success: true, data } });
      }
      if (body.action === "tab_close") {
        if (closeHold) await closeHold;
        if (closeDenied) return route.fulfill({ status: 403, json: { success: false, error: "SYNTHETIC-PRIVATE-CLOSE-DETAIL", failure: { code: "operator_tab_close_profile_policy_denied", effectState: "no_effect", retryDisposition: "inspect_before_retry", recommendedAction: "inspect_operator_tab_close_authority", reuseAllowed: false, missingPermission: "tab_close_any" } } });
        const tabId = { 'target-alice': 'alice', 'target-bob': 'bob', 'target-carol': 'carol' }[body.params.targetId];
        await page.evaluate(id => {
          const closed = JSON.parse(sessionStorage.getItem('fixture-closed-tabs') || '[]');
          sessionStorage.setItem('fixture-closed-tabs', JSON.stringify([...closed, id]));
        }, tabId);
        return route.fulfill({ json: { success: true, data: { source: 'managed-owner', closed: 0, activeIndex: 1,
          targetId: body.params.targetId, closedTabId: tabId, currentTabId: 'bob', physicalTabClosed: true,
          unrelatedTargetsPreserved: true, browserClosed: false } } });
      }
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
    await page.locator('[data-runtime-environment="development"]').waitFor({timeout:1500});
    assert.equal(await page.locator('[data-runtime-environment="development"]').innerText(), 'Development');
    const dataTab = page.locator('.native-workspace-tab').filter({ hasText: 'PRIVATE-HTML-CANARY' });
    assert.equal(await dataTab.count(), 0, 'HTML data payload must not become a visible tab description');
    await page.getByRole('button', { name: 'HTML document', exact: true }).waitFor({ timeout: 1500 });
    await page.getByRole('button', { name: 'Tab actions for HTML document', exact: true }).waitFor({ timeout: 1500 });
    const drawerTrigger = page.getByRole('button', { name: 'Handoffs', exact: true });
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await drawerTrigger.focus(); await page.keyboard.press('Enter');
    const retainedDrawer = page.getByRole('dialog', { name: 'Handoffs', exact: true });
    await retainedDrawer.waitFor({ timeout: 1500 });
    assert.equal(await retainedDrawer.evaluate(node => getComputedStyle(node).animationName), 'none');
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    const motion = await retainedDrawer.evaluate(node => ({ name: getComputedStyle(node).animationName,
      seconds: parseFloat(getComputedStyle(node).animationDuration) }));
    assert.notEqual(motion.name, 'none', 'the drawer must animate when motion is allowed');
    assert.ok(motion.seconds > 0 && motion.seconds < 0.5, 'drawer motion must be brief');
    const motionRailBounds = await page.getByRole('complementary', { name: 'Browsers and tabs', exact: true, includeHidden: true }).boundingBox();
    const anchoredDrawerBounds = await retainedDrawer.boundingBox();
    assert.ok(anchoredDrawerBounds.x >= motionRailBounds.x + motionRailBounds.width && anchoredDrawerBounds.x <= motionRailBounds.x + motionRailBounds.width + 16,
      'desktop drawer must anchor alongside the browser rail');
    await page.emulateMedia({ reducedMotion: 'reduce' });

    const retainedRow = retainedDrawer.getByRole('row').filter({ hasText: 'Retained review' });
    await retainedRow.waitFor({ timeout: 1500 });
    for (const control of [retainedRow.getByRole('link', { name: 'Open', exact: true }),
      ...['Copy', 'Inspect', 'Policies', 'Delete link'].map(name => retainedRow.getByRole('button', { name, exact: true }))]) {
      assert.equal(await control.locator('svg').count(), 1, 'handoff row actions must use compact accessible SVG controls');
      assert.ok(await control.getAttribute('title'), 'handoff row controls need a tooltip');
    }
    assert.equal(await retainedRow.evaluate(node => getComputedStyle(node).color), await retainedDrawer.evaluate(node => getComputedStyle(node).color), 'retained rows must inherit readable dialog text color');
    await retainedRow.getByText('Audit service', { exact: true }).waitFor();
    await retainedRow.getByText('Casey operator', { exact: true }).waitFor();
    await retainedRow.getByText('Target: Expired', { exact: true }).waitFor();
    await retainedRow.getByText('Link: Perpetual', { exact: true }).waitFor();
    await retainedRow.getByText('Tab: 30 s inactivity', { exact: true }).waitFor();
    assert.match(await retainedRow.getByRole('link', { name: 'Open', exact: true }).getAttribute('href'), /^\/remote-view\/retained-old$/);
    await page.setViewportSize({ width: 390, height: 760 });
    const retainedBounds = await retainedDrawer.boundingBox();
    assert.ok(retainedBounds.x >= 0 && retainedBounds.x + retainedBounds.width <= 390, 'drawer must fit the narrow viewport');
    assert.equal(await retainedDrawer.evaluate(node => node.scrollWidth <= node.clientWidth), true, 'retained rows must not hide actions in horizontal overflow');
    for (const control of [retainedRow.getByRole('link', { name: 'Open', exact: true }), retainedRow.getByRole('button', { name: 'Copy', exact: true }), retainedRow.getByRole('button', { name: 'Inspect', exact: true })]) {
      const box = await control.boundingBox();
      assert.ok(box.x >= retainedBounds.x && box.x + box.width <= retainedBounds.x + retainedBounds.width, 'each retained row action must remain visible');
    }
    await page.screenshot({ path: path.join(artifacts, 'retained-handoffs-narrow.png') });
    await page.setViewportSize({ width: 1280, height: 720 });

    assert.equal(requests.length, 0, 'opening retained records must not prepare or focus a task');
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-pending')));
    await retainedRow.getByText('Awaiting use', { exact: true }).waitFor();
    await retainedRow.getByText('Pending use: Until used', { exact: true }).waitFor();
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-used')));
    await retainedRow.getByText('Used', { exact: true }).waitFor();
    assert.equal(await retainedRow.getByText('Awaiting use', { exact: true }).count(), 0);
    await retainedDrawer.getByRole('button', { name: 'Refresh handoffs', exact: true }).click();
    await retainedRow.getByText('Use not recorded', { exact: true }).waitFor();

    const beforePolicy = requests.length;
    await retainedRow.getByRole('button', { name: 'Policies', exact: true }).click();
    const linkForm = retainedDrawer.getByRole('form', { name: 'Link lifetime', exact: true });
    await linkForm.waitFor({ timeout: 1500 });
    assert.equal(requests.length, beforePolicy, 'opening policy controls must not change retention');
    await linkForm.getByLabel('Link lifetime mode', { exact: true }).selectOption('finite');
    await linkForm.getByLabel('Link lifetime seconds', { exact: true }).fill('30');
    await page.setViewportSize({ width: 390, height: 760 });
    await linkForm.scrollIntoViewIfNeeded();
    assert.equal(await linkForm.evaluate(node => node.scrollWidth <= node.clientWidth), true, 'policy fields must fit a narrow dialog');
    const policyBounds = await linkForm.getByRole('button', { name: 'Save link lifetime', exact: true }).boundingBox();
    assert.ok(policyBounds.x >= 0 && policyBounds.x + policyBounds.width <= 390);
    await page.screenshot({ path: path.join(artifacts, 'link-lifetime-policy-narrow.png') });
    await page.setViewportSize({ width: 1280, height: 720 });

    const policyReply = page.waitForResponse(response => response.url().endsWith('/api/service/request') && response.request().postDataJSON()?.params?.handoffOperation === 'set_link_lifetime');
    await linkForm.getByRole('button', { name: 'Save link lifetime', exact: true }).click();
    await policyReply;
    await linkForm.getByRole('status').filter({ hasText: 'Link lifetime saved' }).waitFor();
    const policyRequest = requests.at(-1);
    assert.equal(policyRequest.action, 'service_remote_view_handoff_resolve');
    assert.deepEqual(policyRequest.params, { handoffId: 'retained-old', handoffOperation: 'set_link_lifetime', ttlMs: 30000 });
    assert.equal(policyRequest.params.tabInactivityMs, undefined);
    assert.equal(policyRequest.params.pendingHumanRetentionMs, undefined);
    await linkForm.getByLabel('Link lifetime mode', { exact: true }).selectOption('perpetual');
    const perpetualReply = page.waitForResponse(response => response.url().endsWith('/api/service/request') && response.request().postDataJSON()?.params?.ttlMs === 0);
    await linkForm.getByRole('button', { name: 'Save link lifetime', exact: true }).click();
    await perpetualReply;
    await linkForm.getByRole('status').filter({ hasText: 'Link lifetime saved' }).waitFor();
    assert.deepEqual(requests.at(-1).params, { handoffId: 'retained-old', handoffOperation: 'set_link_lifetime', ttlMs: 0 });
    await linkForm.getByLabel('Link lifetime mode', { exact: true }).selectOption('finite');
    const invalidCount = requests.length;
    for (const value of ['-1', '1.5', '9999999999999999']) {
      await linkForm.getByLabel('Link lifetime seconds', { exact: true }).fill(value);
      assert.equal(await linkForm.getByRole('button', { name: 'Save link lifetime', exact: true }).isDisabled(), true);
    }
    assert.equal(requests.length, invalidCount);
    await linkForm.getByLabel('Link lifetime seconds', { exact: true }).fill('45');
    policyFailure = true;
    await linkForm.getByRole('button', { name: 'Save link lifetime', exact: true }).click();
    await linkForm.getByRole('alert').filter({ hasText: 'Could not confirm this change' }).waitFor();
    assert.equal((await linkForm.innerText()).includes('PRIVATE-POLICY-DETAIL'), false);
    assert.equal(await linkForm.getByRole('button', { name: 'Save link lifetime', exact: true }).isDisabled(), true);
    const deniedPolicies = requests.length;
    await page.keyboard.press('Enter');
    assert.equal(requests.length, deniedPolicies, 'denied policy update must not automatically repeat');
    policyFailure = false;
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-live')));
    const tabForm = retainedDrawer.getByRole('form', { name: 'Tab inactivity', exact: true });
    await tabForm.waitFor({ timeout: 1500 });
    await tabForm.getByLabel('Tab inactivity mode', { exact: true }).selectOption('finite');
    await tabForm.getByLabel('Tab inactivity seconds', { exact: true }).fill('60');
    const tabReply = page.waitForResponse(response => response.url().endsWith('/api/service/request') && response.request().postDataJSON()?.params?.handoffOperation === 'set_tab_lifetime');
    await tabForm.getByRole('button', { name: 'Save tab inactivity', exact: true }).click();
    await tabReply;
    await tabForm.getByRole('status').filter({ hasText: 'Tab inactivity saved' }).waitFor();
    assert.deepEqual(requests.at(-1).params, { handoffId: 'retained-old', handoffOperation: 'set_tab_lifetime', tabInactivityMs: 60000 });
    assert.equal(requests.at(-1).params.ttlMs, undefined);
    assert.equal(requests.at(-1).params.pendingHumanRetentionMs, undefined);
    assert.equal(requests.at(-1).params.pressureInactivityMs, undefined);
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-live')));
    await tabForm.getByLabel('Tab inactivity mode', { exact: true }).selectOption('perpetual');
    const tabPerpetual = page.waitForResponse(response => response.url().endsWith('/api/service/request') && response.request().postDataJSON()?.params?.tabInactivityMs === 0);
    await tabForm.getByRole('button', { name: 'Save tab inactivity', exact: true }).click();
    await tabPerpetual;
    assert.deepEqual(requests.at(-1).params, { handoffId: 'retained-old', handoffOperation: 'set_tab_lifetime', tabInactivityMs: 0 });
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-live')));
    policyWrongTab = true;
    await tabForm.getByLabel('Tab inactivity mode', { exact: true }).selectOption('finite');
    await tabForm.getByLabel('Tab inactivity seconds', { exact: true }).fill('70');
    await tabForm.getByRole('button', { name: 'Save tab inactivity', exact: true }).click();
    await tabForm.getByRole('alert').filter({ hasText: 'Could not confirm this change' }).waitFor();
    assert.equal(await tabForm.getByRole('status').filter({ hasText: 'Tab inactivity saved' }).count(), 0, 'a peer-tab receipt cannot confirm this task policy');
    policyWrongTab = false;
    const unusedForm = retainedDrawer.getByRole('form', { name: 'Unused retention', exact: true });
    await unusedForm.waitFor({ timeout: 1500 });
    await unusedForm.getByLabel('Unused retention mode', { exact: true }).selectOption('finite');
    await unusedForm.getByLabel('Unused retention seconds', { exact: true }).fill('14400');
    await page.setViewportSize({ width: 390, height: 760 });
    await unusedForm.scrollIntoViewIfNeeded();
    assert.equal(await unusedForm.evaluate(node => node.scrollWidth <= node.clientWidth), true);
    await page.screenshot({ path: path.join(artifacts, 'unused-retention-policy-narrow.png') });
    await page.setViewportSize({ width: 1280, height: 720 });

    const unusedReply = page.waitForResponse(response => response.url().endsWith('/api/service/request') && response.request().postDataJSON()?.params?.handoffOperation === 'prepare_human');
    await unusedForm.getByRole('button', { name: 'Save unused retention', exact: true }).click();
    await unusedReply;
    await unusedForm.getByRole('status').filter({ hasText: 'Unused retention saved' }).waitFor();
    assert.deepEqual(requests.at(-1).params, { handoffId: 'retained-old', handoffOperation: 'prepare_human', pendingHumanRetentionMs: 14400000 });
    assert.equal(requests.at(-1).params.ttlMs, undefined);
    assert.equal(requests.at(-1).params.tabInactivityMs, undefined);
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-live')));
    await unusedForm.getByLabel('Unused retention mode', { exact: true }).selectOption('perpetual');
    const untilUsed = page.waitForResponse(response => response.url().endsWith('/api/service/request') && response.request().postDataJSON()?.params?.pendingHumanRetentionMs === 0);
    await unusedForm.getByRole('button', { name: 'Save unused retention', exact: true }).click();
    await untilUsed;
    await unusedForm.getByRole('status').filter({ hasText: 'Unused retention saved' }).waitFor();
    assert.deepEqual(requests.at(-1).params, { handoffId: 'retained-old', handoffOperation: 'prepare_human', pendingHumanRetentionMs: 0 });
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-rearmed')));
    await retainedRow.getByText('Awaiting use', { exact: true }).waitFor({ timeout: 1500 });
    assert.equal(await retainedRow.getByText('Used', { exact: true }).count(), 0, 'historical first use must not mask a newly requested unused-retention window');


    assert.equal(new URL(page.url()).searchParams.get('tab'), null);
    requests.length = 0;
    await retainedRow.getByRole('button', { name: 'Policies', exact: true }).click();
    await retainedRow.getByRole('button', { name: 'Inspect', exact: true }).click();
    await page.setViewportSize({ width: 1280, height: 720 });
    const detailsWidth = (await retainedDrawer.locator('dl').boundingBox()).width;
    const tableWidth = (await retainedDrawer.getByRole('table', { name: 'Retained handoffs', exact: true }).boundingBox()).width;
    assert.ok(detailsWidth >= tableWidth * 0.8, 'inspection must use the row width rather than stretch only the actions column');

    await retainedDrawer.getByText('archived-profile', { exact: true }).waitFor();
    await retainedDrawer.getByText('old-task', { exact: true }).waitFor();
    await retainedDrawer.getByText(`http://127.0.0.1:${server.address().port}/remote-view/retained-old`, { exact: true }).waitFor();
    await page.screenshot({ path: path.join(artifacts, 'retained-handoff-inspection-wide.png') });
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
    await retainedRow.getByRole('button', { name: 'Copy', exact: true }).click();
    assert.equal(await page.evaluate(() => navigator.clipboard.readText()), `http://127.0.0.1:${server.address().port}/remote-view/retained-old`);
    await page.evaluate(() => {
      window.recordClipboardWrite = navigator.clipboard.writeText.bind(navigator.clipboard);
      navigator.clipboard.writeText = async () => { throw new DOMException('PRIVATE-COPY-CANARY', 'NotAllowedError'); };
    });
    await retainedRow.getByRole('button', { name: 'Copy', exact: true }).click();
    await retainedDrawer.getByRole('alert').filter({ hasText: 'Could not copy this link.' }).waitFor();
    assert.equal((await retainedDrawer.innerText()).includes('PRIVATE-COPY-CANARY'), false);
    await page.evaluate(() => { navigator.clipboard.writeText = window.recordClipboardWrite; delete window.recordClipboardWrite; });
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-route')));
    await retainedRow.getByRole('alert').filter({ hasText: 'Link unavailable' }).waitFor();
    assert.equal(await retainedRow.getByRole('link', { name: 'Open', exact: true }).count(), 0);
    assert.equal(await retainedRow.getByRole('button', { name: 'Copy', exact: true }).isDisabled(), true);
    assert.equal((await retainedDrawer.innerText()).includes('PRIVATE-ROUTE-CANARY'), false);
    await retainedDrawer.getByRole('button', { name: 'Refresh handoffs', exact: true }).click();
    await retainedRow.getByRole('link', { name: 'Open', exact: true }).waitFor();
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-pause-refresh')));
    await retainedDrawer.getByRole('button', { name: 'Refresh handoffs', exact: true }).click();
    await retainedDrawer.getByRole('status').filter({ hasText: 'Refreshing handoffs…' }).waitFor();
    assert.equal(await retainedDrawer.getByRole('button', { name: 'Refresh handoffs', exact: true }).isDisabled(), true);
    await retainedRow.waitFor();
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-finish-refresh')));
    await retainedDrawer.getByRole('status').filter({ hasText: 'Refreshing handoffs…' }).waitFor({ state: 'hidden' });
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-fail')));
    await retainedDrawer.getByRole('button', { name: 'Refresh handoffs', exact: true }).click();
    await retainedDrawer.getByRole('alert').filter({ hasText: 'Could not refresh handoffs' }).waitFor();
    await retainedRow.waitFor();
    assert.equal((await retainedDrawer.innerText()).includes('SYNTHETIC-PRIVATE-INVENTORY-DETAIL'), false);
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-repair')));
    await retainedDrawer.getByRole('button', { name: 'Refresh handoffs', exact: true }).click();
    await retainedDrawer.getByRole('alert').filter({ hasText: 'Could not refresh handoffs' }).waitFor({ state: 'hidden' });
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-empty')));
    await retainedDrawer.getByRole('status').filter({ hasText: 'No retained handoffs.' }).waitFor();
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-handoffs-unavailable')));
    await retainedDrawer.getByRole('status').filter({ hasText: 'Handoff inventory is unavailable.' }).waitFor();
    assert.equal(await retainedDrawer.getByRole('status').filter({ hasText: 'No retained handoffs.' }).count(), 0);
    await retainedDrawer.getByRole('button', { name: 'Refresh handoffs', exact: true }).click();
    await retainedRow.waitFor();
    assert.equal(requests.length, 0, 'record inspection, copy and inventory refresh must not issue handoff or browser effects');

    await page.keyboard.press('Escape');
    await retainedDrawer.waitFor({ state: 'hidden' });
    await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Handoffs');
    const actions = page.getByRole('button', { name: 'Tab actions for Alice invoice', exact: true });
    await actions.waitFor({ timeout: 1500 });
    await actions.focus();
    await page.keyboard.press('Enter');
    await page.getByRole('menuitem', { name: 'Properties', exact: true }).press('Enter');
    const properties = page.getByRole('dialog', { name: 'Tab properties', exact: true });
    await properties.waitFor();
    for (const value of ['Synthetic billing', 'Ada worker', 'Review operator', 'Review invoice',
      'Check supplied invoice totals', 'job-review', 'request-review', 'shared-profile', 'desktop-a', 'https://example.test/alice', 'Alice invoice']) {
      await properties.getByText(value, { exact: true }).waitFor();
    }
    await properties.getByText(/1970/).waitFor();
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

    await actions.click();
    await page.getByRole('menuitem', { name: 'Close Tab', exact: true }).waitFor({ timeout: 1500 });
    await page.getByRole('menuitem', { name: 'Close Tab', exact: true }).press('Enter');
    const closeDialog = page.getByRole('dialog', { name: 'Close tab?', exact: true });
    await closeDialog.waitFor();
    await closeDialog.getByText('Alice invoice', { exact: true }).waitFor();
    assert.equal(requests.length, 0, 'opening close confirmation must not send an effect');
    await closeDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    await closeDialog.waitFor({ state: 'hidden' });
    await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Tab actions for Alice invoice');
    assert.equal(requests.length, 0, 'cancelling close must preserve every task');
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
    assert.equal(await page.getByRole('dialog', { name: 'Handoffs', exact: true }).count(), 0, 'handoffs must start hidden');
    shareFailure = true;
    await actions.click();
    await page.getByRole('menuitem', { name: 'Share', exact: true }).click();
    const failedShare = page.getByRole('dialog', { name: 'Handoffs', exact: true });
    await failedShare.getByRole('alert').filter({ hasText: 'Could not prepare this link.' }).waitFor();
    assert.equal((await failedShare.innerText()).includes('PRIVATE-SHARE-DIAGNOSTIC'), false);
    assert.equal(await failedShare.getByLabel('Shared handoff URL', { exact: true }).count(), 0);
    const failedShareCount = requests.filter(request => request.taskName === 'Share browser task').length;
    await page.keyboard.press('Enter');
    assert.equal(requests.filter(request => request.taskName === 'Share browser task').length, failedShareCount, 'failed Share must not automatically repeat');
    await page.keyboard.press('Escape');
    await failedShare.waitFor({ state: 'hidden' });
    shareFailure = false;
    shareHold = new Promise(resolve => { releaseHeldShare = resolve; });
    const selectionBeforeShare = page.url();
    const focusBeforeShare = requests.filter(request => request.action === 'view_focus').length;
    await actions.click();
    await page.getByRole('menuitem', { name: 'Share', exact: true }).click();
    const handoffs = page.getByRole('dialog', { name: 'Handoffs', exact: true });
    await handoffs.getByRole('status').filter({ hasText: 'Preparing link…' }).waitFor();
    assert.equal(await handoffs.getByLabel('Shared handoff URL', { exact: true }).count(), 0);
    releaseHeldShare(); shareHold = null;
    await handoffs.getByLabel('Shared handoff URL', { exact: true }).waitFor();
    assert.match(await handoffs.getByLabel('Shared handoff URL', { exact: true }).inputValue(), /\/remote-view\/task-link/);
    assert.equal(page.url(), selectionBeforeShare, 'sharing must not replace selected Bob');
    assert.equal(requests.filter(request => request.action === 'view_focus').length, focusBeforeShare);
    const sharedRequest = requests.filter(request => request.action === 'remote_view_open').at(-1);
    assert.equal(sharedRequest.browserId, 'alice-browser');
    assert.equal(sharedRequest.sessionName, 'alice-session');
    assert.equal(sharedRequest.params.targetId, 'target-alice');
    assert.equal(sharedRequest.params.displayAllocationId, 'desktop-a');
    assert.equal(sharedRequest.params.url, undefined, 'sharing an existing task must not navigate');
    await page.keyboard.press('Escape');
    await handoffs.waitFor({ state: 'hidden' });
    await page.waitForFunction(() => document.activeElement?.getAttribute('aria-label') === 'Tab actions for Alice invoice');
    closeDenied = true;
    await actions.click();
    await page.getByRole('menuitem', { name: 'Close Tab', exact: true }).click();
    await closeDialog.getByRole('button', { name: 'Close tab', exact: true }).click();
    await closeDialog.getByText('This profile does not allow you to close this tab. No tab was closed. Ask the profile owner to review close permission.', { exact: true }).waitFor();
    assert.equal((await closeDialog.innerText()).includes('SYNTHETIC-PRIVATE-CLOSE-DETAIL'), false);
    assert.equal(await closeDialog.getByRole('button', { name: 'Close tab', exact: true }).isDisabled(), true);
    assert.equal(await page.locator('button.native-workspace-tab[aria-label="Alice invoice"]').count(), 1);
    const deniedCount = requests.filter(request => request.action === 'tab_close').length;
    await page.keyboard.press('Enter');
    assert.equal(requests.filter(request => request.action === 'tab_close').length, deniedCount, 'denied close must not automatically replay');
    await closeDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    closeDenied = false;
    const beforeClose = requests.filter(request => request.action === 'tab_close').length;
    const selectedBeforeClose = page.url();
    await actions.click();
    await page.getByRole('menuitem', { name: 'Close Tab', exact: true }).click();
    await closeDialog.waitFor();
    closeHold = new Promise(resolve => { releaseHeldClose = resolve; });
    const closeReply = page.waitForResponse(response => response.url().endsWith('/api/service/request') &&
      response.request().postDataJSON()?.action === 'tab_close', { timeout: 1500 });
    await closeDialog.getByRole('button', { name: 'Close tab', exact: true }).click();
    await closeDialog.getByText('Closing tab…', { exact: true }).waitFor();
    assert.equal(await closeDialog.getByRole('button', { name: 'Close tab', exact: true }).isDisabled(), true);
    assert.equal(await closeDialog.getByRole('button', { name: 'Cancel', exact: true }).isDisabled(), true);
    await page.keyboard.press('Escape');
    await closeDialog.waitFor();
    assert.equal(requests.filter(request => request.action === 'tab_close').length, beforeClose + 1);
    releaseHeldClose(); closeHold = null;
    await closeReply;
    await page.getByRole('button', { name: 'Alice invoice', exact: true }).waitFor({ state: 'hidden' });
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).waitFor();
    assert.equal(page.url(), selectedBeforeClose, 'closing a background task must preserve the selected peer');
    const closes = requests.filter(request => request.action === 'tab_close');
    assert.equal(closes.length, beforeClose + 1);
    assert.equal(closes.at(-1).browserId, 'alice-browser');
    assert.equal(closes.at(-1).sessionName, 'alice-session');
    assert.deepEqual(closes.at(-1).params, { targetId: 'target-alice', operatorClose: true });
    await page.getByRole('button', { name: 'Refresh browsers', exact: true }).click();
    assert.equal(await page.getByRole('button', { name: 'Alice invoice', exact: true }).count(), 0);
    await page.reload();
    await page.getByRole('button', { name: 'Bob invoice', exact: true }).waitFor();
    assert.equal(await page.getByRole('button', { name: 'Alice invoice', exact: true }).count(), 0);
    assert.equal(new URL(page.url()).searchParams.get('tab'), 'bob');
    const requestsBeforeSelectedClose = requests.length;
    await page.getByRole('button', { name: 'Tab actions for Bob invoice', exact: true }).click();
    await page.getByRole('menuitem', { name: 'Close Tab', exact: true }).click();
    await page.getByRole('dialog', { name: 'Close tab?', exact: true }).getByRole('button', { name: 'Close tab', exact: true }).click();
    await page.getByRole('status').filter({ hasText: 'The selected tab was closed. Select another tab to continue.' }).waitFor();
    assert.equal(await page.locator('iframe[title="Native Remote View desktop"]').count(), 0, 'selected closed task must release its stale native frame');
    assert.equal(new URL(page.url()).searchParams.get('tab'), 'bob', 'close must not silently select a peer');
    assert.equal(requests.slice(requestsBeforeSelectedClose).filter(request => request.action !== 'tab_close').length, 0, 'close must not issue a new peer handoff or focus');
    await page.getByRole('button', { name: 'Handoffs', exact: true }).press('Enter');
    const deleteDrawer = page.getByRole('dialog', { name: 'Handoffs', exact: true });
    const deletingRow = deleteDrawer.getByRole('row').filter({ hasText: 'Retained review' });
    const deleteCount = requests.length;
    await deletingRow.getByRole('button', { name: 'Delete link', exact: true }).click();
    const deleteDialog = page.getByRole('dialog', { name: 'Delete link?', exact: true });
    await deleteDialog.getByText('Retained review', { exact: true }).waitFor();
    await page.setViewportSize({ width: 390, height: 760 });
    const deleteBounds = await deleteDialog.boundingBox();
    assert.ok(deleteBounds.x >= 0 && deleteBounds.x + deleteBounds.width <= 390);
    await page.screenshot({ path: path.join(artifacts, 'delete-link-confirmation-narrow.png') });
    await page.setViewportSize({ width: 1280, height: 720 });

    assert.equal(requests.length, deleteCount, 'opening deletion confirmation must have no effect');
    await deleteDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
    assert.equal(requests.length, deleteCount, 'cancel must not revoke a link');
    await deletingRow.waitFor();
    deleteWrongId = true;
    await deletingRow.getByRole('button', { name: 'Delete link', exact: true }).click();
    await deleteDialog.getByRole('button', { name: 'Delete link', exact: true }).click();
    await deleteDialog.getByRole('alert').filter({ hasText: 'Could not confirm deletion' }).waitFor();
    assert.equal(await deleteDialog.getByRole('button', { name: 'Delete link', exact: true }).isDisabled(), true);
    assert.equal((await deleteDialog.innerText()).includes('peer-link'), false);
    await page.keyboard.press('Escape');
    await deletingRow.waitFor();
    assert.equal(await deleteDrawer.getByRole('status').filter({ hasText: 'Link deleted.' }).count(), 0);
    assert.equal(requests.length, deleteCount + 1, 'unconfirmed deletion must not automatically repeat');
    deleteWrongId = false;
    deleteHold = new Promise(resolve => { releaseHeldDelete = resolve; });
    await deletingRow.getByRole('button', { name: 'Delete link', exact: true }).click();
    const deleteReply = page.waitForResponse(response => response.url().endsWith('/api/service/request') && response.request().postDataJSON()?.params?.handoffOperation === 'delete');
    await deleteDialog.getByRole('button', { name: 'Delete link', exact: true }).click();
    await deleteDialog.getByRole('status').filter({ hasText: 'Deleting link…' }).waitFor();
    assert.equal(await deleteDialog.getByRole('button', { name: 'Cancel', exact: true }).isDisabled(), true);
    assert.equal(await deleteDialog.getByRole('button', { name: 'Delete link', exact: true }).isDisabled(), true);
    await page.keyboard.press('Escape');
    await deleteDialog.waitFor();
    assert.equal(requests.length, deleteCount + 2);
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-pause-refresh')));
    releaseHeldDelete(); deleteHold = null;
    await deleteReply;
    await deletingRow.waitFor({ state: 'hidden' });
    assert.equal(requests.length, deleteCount + 2);
    assert.equal(requests.at(-1).action, 'service_remote_view_handoff_resolve');
    assert.deepEqual(requests.at(-1).params, { handoffId: 'retained-old', handoffOperation: 'delete' });
    await deleteDrawer.getByRole('status').filter({ hasText: 'Link deleted.' }).waitFor();
    await page.waitForFunction(() => {
      const dialog = [...document.querySelectorAll('[role="dialog"]')].find(node => node.textContent.includes('Handoffs'));
      return dialog?.contains(document.activeElement);
    }, null, { timeout: 1500 });
    await page.evaluate(() => window.dispatchEvent(new Event('fixture-finish-refresh')));

    await deleteDrawer.getByRole('button', { name: 'Refresh handoffs', exact: true }).click();
    assert.equal(await deletingRow.count(), 0);
    await page.reload();
    await page.getByRole('button', { name: 'Handoffs', exact: true }).press('Enter');
    await page.getByRole('dialog', { name: 'Handoffs', exact: true }).getByRole('status').filter({ hasText: 'No retained handoffs.' }).waitFor();
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ success: true, artifacts, seam: 'actual compact component in Chromium; synthetic inventory and viewer boundary' }));
  } finally {
    releaseHeldFocus?.(null);
    releaseHeldClose?.();
    releaseHeldShare?.();
    releaseHeldDelete?.();
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
