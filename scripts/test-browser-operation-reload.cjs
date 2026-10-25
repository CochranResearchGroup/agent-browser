const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'browser-operation-reload-'));
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/browser-mode-inspection.tsx'), artifacts);
  const server = http.createServer((req,res) => {
    res.setHeader('Content-Type',req.url==='/bundle.js'?'application/javascript':'text/html');
    res.end(req.url==='/bundle.js'?fs.readFileSync(path.join(artifacts,'bundle.js')):'<div id="root"></div><script src="/bundle.js"></script>');
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  let browser;
  try {
    browser=await chromium.launch({executablePath:process.env.AGENT_BROWSER_TEST_CHROME||'/usr/bin/google-chrome',headless:true});
    const origin=`http://127.0.0.1:${server.address().port}`;
    const checks=[];
    const open=async(page,kind)=>{
      await page.getByRole('button',{name:'Browser mode for Synthetic browser: No CDP',exact:true}).focus();
      await page.keyboard.press('ArrowDown');
      await page.getByRole('menuitem',{name:kind==='mode_convert'?'Relaunch without CDP':'Duplicate profile',exact:true}).click();
    };
    const operation=(kind,id,phase)=>({schemaVersion:kind==='mode_convert'?'agent-browser.mode-operation.v1':'agent-browser.profile-duplication.v1',operationId:id,kind,browserId:'browser',profileId:'profile',taskId:'task-one',source:{id:'profile'},destination:{id:'destination-'+id,name:'Copy '+id},requestedMode:'cdp_free',phase,affectedTaskIds:['task-one'],actions:phase==='prepared'?[{action:kind==='mode_convert'?'relaunch':'close_and_duplicate',requiresConfirmation:true},{action:'cancel',requiresConfirmation:false}]:[{action:'reconcile',requiresConfirmation:false}]});
    const lookup=(kind,operations)=>({schemaVersion:'agent-browser.browser-operations.v1',kind,profileId:'profile',browserId:'browser',taskId:'task-one',operations});
    for(const kind of ['mode_convert','profile_duplicate'])for(const state of ['prepared','pending','lost_response']) {
      const page=await browser.newPage();const commands=[];const id=`original-${kind}-${state}`;
      let retained=operation(kind,id,state==='pending'?'launch_dispatched':'prepared');
      let loseReply=state==='lost_response';
      await page.route('**/api/command',async route=>{
        const command=route.request().postDataJSON();commands.push(command);
        if(command.choice==='inspect'){await route.fulfill({json:{success:true,data:lookup(kind,[retained])}});return;}
        if(loseReply){loseReply=false;retained=operation(kind,id,'launch_dispatched');await route.abort('failed');return;}
        await route.fulfill({json:{success:true,data:retained}});
      });
      await page.goto(origin);await open(page,kind);
      await page.getByText(`Operation: ${id}`,{exact:true}).waitFor({timeout:2500});
      assert.equal(commands.length,1);assert.equal(commands[0].choice,'inspect');
      if(state==='lost_response'){
        await page.getByRole('checkbox',{name:'Confirm the selected browser action'}).check();
        await page.getByRole('button',{name:kind==='mode_convert'?'Relaunch now':'Close source and duplicate',exact:true}).click();
        await page.getByRole('button',{name:'Reconcile operation',exact:true}).waitFor();
        assert.equal(commands.at(-1).operationId,id);
      }
      const beforeReload=commands.filter(command=>command.choice!=='inspect').length;
      await page.reload();await open(page,kind);
      await page.getByText(`Operation: ${id}`,{exact:true}).waitFor({timeout:2500});
      assert.equal(commands.filter(command=>command.choice!=='inspect').length,beforeReload,'Reload and selection never dispatch browser effects');
      if(state==='prepared')await page.getByRole('checkbox',{name:'Confirm the selected browser action'}).check();
      await page.getByRole('button',{name:state==='prepared'?(kind==='mode_convert'?'Relaunch now':'Close source and duplicate'):'Reconcile operation',exact:true}).click();
      await page.waitForFunction(()=>document.querySelector('[data-mode-operation-busy],[data-profile-operation-busy]')===null);
      assert.equal(commands.at(-1).operationId,id,'Remediation resumes the original operation after full reload');
      assert.equal(commands.at(-1).choice,state==='prepared'?(kind==='mode_convert'?'relaunch':'close_and_duplicate'):'reconcile');
      assert.equal(await page.evaluate(()=>localStorage.length+sessionStorage.length),0,'No profile, operation or credential data is persisted in the client');
      checks.push(`${kind}:${state}`);await page.close();
    }
    {
      const page=await browser.newPage();const commands=[];
      const records=[operation('profile_duplicate','copy-one','launch_dispatched'),operation('profile_duplicate','copy-two','launch_dispatched')];
      await page.route('**/api/command',async route=>{const command=route.request().postDataJSON();commands.push(command);await route.fulfill({json:{success:true,data:command.choice==='inspect'?lookup('profile_duplicate',records):records[1]}});});
      await page.goto(origin);await open(page,'profile_duplicate');
      await page.getByText('Select an existing operation',{exact:true}).waitFor();
      assert.equal(commands.length,1);assert.equal(await page.getByRole('button',{name:'Reconcile operation',exact:true}).count(),0);
      await page.getByRole('button',{name:'Resume Copy copy-two (copy-two)',exact:true}).click();
      assert.equal(commands.length,1,'Selecting an operation only selects owner readback');
      await page.getByRole('button',{name:'Reconcile operation',exact:true}).click();
      await page.waitForFunction(()=>document.querySelector('[data-profile-operation-busy]')===null);
      assert.equal(commands.at(-1).operationId,'copy-two');checks.push('multiple-copy-selection');await page.close();
    }
    {
      const page=await browser.newPage();const commands=[];const active=operation('mode_convert','busy-original','prepared');
      await page.route('**/api/command',async route=>{const command=route.request().postDataJSON();commands.push(command);await route.fulfill({json:{success:true,data:command.choice==='inspect'?lookup('mode_convert',[]):command.choice==='prepare'?{...lookup('mode_convert',[active]),state:'busy',activeOperation:active}:active}});});
      await page.goto(origin);await open(page,'mode_convert');await page.getByRole('button',{name:'Review relaunch',exact:true}).click();
      await page.getByText('Operation: busy-original',{exact:true}).waitFor();
      await page.getByRole('checkbox',{name:'Confirm the selected browser action'}).check();await page.getByRole('button',{name:'Relaunch now',exact:true}).click();
      await page.waitForFunction(()=>document.querySelector('[data-mode-operation-busy]')===null);
      assert.equal(commands.at(-1).operationId,'busy-original');checks.push('busy-original-locator');await page.close();
    }
    {
      const page=await browser.newPage();const commands=[];
      await page.route('**/api/command',async route=>{const command=route.request().postDataJSON();commands.push(command);await route.fulfill({json:command.choice==='inspect'?{success:true,data:lookup('mode_convert',[])}:{success:false,error:'mode_operation_affected_tasks_changed:review again'}});});
      await page.goto(origin);await open(page,'mode_convert');await page.getByRole('button',{name:'Review relaunch',exact:true}).click();
      await page.getByText('mode_operation_affected_tasks_changed:review again',{exact:true}).waitFor();
      assert.equal(await page.getByRole('button',{name:'Reconcile operation',exact:true}).count(),0,'Known refusal is not uncertain dispatch');
      await page.getByRole('button',{name:'Inspect existing operations',exact:true}).click();
      await page.waitForFunction(()=>!document.querySelector('[role=alert]'));
      assert.equal(commands.at(-1).choice,'inspect');checks.push('known-refusal-readback');await page.close();
    }
    for (const [kind, phase, action, label] of [
      ['mode_convert', 'launch_absent', 'relaunch', 'Relaunch now'],
      ['mode_convert', 'launch_absent', 'restore_previous', 'Restore previous mode'],
      ['mode_convert', 'replacement_pending', 'restore_previous', 'Restore previous mode'],
      ['profile_duplicate', 'launch_absent', 'retry_launch', 'Retry opening copy'],
      ['profile_duplicate', 'launch_absent', 'finish_without_opening', 'Finish without opening'],
    ]) {
      const page=await browser.newPage(); const commands=[]; const id=`recovery-${kind}-${action}-${phase}`;
      const retained={...operation(kind,id,phase),actions:[{action,requiresConfirmation:true},{action:'reconcile',requiresConfirmation:false}]};
      await page.route('**/api/command',async route=>{const command=route.request().postDataJSON();commands.push(command);await route.fulfill({json:{success:true,data:command.choice==='inspect'?lookup(kind,[retained]):action==='restore_previous'?{...retained,phase:'completed',effectiveMode:'cdp',actions:[]}:retained}});});
      await page.goto(origin);await open(page,kind);
      await page.getByText(`Operation: ${id}`,{exact:true}).waitFor();
      const button=page.getByRole('button',{name:label,exact:true});
      assert.equal(await button.isDisabled(),true,'Recovered consequential choice requires fresh confirmation');
      await page.getByRole('checkbox',{name:'Confirm the selected browser action'}).check();await button.click();
      await page.waitForFunction(()=>document.querySelector('[data-mode-operation-busy],[data-profile-operation-busy]')===null);
      assert.equal(commands.at(-1).operationId,id);assert.equal(commands.at(-1).choice,action);assert.equal(commands.at(-1).ownerConfirmed,true);
      if(action==='restore_previous') await page.getByText('Effective mode: CDP',{exact:true}).waitFor();
      checks.push(`${kind}:${phase}:${action}`);await page.close();
    }
    console.log(JSON.stringify({success:true,seam:'mounted full-reload operation recovery',checks,artifacts}));
  } finally {await browser?.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;});
