/** Owned synthetic HTTPS oracle and current Native checkbox qualification.
 * CDP only stages/observes this owned app and supplies the explicitly required
 * owner origin/focus admission. Challenge perception/input use actual Native.
 */
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash,randomUUID} from 'node:crypto';
import {createChallengeAssistanceClient} from '../packages/client/src/challenge-assistance.js';

export function createNativeCheckboxOracle() {
  const scenes=new Map();
  return {
    create(id,options={}){assert(!scenes.has(id));const value={id,inputEvents:0,passiveTransitions:0,cleared:false,browserAccessRequests:0,backendAccessRequests:0,allowAccess:options.allowAccess!==false,image:options.image===true};scenes.set(id,value);return value;},
    state:id=>structuredClone(scenes.get(id)),
    passive(id){const value=scenes.get(id);assert(value);value.passiveTransitions++;value.cleared=true;},
    handle(req,res){
      const match=/^\/checkbox470\/([-a-z0-9]+)\/(input|clearance|protected)$/.exec(req.url??'');
      if(!match)return false;
      const value=scenes.get(match[1]);if(!value){res.writeHead(404);res.end();return true;}
      const operation=match[2];let status=200;
      if(operation==='input'&&req.method==='POST'){value.inputEvents++;value.cleared=!value.image;}
      else if(operation==='protected'&&req.method==='GET'){
        if(req.headers['x-synthetic-browser']==='1')value.browserAccessRequests++;else value.backendAccessRequests++;
        if(!value.cleared||!value.allowAccess)status=403;
      }else if(operation!=='clearance'||req.method!=='GET'){res.writeHead(405);res.end();return true;}
      res.writeHead(status,{'content-type':'application/json','cache-control':'no-store'});
      res.end(JSON.stringify({...value,report:status===200&&value.cleared&&value.allowAccess?'ready':'unavailable'}));return true;
    },
  };
}

export async function qualifyNativeCheckbox({workflow,task,nextTask,primary,site,profilePath,configFile,faultFile,oracle,save,continuity,observeOnly=false}) {
  // Prepare the owned app's existing window before observation. Product focus
  // must be idempotent here; a changed frame in the attempt still refuses.
  const staging=await primary.context().newCDPSession(primary);
  try{const window=await staging.send('Browser.getWindowForTarget');await staging.send('Browser.setWindowBounds',{windowId:window.windowId,bounds:{windowState:'maximized'}});
    await save('checkbox470-window-preparation.json',{appOwnedCdp:true,windowId:window.windowId,windowState:'maximized',challengeInputDispatched:false});
  }finally{await staging.detach();}
  const provider={id:'owned-native-provider',kind:'captcha',enabled:true,capabilities:['captcha_solve'],configRef:'agent-browser:remote-view-native:v1'};
  const sitePolicy={id:'owned-checkbox-site',originPattern:site,origins:[],challengePolicy:'provider_allowed',allowedChallengeProviders:[provider.id]};
  const config={service:{profiles:{'borrowed-profile':{id:'borrowed-profile',name:'Owned fixture profile',userDataDir:profilePath,profileOrigin:'external_byop',profileClass:'operator_supplied',persistent:true,sitePolicyIds:[sitePolicy.id]}},
    sitePolicies:{[sitePolicy.id]:sitePolicy},providers:{[provider.id]:provider}}};
  await writeFile(configFile,JSON.stringify(config),{mode:0o600});
  const receipts=[];
  async function stage(scene,{family='hcaptcha',passive=false}={}) {
    await primary.evaluate(({scene,family,passive})=>{
      document.querySelector('#checkbox470')?.remove();
      const panel=document.createElement('section');panel.id='checkbox470';
      panel.style.cssText='position:fixed;inset:0;z-index:999999;background:white;color:black;font:14px Arial,sans-serif';
      panel.innerHTML='<div style="position:absolute;right:10px;bottom:10px;background:#00ff00;width:80px;height:40px"></div><h1 style="position:absolute;left:20px;top:20px;font:20px Arial">Owned Native checkbox fixture</h1>';
      if(passive)panel.innerHTML+='<p style="position:absolute;left:100px;top:150px;font:24px Arial">Checking your browser</p>';
      else {
        const size=family==='hcaptcha'?30:24,gap=family==='hcaptcha'?16:8;
        const row=document.createElement('div');row.style.cssText='position:absolute;left:100px;top:150px;display:flex;align-items:center;gap:'+gap+'px;height:'+size+'px';
        const box=document.createElement('button');box.type='button';box.style.cssText='box-sizing:border-box;display:block;margin:0;padding:0;border:2px solid #222;border-radius:0;background:white;width:'+size+'px;height:'+size+'px;min-width:'+size+'px';
        // One declared synthetic candidate from the retained four-variant
        // preparation. The existing registered geometry is unchanged.
        const prompt=document.createElement('span');prompt.style.cssText='font:'+(family==='hcaptcha'?'18px Georgia,serif':'18px Arial,sans-serif')+';color:black;white-space:nowrap;transform:translateY(2px)';prompt.textContent=family==='hcaptcha'?'I am human':'Verify you are human';
        row.append(box,prompt);panel.append(row);
        const brand=document.createElement('span');brand.textContent=family==='hcaptcha'?'hCaptcha':'Cloudflare';brand.style.cssText='position:absolute;left:340px;top:156px;font:18px Arial,sans-serif';panel.append(brand);
        box.onclick=async()=>{
          const applied=await fetch('/checkbox470/'+scene+'/input',{method:'POST'});const state=await applied.json();
          if(state.image){panel.innerHTML='<h1>Select all images containing buses</h1><p>hCaptcha</p>';return;}
          const report=await fetch('/checkbox470/'+scene+'/protected',{headers:{'x-synthetic-browser':'1'}});
          panel.innerHTML=report.ok?'<h1>Report ready</h1>':'<h1>Review pending</h1>';
        };
      }
      document.body.append(panel);
      window.completePassive470=async()=>{const report=await fetch('/checkbox470/'+scene+'/protected',{headers:{'x-synthetic-browser':'1'}});if(report.ok)panel.innerHTML='<h1>Report ready</h1>';};
    },{scene,family,passive});
    await primary.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    await save('checkbox470-rendered-state.json',await primary.evaluate(()=>{
      const panel=document.querySelector('#checkbox470'),box=panel?.querySelector('button');
      const rect=element=>element?Object.fromEntries(['x','y','width','height'].map(key=>[key,element.getBoundingClientRect()[key]])):null;
      return {synthetic:true,ready:!!panel,text:panel?.textContent?.slice(0,512),panel:rect(panel),box:rect(box),visibility:document.visibilityState,viewport:{width:innerWidth,height:innerHeight}};
    }));
  }
  async function sourcePolicy(context) {
    const bytes=await readFile(configFile),current=JSON.parse(bytes);
    const currentTask=await workflow('browser_command',{taskId:context.taskId,action:'task_briefing'});
    return {taskId:context.taskId,binding:Object.fromEntries(['taskId','profileId','browserId','tabId','targetId'].map(key=>[key,currentTask[key]])),
      origin:site,sitePolicy:current.service.sitePolicies[sitePolicy.id],provider:current.service.providers[provider.id],profile:current.service.profiles[currentTask.profileId],
      taskPolicy:await workflow('browser_policy',{taskId:context.taskId,operation:'status'}),sourceRef:'owned-config-sha256:'+createHash('sha256').update(bytes).digest('hex')};
  }
  async function run(label,family='hcaptcha',options={}) {
    const current=label==='hcaptcha-success'?task:await nextTask(label);
    const scene=label+'-'+randomUUID();oracle.create(scene,options);await stage(scene,{family,passive:options.passive});
    const originalId='native470:'+randomUUID();let passiveAdvanced=false;
    const client=createChallengeAssistanceClient({
      callTool:async(name,args)=>{
        const value=await workflow(name,args);
        if(options.passive&&name==='browser_command'&&args.action==='native_capture'&&value.challenge?.kind==='passive_check'&&!passiveAdvanced){
          await save('checkbox470-passive-initial.json',{classification:value.challenge,frameSha256:value.capture.capture.sha256,actualNative:true,productInputEvents:0});
          passiveAdvanced=true;oracle.passive(scene);await primary.evaluate(()=>window.completePassive470());
        }
        return {content:[{type:'text',text:JSON.stringify({success:true,data:value})}]};
      },resolvePolicy:sourcePolicy,
      verifyClearance:async context=>{const response=await fetch(site+'/checkbox470/'+scene+'/clearance');const value=await response.json();return {state:response.ok&&value.cleared?'verified':'unverified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'owned-https-clearance:'+scene};},
      verifyAccess:async context=>{const response=await fetch(site+'/checkbox470/'+scene+'/protected');const value=await response.json();return {state:response.ok&&value.report==='ready'?'verified':'unverified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'owned-https-protected:'+scene};},
    });
    if(options.dropReply)await writeFile(faultFile,JSON.stringify({state:'armed',kind:'drop_input_reply_once'}),{mode:0o600});
    const result=await client.resolve({taskId:current.taskId,requestId:originalId});
    const application=oracle.state(scene);
    await save('checkbox470-'+label+'.json',{result,application,task:current,originalId,family,synthetic:true});
    if(options.passive){assert.equal(result.status,'verified_access_after_passive_wait');assert.equal(application.inputEvents,0);assert.equal(application.passiveTransitions,1);}
    else if(options.dropReply){
      assert(['awaiting_operator','intervention'].includes(result.status));assert.equal(application.inputEvents,1);
      const repeated=await client.resolve({taskId:current.taskId,requestId:originalId});assert.equal(oracle.state(scene).inputEvents,1);assert.equal(repeated.retryAllowed,false);
      const original=await workflow('browser_command',{taskId:current.taskId,action:'task_operation',params:{requestId:originalId}});assert.equal(original.operation.state,'uncertain');
      await save('checkbox470-uncertain-original.json',{original,repeated});
    }else if(options.allowAccess===false){assert(['awaiting_operator','intervention'].includes(result.status));assert.equal(application.inputEvents,1);assert.equal(application.cleared,true);}
    else if(options.image){assert(['awaiting_operator','intervention'].includes(result.status));assert.equal(application.inputEvents,1);assert.equal(application.cleared,false);}
    else {assert.equal(result.status,'verified_access_after_native',JSON.stringify(result));assert.equal(application.inputEvents,1);assert.equal(application.cleared,true);assert(application.browserAccessRequests>0);}
    assert.equal(result.retryAllowed,false);await continuity('checkbox470-'+label);
    receipts.push({label,taskId:current.taskId,status:result.status,inputEvents:application.inputEvents,cleared:application.cleared,accessAllowed:application.allowAccess,originalId});
    return {task:current,scene,originalId};
  }
  if(observeOnly){
    const scene='observe-only-'+randomUUID();oracle.create(scene,{});await stage(scene);
    const currentUrl=await workflow('browser_command',{taskId:task.taskId,action:'url'});
    const appUrl=primary.url();
    await save('checkbox470-current-origin.json',{taskId:task.taskId,currentUrl,appUrl,oracleOrigin:site,source:'Existing addressed task URL and independently owned app page URL; not launch intent'});
    assert.equal(new URL(appUrl).protocol,'https:');
    assert.equal(new URL(currentUrl.url).protocol,'https:');assert.equal(new URL(currentUrl.url).origin,new URL(appUrl).origin);
    const capture=await workflow('browser_command',{taskId:task.taskId,action:'native_capture',params:{classifyChallenges:true}});
    await save('checkbox470-observed.json',{capture,application:oracle.state(scene),observeOnly:true,challengeInputDispatched:false});
    assert.equal(capture.challenge.kind,'checkbox');
    assert.equal(oracle.state(scene).inputEvents,0);return;
  }
  const first=await run('hcaptcha-success');
  await run('turnstile-success','turnstile');
  // A fresh frame, changed family and fresh request cannot renew this task's budget.
  {
    const scene='budget-'+randomUUID();oracle.create(scene);await stage(scene,{family:'turnstile'});
    const captureId='native470:'+randomUUID(),attemptId='native470:'+randomUUID();
    const capture=await workflow('browser_command',{taskId:first.task.taskId,action:'native_capture',requestId:captureId,params:{classifyChallenges:true,registeredActionId:'turnstile-checkbox-p169-v1'}});
    assert(capture.nativeObservation,'Fresh family observation must itself qualify');
    let failure;try{await workflow('browser_command',{taskId:first.task.taskId,action:'native_input',requestId:attemptId,params:{nativeAttempt:{observationRequestId:captureId,registeredActionId:'turnstile-checkbox-p169-v1'}}});}catch(error){failure=error;}
    assert(failure);assert.match(failure.message,/browser_native_challenge_attempt_budget_exhausted/);assert.equal(oracle.state(scene).inputEvents,0);
    await save('checkbox470-budget-refusal.json',{refused:true,sourceFamily:'hcaptcha',freshFamily:'turnstile',taskId:first.task.taskId,application:oracle.state(scene)});
  }
  await run('passive','hcaptcha',{passive:true});
  await run('access-denied','hcaptcha',{allowAccess:false});
  await run('image-open','hcaptcha',{image:true});
  // Direct public primitives exercise owner checks independently of client policy.
  for(const condition of ['changed-frame','missing-provider','expired']) {
    const current=await nextTask(condition),scene=condition+'-'+randomUUID();oracle.create(scene);await stage(scene);
    const captureId='native470:'+randomUUID(),attemptId='native470:'+randomUUID();
    const capture=await workflow('browser_command',{taskId:current.taskId,action:'native_capture',requestId:captureId,params:{classifyChallenges:true,registeredActionId:'hcaptcha-checkbox-p181-v2'}});
    assert(capture.nativeObservation,'Direct owner refusal needs a qualified original capture');
    if(condition==='changed-frame')await primary.evaluate(()=>{const marker=document.createElement('div');marker.style.cssText='position:fixed;left:10px;bottom:10px;width:40px;height:40px;background:blue;z-index:1000000';marker.id='changed470';document.body.append(marker);});
    if(condition==='missing-provider'){const denied=structuredClone(config);denied.service.providers[provider.id].capabilities=['human_approval'];await writeFile(configFile,JSON.stringify(denied),{mode:0o600});}
    if(condition==='expired')await new Promise(resolve=>setTimeout(resolve,10050));
    let failure;try{await workflow('browser_command',{taskId:current.taskId,action:'native_input',requestId:attemptId,params:{nativeAttempt:{observationRequestId:captureId,registeredActionId:'hcaptcha-checkbox-p181-v2'}}});}catch(error){failure=error;}
    await writeFile(configFile,JSON.stringify(config),{mode:0o600});
    await primary.evaluate(()=>document.querySelector('#changed470')?.remove());
    assert(failure);
    if(condition==='expired')assert.match(failure.message,/browser_native_original_observation_expired/);
    if(condition==='missing-provider')assert.match(failure.message,/browser_native_site_policy_provider_missing_or_ambiguous/);
    assert.equal(oracle.state(scene).inputEvents,0);
    const original=await workflow('browser_command',{taskId:current.taskId,action:'task_operation',params:{requestId:captureId}});
    await save('checkbox470-'+condition+'.json',{refused:true,taskId:current.taskId,captureOriginal:original,application:oracle.state(scene)});
    receipts.push({label:condition,taskId:current.taskId,inputEvents:0,refused:true});
  }
  await run('uncertain','hcaptcha',{dropReply:true});
  await save('checkbox470-acceptance.json',{passed:true,synthetic:true,actualNativeOwner:true,actualNativeInput:true,
    nativePerceptionDebuggerDependency:false,automaticPolicyOrigin:'Explicit existing verified CDP target + exact task focus; no automatic CDP-free claim',
    independentOracle:'Owned HTTPS application clearance and protected access, distinct from dispatched input',receipts,taskCompleted:false});
  return receipts;
}
