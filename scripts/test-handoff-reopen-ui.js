#!/usr/bin/env node
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { chromium } from 'playwright';
const source = readFileSync(new URL('../cli/src/native/stream/handoff-reopen.html', import.meta.url), 'utf8');
const server = await chromium.launchServer({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
const pid = server.process().pid;
const browser = await chromium.connect(server.wsEndpoint());
const observed = new Map();
function observeOwnedProcesses() {
  const rows = new Map();
  for (const name of readdirSync('/proc').filter(name => /^\d+$/.test(name))) {
    try {
      const stat = readFileSync(`/proc/${name}/stat`, 'utf8');
      const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
      rows.set(Number(name), { parent: Number(fields[1]), start: fields[19] });
    } catch { /* Process exited during the read-only census. */ }
  }
  const owned = new Set([pid]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const [child, row] of rows) {
      if (owned.has(row.parent) && !owned.has(child)) { owned.add(child); changed = true; }
    }
  }
  for (const child of owned) if (rows.has(child)) observed.set(child, rows.get(child).start);
}
try {
  for (const scenario of ['preserved', 'missing', 'unqualified']) {
    const missing = scenario === 'missing';
    const unqualified = scenario === 'unqualified';
    const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
    const page = await context.newPage();
    let visits = 0;
    const requests = [];
    let dialogs = 0;
    page.on('dialog', async dialog => { dialogs++; await dialog.dismiss(); });
    await page.route('https://fixture.example/remote-view/retained-link', route => {
      visits++;
      return route.fulfill({ contentType: 'text/html', body: source });
    });
    await page.route('https://fixture.example/api/service/request', async route => {
      assert.equal(route.request().method(), 'POST');
      const request = route.request().postDataJSON();
      requests.push(request);
      assert.equal(request.action, 'service_remote_view_handoff_resolve');
      assert.equal(request.params.handoffId, 'retained-link');
      assert.equal(request.params.nativeDesktop, true);
      if (unqualified) {
        await route.fulfill({ json: { success: false, error: 'Unverified transport prose: remote_view_handoff_reopen_profile_replacement_required' } });
      } else if (missing && requests.length === 1) {
        await route.fulfill({ json: { success: false, error: 'remote_view_handoff_reopen_profile_replacement_required' } });
      } else {
        await route.fulfill({ json: { success: true, data: { operatorVisible: { state: 'ready' } } } });
      }
    });
    await page.goto('https://fixture.example/remote-view/retained-link');
    assert.equal(requests.length, 0, 'GET must not recreate a target');
    assert.equal(await page.getByRole('heading', { name: 'Browser task closed' }).count(), 1);
    await page.getByRole('button', { name: 'Reopen task', exact: true }).click();
    if (unqualified) {
      await page.getByRole('status').filter({ hasText: 'could not be reopened yet' }).waitFor();
      assert.equal(await page.getByRole('button', { name: 'Reopen with a new profile' }).isVisible(), false);
      assert.equal(requests.length, 1);
      assert.equal(visits, 1);
      observeOwnedProcesses();
      await context.close();
      continue;
    }
    if (missing) {
      const fresh = page.getByRole('button', { name: 'Reopen with a new profile', exact: true });
      await fresh.waitFor({ state: 'visible' });
      assert.equal(requests.length, 1, 'profile loss must not allocate a replacement automatically');
      assert.match(await page.getByRole('status').innerText(), /will not restore previous sign-ins/);
      assert.equal(await page.getByRole('button', { name: 'Reopen task', exact: true }).isVisible(), false);
      await fresh.click();
    }
    await page.waitForFunction(() => document.readyState === 'complete');
    for (let n = 0; n < 40 && visits < 2; n++) await page.waitForTimeout(25);
    assert.equal(visits, 2, 'success must revisit exactly the same durable URL');
    assert.equal(page.url(), 'https://fixture.example/remote-view/retained-link');
    assert.deepEqual(requests.map(request => request.params.handoffOperation), missing ? ['reopen', 'reopen_with_new_profile'] : ['reopen']);
    assert.equal(dialogs, 0, 'reopening uses inline choices, without native dialogs');
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    observeOwnedProcesses();
    await context.close();
  }
  console.log('Handoff reopen UI: same-link navigation, explicit effects, disclosed fresh-profile choice, no dialogs and narrow viewport PASS');
} finally {
  observeOwnedProcesses();
  await browser.close();
  await server.close();
  assert.equal(existsSync(`/proc/${pid}`), false, 'owned test browser must exit');
  for (const [child, start] of observed) {
    try {
      const stat = readFileSync(`/proc/${child}/stat`, 'utf8');
      const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
      assert.notEqual(fields[19], start, `owned browser process ${child} remains`);
    } catch (error) { if (error.code !== 'ENOENT' && error.code !== 'ESRCH') throw error; }
  }
}
