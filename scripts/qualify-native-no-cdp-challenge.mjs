/** Actual managed manual-login fixture. Browser work uses Native only;
 * /proc and the owned HTTPS application independently establish the posture.
 */
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash,randomUUID} from 'node:crypto';
import {qualifyNativeChallengeAssistance} from './qualify-native-challenge-assistance.mjs';

export async function qualifyNativeNoCdpChallenge({workflow,runCli,context,chrome,service,bot,page,nativeOrigin,site,save,selectHandoff,observeOwned,onClosed}) {
  const profile='no-cdp-challenge-'+randomUUID();
  const created=JSON.parse((await runCli(context,['--json','runtime','create',profile,'--browser-family','chromium'])).stdout);
  assert.equal(created.created,true);
  const selection=await workflow('browser_build',{params:{profileId:profile,operationId:randomUUID(),choice:'select_for_next_launch',
    mode:'cdp_free',executablePath:chrome,expectedSha256:createHash('sha256').update(await readFile(chrome)).digest('hex'),ownerConfirmed:true}});
  assert.equal(selection.phase,'selected_for_next_launch');
  const response=await fetch(service.url+`/api/bots/${bot.id}/hosted-browser`,{method:'POST',headers:{origin:service.url,'content-type':'application/json'},
    body:JSON.stringify({mode:'launch',sessionId:bot.threadId,requestId:randomUUID(),taskKey:randomUUID(),taskName:'Owned Native challenge without CDP',profileMode:'existing',profile,url:site+'/native-no-cdp'}),signal:AbortSignal.timeout(120000)});
  const task=await response.json();
  await save('challenge469-no-cdp-open.json',{status:response.status,task,selection});
  assert.equal(response.status,201);
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
  await frame.locator('#desktop-surface').focus();
  await page.keyboard.type('thisisunsafe');
  const deadline=Date.now()+15000;
  let arrived=false;
  while(Date.now()<deadline){const state=await (await fetch(site+'/native/challenge/evidence')).json();if(state.pageRequests>0){arrived=true;break;}await new Promise(resolve=>setTimeout(resolve,100));}
  assert(arrived,'Owned HTTPS challenge must actually load');
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Give control back',exact:true}).click();
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Take control',exact:true}).waitFor();
  const capture=await workflow('browser_command',{taskId:task.taskId,action:'native_capture',params:{classifyChallenges:true}});
  const process=capture.capture.processIdentity;
  const kernel=await observeOwned(process.pid,created.userDataDir);
  const cmdline=(await readFile(`/proc/${process.pid}/cmdline`,'utf8')).split('\0').filter(Boolean);
  assert(!cmdline.some(arg=>arg.startsWith('--remote-debugging-')),'Managed browser must have no debugger switches');
  assert(!cmdline.includes('--enable-automation'));
  const receipt=await qualifyNativeChallengeAssistance({workflow,task,page,nativeOrigin,site:site+'/native',save,consumerControl:control,alreadyStaged:true,
    browserPosture:'Managed manual_login; /proc verified no remote-debugging switches; no CDP connection opened'});
  await workflow('browser_close',{taskId:task.taskId});
  onClosed(task.taskId);
  await save('challenge469-no-cdp-acceptance.json',{...receipt,managedManualLogin:true,process:kernel,debuggerSwitches:false,cdpConnectionsOpened:0});
  return receipt;
}
