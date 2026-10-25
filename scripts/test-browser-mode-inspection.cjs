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
    await page.clock.pauseAt(new Date(Date.now() + 100));
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    await page.getByRole('button', { name: 'Browser mode for Synthetic browser: No CDP', exact: true }).waitFor();
    await page.getByRole('button', { name: 'Browser mode for Synthetic browser: No CDP', exact: true }).hover();
    assert.match(await page.getByRole('menu').innerText(), /Ownership: Managed/);
    await page.getByRole('menuitem', {name:'Relaunch without CDP',exact:true}).waitFor({timeout:750});
    await page.getByRole('menuitem', {name:'Relaunch with CDP',exact:true}).waitFor({timeout:750});
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Browser mode for Synthetic browser: No CDP', exact: true }).click();
    await page.getByRole('menuitem', {name:'Choose browser build',exact:true}).waitFor({timeout:750});
    await page.keyboard.press('Escape');
    const commands=[];
    await page.route('**/api/command',async route=>{
      const command=route.request().postDataJSON();if(command.choice==='inspect'){await route.fulfill({json:{success:true,data:{schemaVersion:'agent-browser.browser-operations.v1',kind:'profile_duplicate',profileId:'profile',browserId:'browser',taskId:'task-one',operations:[]}}});return;}commands.push(command);
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
    const modeCommands=[];
    await page.unroute('**/api/command');
    await page.route('**/api/command', async route => {
      const command=route.request().postDataJSON();if(command.choice==='inspect'){await route.fulfill({json:{success:true,data:{schemaVersion:'agent-browser.browser-operations.v1',kind:'mode_convert',taskId:'task-one',profileId:'profile',browserId:'browser',operations:modeCommands.length?[{schemaVersion:'agent-browser.mode-operation.v1',operationId:modeCommands[0].operationId,phase:'prepared',affectedTaskIds:['task-one','task-two'],actions:[{action:'relaunch',requiresConfirmation:true},{action:'cancel',requiresConfirmation:false}]}]:[]}}});return;}modeCommands.push(command);
      if(modeCommands.length===2){await route.abort('failed');return;}
      await route.fulfill({json:{success:true,data:{schemaVersion:'agent-browser.mode-operation.v1',operationId:command.operationId,phase:'prepared',affectedTaskIds:['task-one','task-two'],actions:[{action:'relaunch',requiresConfirmation:true},{action:'cancel',requiresConfirmation:false}]}}});
    });
    await page.getByRole('button',{name:'Browser mode for Synthetic browser: No CDP',exact:true}).click();
    await page.getByRole('menuitem',{name:'Relaunch without CDP',exact:true}).click();
    await page.getByRole('dialog',{name:'Change browser mode',exact:true}).waitFor();
    assert.equal(modeCommands.length,0,'opening mode dialog has no relaunch effect');
    await page.getByRole('button',{name:'Review relaunch',exact:true}).click();
    await page.getByText('task-two',{exact:true}).waitFor();
    assert.equal(modeCommands[0].sessionId,'task-one');
    assert.equal(modeCommands[0].sessionName,'synthetic-owner');
    assert.equal(modeCommands[0].mode,'cdp_free');
    assert.equal(await page.getByRole('button',{name:'Relaunch now',exact:true}).isEnabled(),false);
    await page.getByRole('checkbox',{name:'Confirm the selected browser action'}).check();
    await page.getByRole('button',{name:'Relaunch now',exact:true}).click();
    await page.getByRole('button',{name:'Reconcile operation',exact:true}).waitFor();
    assert.equal(await page.getByRole('button',{name:'Relaunch now',exact:true}).count(),0);
    await page.getByRole('button',{name:'Reconcile operation',exact:true}).click();
    await page.getByText('task-two',{exact:true}).waitFor();
    assert.equal(modeCommands[2].operationId,modeCommands[0].operationId);
    assert.equal(modeCommands[2].choice,'reconcile');
    await page.getByRole('button',{name:'Close',exact:true}).click();
    await page.getByRole('button',{name:'Browser mode for Synthetic browser: No CDP',exact:true}).click();
    await page.getByRole('menuitem',{name:'Relaunch without CDP',exact:true}).click();
    await page.getByText('task-two',{exact:true}).waitFor({timeout:750});
    assert.equal(modeCommands.length,3,'closing and reopening retains the operation without a new effect');
    await page.getByRole('button',{name:'Close',exact:true}).click();
    // Build discovery is read-only; a family/version warning cannot veto an owner choice.
    await page.unroute('**/api/command');
    const buildCommands=[];
    let selectedReceipt=null;
    let loseSelectionReply=true;
    let activeBuildTasks=[];
    const choice={buildId:'build:synthetic',expectedSha256:'a'.repeat(64)};
    const build={id:choice.buildId,name:'chromium',family:'chromium',version:'150.0.0.0',available:true,nativeBinary:true,selection:choice,compatibility:{versionRelation:'older',familyRelation:'different'}};
    await page.route('**/api/command',async route=>{
      const command=route.request().postDataJSON(); buildCommands.push(command);
      if(command.action==='browser_session_build_select' && command.choice==='inspect') {
        await route.fulfill({json:{success:true,data:{schemaVersion:'agent-browser.build-selection.v1',profileId:'profile',profileDataVersion:'153.0.0.0',nextLaunchMode:'cdp_free',availableBuilds:[build],activeTasks:activeBuildTasks,currentSelectionOperationId:selectedReceipt}}});return;
      }
      if(command.action==='browser_session_build_select') {
        selectedReceipt=command.operationId;
        if(command.choice==='select_for_next_launch' && loseSelectionReply){loseSelectionReply=false;await route.abort('failed');return;}
        await route.fulfill({json:{success:true,data:{schemaVersion:'agent-browser.build-selection.v1',operationId:command.operationId,phase:command.choice==='open_selected'?'opened':'selected_for_next_launch'}}});return;
      }
      if(command.choice==='inspect') {
        await route.fulfill({json:{success:true,data:{schemaVersion:'agent-browser.browser-operations.v1',kind:command.kind,profileId:'profile',taskId:command.sessionId,browserId:command.browserId,operations:[]}}});return;
      }
      await route.fulfill({json:{success:true,data:{schemaVersion:command.kind==='mode_convert'?'agent-browser.mode-operation.v1':'agent-browser.profile-duplication.v1',operationId:command.operationId,phase:'prepared',actions:[{action:'cancel',requiresConfirmation:false}]}}});
    });
    const openBuild=async()=>{
      await page.getByRole('button',{name:'Browser mode for Synthetic browser: No CDP',exact:true}).click();
      await page.getByRole('menuitem',{name:'Choose browser build',exact:true}).click();
      await page.getByRole('dialog',{name:'Choose browser build',exact:true}).waitFor();
      await page.getByRole('radio',{name:'chromium 150.0.0.0',exact:true}).check();
    };
    await openBuild();
    assert.equal(buildCommands.filter(command=>command.choice!=='inspect').length,0,'discovery has no browser/profile effect');
    assert.match(await page.getByRole('dialog').innerText(),/does not restore profile bytes/);
    assert.equal(await page.getByRole('radio',{name:'No CDP',exact:true}).isChecked(),true,'build discovery preserves independent mode intent');
    await page.getByRole('radio',{name:'CDP',exact:true}).check();
    await page.getByRole('checkbox',{name:'Confirm build migration and login risk'}).check();
    await page.getByRole('button',{name:'Select for next launch',exact:true}).click();
    await page.getByText(/Refresh browser builds to read retained selection/).waitFor();
    assert.equal(await page.getByRole('button',{name:'Select for next launch',exact:true}).isEnabled(),false,'lost reply fences further selection effects');
    await page.getByRole('button',{name:'Refresh browser builds',exact:true}).click();
    await page.getByRole('button',{name:'Open saved build selection',exact:true}).waitFor();
    const selection=buildCommands.find(command=>command.choice==='select_for_next_launch');
    assert.deepEqual({buildId:selection.buildId,expectedSha256:selection.expectedSha256,mode:selection.mode,ownerConfirmed:selection.ownerConfirmed},{...choice,mode:'cdp',ownerConfirmed:true});
    assert.equal(selection.executablePath,undefined,'the rail uses an opaque reviewed choice');
    await page.getByRole('button',{name:'Close',exact:true}).click();
    await openBuild();
    await page.getByRole('button',{name:'Open saved build selection',exact:true}).click();
    await page.getByText(/opened\. Login/).waitFor();
    assert.equal(buildCommands.find(command=>command.choice==='open_selected').operationId,selectedReceipt,'reopening reads the retained selection');
    await page.getByRole('button',{name:'Duplicate and test selected build',exact:true}).click();
    await page.getByRole('button',{name:'Review duplication',exact:true}).click();
    await page.getByRole('button',{name:'Cancel operation',exact:true}).waitFor();
    const duplicateBuild=buildCommands.find(command=>command.action==='browser_session_profile_duplicate' && command.choice==='prepare');
    assert.equal(duplicateBuild.buildId,choice.buildId); assert.equal(duplicateBuild.mode,'cdp_free');
    await page.getByRole('button',{name:'Close',exact:true}).click();
    await page.getByRole('button',{name:'Close',exact:true}).click();
    await openBuild();
    // A user-entered custom path is submitted only after its own inspection;
    // opaque installed choices still expose no path.
    await page.getByRole('textbox',{name:'Other browser executable'}).fill('/reviewed/custom/chrome');
    await page.getByRole('button',{name:'Inspect executable',exact:true}).click();
    await page.getByRole('radio',{name:'chromium 150.0.0.0',exact:true}).check();
    await page.getByRole('checkbox',{name:'Confirm build migration and login risk'}).check();
    await page.getByRole('button',{name:'Select for next launch',exact:true}).click();
    await page.getByText(/selected_for_next_launch\. Login/).waitFor();
    assert.equal(buildCommands.filter(command=>command.choice==='select_for_next_launch').at(-1).executablePath,'/reviewed/custom/chrome');
    await page.getByRole('button',{name:'Close',exact:true}).click();
    activeBuildTasks=[{id:'task-one',name:'synthetic-owner',browserId:'browser'}];
    await openBuild();
    await page.getByRole('radio',{name:'No CDP',exact:true}).check();
    await page.getByRole('button',{name:'Review original profile relaunch',exact:true}).click();
    await page.getByRole('button',{name:'Review relaunch',exact:true}).click();
    await page.getByRole('button',{name:'Cancel operation',exact:true}).waitFor();
    const relaunchBuild=buildCommands.find(command=>command.action==='browser_session_mode_convert' && command.choice==='prepare');
    assert.equal(relaunchBuild.buildId,choice.buildId); assert.equal(relaunchBuild.mode,'cdp_free');
    await page.getByRole('button',{name:'Close',exact:true}).click();
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
