const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'browser-mode-inspection-'));
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/browser-mode-inspection.tsx'), artifacts);
  const server = http.createServer((req, res) => {
    res.setHeader('Content-Type', req.url === '/bundle.js' ? 'application/javascript' : 'text/html');
    res.end(req.url === '/bundle.js' ? fs.readFileSync(path.join(artifacts, 'bundle.js')) : '<div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  let browser;
  try {
    browser = await chromium.launch({ executablePath: process.env.AGENT_BROWSER_TEST_CHROME || '/usr/bin/google-chrome', headless: true });
    const page = await browser.newPage();
    await page.clock.install();
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.getByRole('button', { name: 'Browser mode for Synthetic browser: No CDP', exact: true }).waitFor();
    await page.getByRole('button', { name: 'Browser mode for Synthetic browser: No CDP', exact: true }).hover();
    assert.match(await page.getByRole('menu').innerText(), /Ownership: Managed/);
    await page.keyboard.press('Escape');
    const commands=[];
    await page.route('**/api/command',async route=>{
      const command=route.request().postDataJSON();commands.push(command);
      if (commands.length===2) { await route.abort('failed'); return; }
      await route.fulfill({json:{success:true,data:{schemaVersion:'agent-browser.profile-duplication.v1',operationId:command.operationId,state:'source_review',affectedTaskIds:['task-one','task-two'],actions:[{action:'close_and_duplicate',requiresConfirmation:true},{action:'cancel',requiresConfirmation:false}]}}});
    });
    await page.getByRole('button',{name:'Browser mode for Synthetic browser: No CDP',exact:true}).click();
    await page.getByRole('menuitem',{name:'Duplicate profile',exact:true}).click({timeout:750});
    await page.getByRole('dialog',{name:'Duplicate browser profile',exact:true}).waitFor();
    assert.equal(commands.length,0,'opening the dialog has no browser effect');
    await page.getByRole('button',{name:'Review duplication',exact:true}).click();
    await page.getByText('task-two',{exact:true}).waitFor();
    assert.equal(commands[0].action,'browser_session_profile_duplicate');
    assert.equal(commands[0].sourceProfileId,'profile');
    assert.equal(commands[0].choice,'prepare');
    const operationId=commands[0].operationId;
    assert.equal(await page.getByRole('button',{name:'Close source and duplicate',exact:true}).isEnabled(),false);
    await page.getByRole('checkbox',{name:'Confirm the selected browser action'}).check();
    await page.getByRole('button',{name:'Close source and duplicate',exact:true}).click();
    await page.waitForFunction(()=>document.querySelector('[data-profile-operation-busy]')===null);
    assert.equal(commands[1].operationId,operationId,'remediation retains the same durable operation');
    assert.equal(commands[1].ownerConfirmed,true);
    await page.getByRole('button',{name:'Reconcile operation',exact:true}).waitFor();
    assert.equal(await page.getByRole('button',{name:'Close source and duplicate',exact:true}).count(),0,'lost reply removes effect replay choices');
    await page.getByRole('button',{name:'Reconcile operation',exact:true}).click();
    await page.getByText('task-two',{exact:true}).waitFor();
    assert.equal(commands[2].operationId,operationId);
    assert.equal(commands[2].choice,'reconcile');
    await page.getByRole('button',{name:'Close',exact:true}).click();
    // A stopped or failed inventory poll must not leave a verified mode forever.
    await page.clock.fastForward(31_000);
    await page.getByRole('button', { name: 'Browser mode for Synthetic browser: Unverified', exact: true }).waitFor({ timeout: 1500 });
    assert.equal(await page.getByRole('button', { name: /No CDP/ }).count(), 0);
    await page.getByRole('button', { name: 'Browser mode for Synthetic browser: Unverified', exact: true }).hover();
    assert.match(await page.getByRole('menu').innerText(), /Ownership: Unverified/);
    await page.keyboard.press('Escape');
    const observed = page.getByRole('button', { name: 'Browser mode for Observed synthetic window: Unverified', exact: true });
    await observed.waitFor({ timeout: 1500 });
    await observed.hover();
    assert.match(await page.getByRole('menu').innerText(), /Ownership: Observed/);
    await page.keyboard.press('Escape');
    console.log(JSON.stringify({ success: true, seam: 'mounted mode button; independent evidence expiry', artifacts }));
  } finally {
    await browser?.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
