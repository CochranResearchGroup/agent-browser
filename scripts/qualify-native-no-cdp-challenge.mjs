/** Actual managed manual-login fixture. Browser work uses Native only;
 * /proc and the owned HTTPS application independently establish the posture.
 */
import assert from 'node:assert/strict';
import {readFile,realpath,writeFile} from 'node:fs/promises';
import {createHash,randomUUID} from 'node:crypto';
import {join} from 'node:path';
import {qualifyNativeChallengeAssistance} from './qualify-native-challenge-assistance.mjs';

export async function qualifyNativeNoCdpChallenge({workflow,runCli,context,chrome,service,bot,page,nativeOrigin,site,save,selectHandoff,observeOwned,onClosed}) {
  const profile='no-cdp-challenge-'+randomUUID();
  const created=JSON.parse((await runCli(context,['--json','runtime','create',profile,'--browser-family','chromium'])).stdout);
  assert.equal(created.created,true);
  async function launch(url,taskName){
    const response=await fetch(service.url+`/api/bots/${bot.id}/hosted-browser`,{method:'POST',headers:{origin:service.url,'content-type':'application/json'},
      body:JSON.stringify({mode:'launch',sessionId:bot.threadId,requestId:randomUUID(),taskKey:randomUUID(),taskName,profileMode:'existing',profile,url}),signal:AbortSignal.timeout(120000)});
    const task=await response.json();
    await save('challenge469-no-cdp-'+(url==='about:blank'?'profile-preparation':'open')+'.json',{status:response.status,task});
    assert.equal(response.status,201);
    selectHandoff(task);
    return task;
  }
  // The build owner selects existing owner-catalog profiles. Import this new
  // disposable profile through an explicit ordinary blank launch and close;
  // it is a different process, not the qualified manual-login browser below.
  const prepared=await launch('about:blank','Prepare owned synthetic profile');
  await workflow('browser_close',{taskId:prepared.taskId});onClosed(prepared.taskId);
  const profilePath=await realpath(created.userDataDir),fixtureRoot=await realpath(context.tempHome);
  assert(profilePath.startsWith(fixtureRoot+'/'),'First-run setup must remain inside this newly owned fixture');
  // Same synthetic setup as smoke-mcp-manual-login.py:379. This models an
  // already configured empty browser, not legal consent, login or auth proof.
  await writeFile(join(profilePath,'First Run'),'',{flag:'wx',mode:0o600});
  await save('challenge469-no-cdp-modeled-first-run.json',{fixtureOnly:true,profilePath,
    source:'scripts/smoke-mcp-manual-login.py:379',setup:'modeled_first_run_marker',actualLegalConsent:false,loginOrAuthenticationClaimed:false});
  const selection=await workflow('browser_command',{action:'browser_build',params:{profileId:profile,operationId:randomUUID(),choice:'select_for_next_launch',
    mode:'cdp_free',executablePath:chrome,expectedSha256:createHash('sha256').update(await readFile(chrome)).digest('hex'),ownerConfirmed:true}});
  assert.equal(selection.phase,'selected_for_next_launch');
  const task=await launch(site+'/native-no-cdp','Owned Native challenge without CDP');
  await save('challenge469-no-cdp-selection.json',selection);
  assert.equal(task.mode,'manual_login');
  const control=selectHandoff(task);
  await page.goto(task.handoffUrl,{waitUntil:'domcontentloaded',timeout:30000});
  const frame=await (async()=>{
    const deadline=Date.now()+30000;
    while(Date.now()<deadline){const candidate=page.frames().find(f=>f.url().startsWith(nativeOrigin+'/'));if(candidate){try{await candidate.waitForFunction(()=>document.querySelector('#connection-status')?.dataset.state==='connected',null,{timeout:1000});return candidate;}catch{}}
      await new Promise(resolve=>setTimeout(resolve,100));}
    throw Error('no_cdp_native_viewer_unavailable');
  })();
  // The generated loopback certificate is intentionally not globally trusted.
  // A real Native operator dismisses only this owned synthetic interstitial.
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Take control',exact:true}).click();
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Give control back',exact:true}).waitFor();
  const taken=await control();assert.equal(taken.control.owner,'human');assert.equal(taken.native.owner,'operator');
  await frame.waitForFunction(()=>['active','released'].includes(document.querySelector('#input-status')?.dataset.state));
  await frame.locator('#desktop-surface').focus();
  await save('challenge469-no-cdp-interstitial-before.json',{png:(await page.screenshot()).toString('base64')});
  await frame.locator('#desktop-surface').click({position:{x:200,y:250}});
  await page.keyboard.type('thisisunsafe');
  await save('challenge469-no-cdp-interstitial-after.json',{png:(await page.screenshot()).toString('base64')});
  const deadline=Date.now()+15000;
  let arrived=false;
  while(Date.now()<deadline){const state=await (await fetch(site+'/native/challenge/evidence')).json();if(state.pageRequests>0){arrived=true;break;}await new Promise(resolve=>setTimeout(resolve,100));}
  if(!arrived)throw Object.assign(Error('Owned HTTPS challenge must actually load'),{code:'no_cdp_synthetic_https_not_loaded'});
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Give control back',exact:true}).click();
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Take control',exact:true}).waitFor();
  const capture=await workflow('browser_command',{taskId:task.taskId,action:'native_capture',params:{classifyChallenges:true}});
  const process=capture.capture.processIdentity;
  const kernel=await observeOwned(process.pid,created.userDataDir);
  const cmdline=(await readFile(`/proc/${process.pid}/cmdline`,'utf8')).split('\0').filter(Boolean);
  assert(!cmdline.some(arg=>arg.startsWith('--remote-debugging-')),'Managed browser must have no debugger switches');
  assert(!cmdline.includes('--enable-automation'));
  const receipt=await qualifyNativeChallengeAssistance({workflow,task,page,nativeOrigin,site:site+'/native',
    save:(name,value)=>save(name.replace('challenge469-','challenge469-no-cdp-'),value),consumerControl:control,alreadyStaged:true,
    browserPosture:'Managed manual_login; /proc verified no remote-debugging switches; no CDP connection opened'});
  await workflow('browser_close',{taskId:task.taskId});
  onClosed(task.taskId);
  await save('challenge469-no-cdp-acceptance.json',{...receipt,managedManualLogin:true,process:kernel,debuggerSwitches:false,cdpConnectionsOpened:0,
    profileSetup:'Modeled first-run marker on newly owned empty synthetic profile; no legal consent, login or authentication claim'});
  return receipt;
}
