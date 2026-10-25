const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { createRequire } = require('node:module');
const { chromium } = require('playwright');
const { buildWorkspaceViewportFixture } = require('./lib/workspace-viewport-browser-fixture.cjs');
(async () => {
  const root = path.resolve(__dirname, '..');
  const artifacts = fs.mkdtempSync(path.join(os.tmpdir(), 'browser-operation-reload-'));
  await buildWorkspaceViewportFixture(root, path.join(root, 'scripts/fixtures/browser-mode-inspection.tsx'), artifacts);
  // Geometry qualification uses the same product stylesheet and PostCSS plugin as the dashboard.
  const dashboard=path.join(root,'packages/dashboard');
  const dashboardRequire=createRequire(path.join(dashboard,'package.json'));
  const postcss=createRequire(dashboardRequire.resolve('next/package.json'))('postcss');
  const cssPath=path.join(dashboard,'src/app/globals.css');
  const css=await postcss([dashboardRequire('@tailwindcss/postcss')({base:dashboard})]).process(fs.readFileSync(cssPath,'utf8'),{from:cssPath,to:path.join(artifacts,'dashboard.css')});
  fs.writeFileSync(path.join(artifacts,'dashboard.css'),css.css);
  const server = http.createServer((req,res) => {
    res.setHeader('Content-Type',req.url==='/bundle.js'?'application/javascript':req.url==='/dashboard.css'?'text/css':'text/html');
    res.end(req.url==='/bundle.js'?fs.readFileSync(path.join(artifacts,'bundle.js')):req.url==='/dashboard.css'?fs.readFileSync(path.join(artifacts,'dashboard.css')):'<link rel="stylesheet" href="/dashboard.css"><div id="root"></div><script src="/bundle.js"></script>');
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
    {
      const page=await browser.newPage();const commands=[];const id='pause-original';
      let retained={...operation('mode_convert',id,'prepared'),affectedTaskIds:['task-one','task-two'],actions:[{action:'pause_and_relaunch',requiresConfirmation:false},{action:'relaunch',requiresConfirmation:true},{action:'cancel',requiresConfirmation:false}]};
      await page.route('**/api/command',async route=>{
        const command=route.request().postDataJSON();commands.push(command);
        if(command.choice==='pause_and_relaunch')retained={...retained,phase:'paused',actions:[{action:'relaunch',requiresConfirmation:true},{action:'cancel',requiresConfirmation:false}]};
        if(command.choice==='cancel')retained={...retained,phase:'cancelled',actions:[]};
        await route.fulfill({json:{success:true,data:command.choice==='inspect'?lookup('mode_convert',[retained]):retained}});
      });
      await page.goto(origin);await open(page,'mode_convert');
      await page.getByRole('button',{name:'Pause before relaunch',exact:true}).click({timeout:2500});
      await page.getByRole('status').filter({hasText:/^paused$/}).waitFor();
      assert.deepEqual(commands.filter(c=>c.choice!=='inspect').map(c=>c.choice),['pause_and_relaunch'],'Pause never dispatches an immediate relaunch');
      await page.reload();await open(page,'mode_convert');
      await page.getByText('Operation: '+id,{exact:true}).waitFor();
      assert.equal(await page.getByRole('button',{name:'Relaunch now',exact:true}).isDisabled(),true,'Paused relaunch still needs fresh confirmation');
      await page.getByRole('button',{name:'Cancel operation',exact:true}).click();
      await page.getByRole('button',{name:'Prepare another relaunch',exact:true}).waitFor();
      assert.deepEqual(commands.filter(c=>c.choice!=='inspect').map(c=>[c.choice,c.operationId]),[['pause_and_relaunch',id],['cancel',id]],'Reload resumes the original paused operation without automatic effects');
      checks.push('mode_convert:pause-reload-cancel');await page.close();
    }
    {
      const page=await browser.newPage();const commands=[];const id='prompt-original';
      let retained={...operation('mode_convert',id,'close_prompt'),actions:[{action:'cancel',requiresConfirmation:false},{action:'force_close',requiresConfirmation:true},{action:'focus',requiresConfirmation:false}]};
      await page.route('**/api/command',async route=>{const command=route.request().postDataJSON();commands.push(command);if(command.choice==='cancel')retained={...retained,phase:'cancelled',actions:[]};await route.fulfill({json:{success:true,data:command.choice==='inspect'?lookup('mode_convert',[retained]):retained}});});
      await page.goto(origin);await open(page,'mode_convert');
      assert.equal(await page.getByRole('button',{name:'Force close',exact:true}).isDisabled(),true);
      await page.getByRole('button',{name:'Cancel operation',exact:true}).click();
      await page.getByRole('button',{name:'Prepare another relaunch',exact:true}).waitFor();
      assert.deepEqual(commands.filter(c=>c.choice!=='inspect').map(c=>[c.choice,c.operationId]),[['cancel',id]],'Cancelling the close prompt never dispatches force or relaunch');
      checks.push('mode_convert:close-prompt-cancel');await page.close();
    }
    for (const [kind, phase, action, label] of [
      ['mode_convert', 'launch_absent', 'relaunch', 'Relaunch now'],
      ['mode_convert', 'launch_absent', 'restore_previous', 'Restore previous build and mode'],
      ['mode_convert', 'replacement_pending', 'restore_previous', 'Restore previous build and mode'],
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
    for (const kind of ['mode_convert','profile_duplicate']) {
      const page=await browser.newPage({viewport:{width:1280,height:720}});const commands=[];
      const id=`truncated-current-${kind}`;
      const active=operation(kind,id,'prepared');
      const records=[active,operation(kind,`truncated-pending-${kind}`,'prepared'),...Array.from({length:254},(_,index)=>({...operation(kind,`retained-history-${index}`,'cancelled'),actions:[]}))];
      await page.route('**/api/command',async route=>{const command=route.request().postDataJSON();commands.push(command);await route.fulfill({json:{success:true,data:command.choice==='inspect'?{...lookup(kind,records),historyTruncated:true}:active}});});
      await page.goto(origin);await open(page,kind);
      const resumeName=record=>kind==='mode_convert'?`Resume ${record.operationId}`:`Resume ${record.destination.name} (${record.operationId})`;
      const current=page.getByRole('button',{name:resumeName(active),exact:true});
      await current.waitFor();
      const visible=async(locator,message)=>{const box=await locator.boundingBox();const viewport=page.viewportSize();assert(box&&box.y>=0&&box.y+box.height<=viewport.height&&box.x>=0&&box.x+box.width<=viewport.width,message);};
      await visible(current,'The exact current operation stays visible with a full bounded history at an ordinary viewport');
      await page.getByText('Showing bounded operation history. Current and pending operations appear first.',{exact:true}).waitFor({timeout:2500});
      const last=page.getByRole('button',{name:resumeName(records.at(-1)),exact:true});
      await current.hover();await page.mouse.wheel(0,100000);
      const lastElement=await last.elementHandle();
      try { await page.waitForFunction(element=>{const box=element.getBoundingClientRect();return box.top>=0&&box.bottom<=innerHeight;},lastElement,{timeout:2500}); }
      catch(error) { const region=page.getByRole('region',{name:'Existing browser operations'}); console.error(JSON.stringify({kind,artifacts,current:await current.boundingBox(),last:await last.boundingBox(),region:await region.boundingBox(),dialog:await page.getByRole('dialog').boundingBox(),scroll:await region.evaluate(element=>({top:element.scrollTop,height:element.scrollHeight,visible:element.clientHeight}))}));await page.screenshot({path:path.join(artifacts,`truncated-${kind}-failure.png`)});throw error; }
      await visible(last,'Retained history stays reachable by scrolling');
      await page.mouse.wheel(0,-100000);
      const currentElement=await current.elementHandle();
      await page.waitForFunction(element=>{const box=element.getBoundingClientRect();return box.top>=0&&box.bottom<=innerHeight;},currentElement,{timeout:2500});
      assert.equal(commands.length,1,'Inspecting and scrolling history have no browser effects');
      await current.click();assert.equal(commands.length,1,'Choosing an original operation remains read-only');
      const confirmation=page.getByRole('checkbox',{name:'Confirm the selected browser action'});
      const action=page.getByRole('button',{name:kind==='mode_convert'?'Relaunch now':'Close source and duplicate',exact:true});
      await visible(confirmation,'Fresh confirmation remains visible after operation selection');
      await visible(action,'The selected remediation stays reachable');
      assert.equal(await action.isDisabled(),true);await confirmation.check();await action.click();
      await page.waitForFunction(()=>document.querySelector('[data-mode-operation-busy],[data-profile-operation-busy]')===null);
      assert.equal(commands.at(-1).operationId,id);assert.equal(commands.at(-1).ownerConfirmed,true);
      checks.push(`${kind}:truncated-history-workflow`);await page.close();
    }
    console.log(JSON.stringify({success:true,seam:'mounted full-reload operation recovery',checks,artifacts}));
  } finally {await browser?.close();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;});
