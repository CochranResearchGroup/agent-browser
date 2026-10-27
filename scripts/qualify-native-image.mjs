/** Owned synthetic application and actual Native single-round qualification.
 * CDP stages pictures only. The coordinator selects from actual Native pixels;
 * the oracle's shuffled labels never enter its selection request.
 */
import assert from 'node:assert/strict';
import {createHash,randomInt,randomUUID} from 'node:crypto';
import {readFile,writeFile,lstat} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {createNativeImageChallengeClient} from '../packages/client/src/native-image-challenge.js';
import {waitForNativeCheckboxStability} from './qualify-native-checkbox.mjs';

export function createNativeImageOracle() {
  const scenes=new Map();
  const describe=value=>Object.fromEntries(['id','inputEvents','selectEvents','verifyEvents','cleared','allowAccess','browserAccessRequests','backendAccessRequests'].map(key=>[key,value[key]]));
  return {
    create(id,options={}) {
      assert(!scenes.has(id));const pictures=['bus','bus','bus','bicycle','bicycle','bicycle','boat','boat','boat'];
      for(let index=pictures.length-1;index>0;index--){const other=randomInt(index+1);[pictures[index],pictures[other]]=[pictures[other],pictures[index]];}
      scenes.set(id,{id,pictures,selected:new Set(),inputEvents:0,selectEvents:0,verifyEvents:0,cleared:false,allowAccess:options.allowAccess!==false,browserAccessRequests:0,backendAccessRequests:0});
    },
    render:id=>[...scenes.get(id).pictures],
    state:id=>structuredClone(describe(scenes.get(id))),
    handle(req,res) {
      const match=/^\/image471\/([-a-z0-9]+)\/(select\/([0-8])|verify|clearance|protected)$/.exec(req.url??'');if(!match)return false;
      const value=scenes.get(match[1]);if(!value){res.writeHead(404);res.end();return true;}
      let status=200;
      if(match[2].startsWith('select/')&&req.method==='POST'){value.inputEvents++;value.selectEvents++;value.selected.add(Number(match[3]));}
      else if(match[2]==='verify'&&req.method==='POST'){
        value.inputEvents++;value.verifyEvents++;
        const expected=value.pictures.flatMap((kind,index)=>kind==='bus'?[index]:[]);
        value.cleared=value.selected.size===expected.length&&expected.every(index=>value.selected.has(index));
      }else if(match[2]==='protected'&&req.method==='GET'){
        if(req.headers['x-synthetic-browser']==='1')value.browserAccessRequests++;else value.backendAccessRequests++;
        if(!value.cleared||!value.allowAccess)status=403;
      }else if(match[2]!=='clearance'||req.method!=='GET'){res.writeHead(405);res.end();return true;}
      res.writeHead(status,{'content-type':'application/json','cache-control':'no-store'});res.end(JSON.stringify({...describe(value),report:status===200&&value.cleared&&value.allowAccess?'ready':'unavailable'}));return true;
    },
  };
}

export async function qualifyNativeImage({workflow,task,nextTask,primary,site,profilePath,configFile,faultFile,oracle,save,continuity,selectionDirectory}) {
  assert.equal(new URL(site).protocol,'https:');
  async function prepareWindow(label,current) {
    // Match the owned task's focus preparation before making its original
    // observation. Any later frame change still refuses the original guard.
    await workflow('browser_tab',{taskId:current.taskId,operation:'select',tabId:current.tabId});
    await primary.bringToFront();
    const staging=await primary.context().newCDPSession(primary);
    try{const window=await staging.send('Browser.getWindowForTarget');await staging.send('Browser.setWindowBounds',{windowId:window.windowId,bounds:{windowState:'maximized'}});await save('image471-'+label+'-window-preparation.json',{publicTaskTabSelected:true,taskId:current.taskId,tabId:current.tabId,appOwnedCdp:true,pageBroughtToFront:true,windowId:window.windowId,windowState:'maximized',challengeInputDispatched:false});}finally{await staging.detach();}
  }

  const provider={id:'owned-static-image-provider',kind:'captcha',enabled:true,capabilities:['captcha_solve'],configRef:'agent-browser:remote-view-native-image-static-3x3:v1'};
  const sitePolicy={id:'owned-static-image-site',originPattern:site,origins:[],challengePolicy:'provider_allowed',allowedChallengeProviders:[provider.id]};
  const config={service:{profiles:{'borrowed-profile':{id:'borrowed-profile',name:'Owned image fixture',userDataDir:profilePath,profileOrigin:'external_byop',profileClass:'operator_supplied',persistent:true,sitePolicyIds:[sitePolicy.id]}},sitePolicies:{[sitePolicy.id]:sitePolicy},providers:{[provider.id]:provider}}};
  await writeFile(configFile,JSON.stringify(config),{mode:0o600});
  const receipts=[];
  async function stage(scene,unsupported=false) {
    // Labels are fixture renderer input only. They are not returned to the
    // model-selection port, recorded in the barrier, or exposed as DOM labels.
    await primary.evaluate(({scene,pictures,unsupported})=>{
      document.querySelector('#image471')?.remove();document.querySelector('#checkbox470')?.remove();
      const panel=document.createElement('section');panel.id='image471';panel.style.cssText='position:fixed;inset:0;z-index:999999;background:white;color:black;font:18px Arial,sans-serif';
      const prompt=document.createElement('p');prompt.textContent=unsupported?'Select all images containing tractors':'Select all images containing buses';prompt.style.cssText='position:absolute;left:100px;top:14px;margin:0;font:18px Arial;white-space:nowrap';panel.append(prompt);
      const brand=document.createElement('span');brand.textContent='hCaptcha';brand.style.cssText='position:absolute;left:340px;top:38px;font:18px Arial';panel.append(brand);
      const marker=document.createElement('div');marker.id='image471-marker';marker.style.cssText='position:absolute;right:10px;bottom:10px;width:30px;height:20px;background:#00ff00';panel.append(marker);
      let pending=Promise.resolve();
      pictures.forEach((kind,index)=>{
        const button=document.createElement('button');button.type='button';button.setAttribute('aria-label','Image tile '+(index+1));button.style.cssText='position:absolute;left:'+(100+index%3*66)+'px;top:'+(60+Math.floor(index/3)*66)+'px;width:64px;height:64px;box-sizing:border-box;border:2px solid #141414;border-radius:0;padding:0;margin:0;background:white';
        const canvas=document.createElement('canvas');canvas.width=60;canvas.height=60;canvas.style.cssText='display:block;width:60px;height:60px';button.append(canvas);const c=canvas.getContext('2d');
        c.fillStyle='#eef6ff';c.fillRect(0,0,60,60);c.strokeStyle='#161616';c.lineWidth=2;
        if(kind==='bus'){
          c.fillStyle='#ffc830';c.fillRect(7,15,46,27);c.strokeRect(7,15,46,27);c.fillStyle='#407ca8';for(let n=0;n<4;n++)c.fillRect(11+n*10,19,7,9);c.fillStyle='#161616';for(const x of [17,44]){c.beginPath();c.arc(x,43,6,0,Math.PI*2);c.fill();}c.fillStyle='#ffefad';c.fillRect(8,32,44,3);
        }else if(kind==='bicycle'){
          for(const x of [13,47]){c.beginPath();c.arc(x,40,10,0,Math.PI*2);c.stroke();}c.beginPath();c.moveTo(13,40);c.lineTo(25,23);c.lineTo(36,40);c.lineTo(13,40);c.lineTo(35,29);c.lineTo(47,40);c.lineTo(39,18);c.lineTo(47,18);c.moveTo(20,21);c.lineTo(30,21);c.stroke();
        }else{
          c.fillStyle='#98613a';c.beginPath();c.moveTo(6,36);c.lineTo(54,36);c.lineTo(45,48);c.lineTo(16,48);c.closePath();c.fill();c.stroke();c.beginPath();c.moveTo(30,10);c.lineTo(30,36);c.stroke();c.fillStyle='#fff';c.beginPath();c.moveTo(28,12);c.lineTo(12,32);c.lineTo(28,32);c.closePath();c.fill();c.stroke();c.strokeStyle='#3089ba';for(let n=0;n<3;n++){c.beginPath();c.moveTo(4+n*20,53);c.lineTo(14+n*20,53);c.stroke();}
        }
        button.onclick=()=>{button.style.borderColor='#1478ce';pending=pending.then(async()=>{const response=await fetch('/image471/'+scene+'/select/'+index,{method:'POST'});if(!response.ok)throw Error('Owned selection oracle refused');});};panel.append(button);
      });
      const verify=document.createElement('button');verify.type='button';verify.textContent='Verify';verify.style.cssText='position:absolute;left:100px;top:269px;width:80px;height:30px;box-sizing:border-box;border:2px solid #141414;border-radius:0;padding:0;margin:0;background:white;color:black;font:18px Arial';
      verify.onclick=async()=>{await pending;const response=await fetch('/image471/'+scene+'/verify',{method:'POST'});const state=await response.json();if(!state.cleared)return;const report=await fetch('/image471/'+scene+'/protected',{headers:{'x-synthetic-browser':'1'}});panel.innerHTML=report.ok?'<h1>Report ready</h1>':'<h1>Review pending</h1>';};panel.append(verify);document.body.append(panel);
    },{scene,pictures:oracle.render(scene),unsupported});
    await primary.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  }
  async function policy(context) {
    const bytes=await readFile(configFile),value=JSON.parse(bytes),briefing=await workflow('browser_command',{taskId:context.taskId,action:'task_briefing'});
    return {taskId:context.taskId,binding:Object.fromEntries(['taskId','profileId','browserId','tabId','targetId'].map(key=>[key,briefing[key]])),origin:site,sourceRef:'owned-config-sha256:'+createHash('sha256').update(bytes).digest('hex'),profile:value.service.profiles[briefing.profileId],sitePolicy:value.service.sitePolicies[sitePolicy.id],provider:value.service.providers[provider.id],taskPolicy:await workflow('browser_policy',{taskId:context.taskId,operation:'status'})};
  }
  async function selection(context,label) {
    const nonce=randomUUID(),responseFile='image471-selection-response-'+nonce+'.json';
    const request={schemaVersion:'agent-browser.native-image-selection-request.v1',selectionRequestId:nonce,label,taskId:context.taskId,observationRequestId:context.observationRequestId,frameSha256:context.observation.frameSha256,puzzleSha256:context.observation.puzzleSha256,candidates:context.observation.candidates,pngPath:join(dirname(selectionDirectory),'native-original-ocr-'+context.observation.frameSha256+'.png'),responseFile:join(selectionDirectory,responseFile),deadlineAtMs:Math.min(context.deadlineAtMs-1000,context.observation.observedAtMs+59000),responseMustBeAtomic:true,requiredProvenance:{kind:'agent_visual_inspection',actor:'root',sourceFrameSha256:context.observation.frameSha256}};
    await save('image471-selection-request-'+nonce+'.json',request);await save('image471-selection-request.json',request);
    while(Date.now()<request.deadlineAtMs&&!context.signal.aborted) {
      let info;try{info=await lstat(request.responseFile)}catch(error){if(error.code!=='ENOENT')throw error;}
      if(info){
        assert(info.isFile()&&!info.isSymbolicLink()&&info.uid===process.getuid()&&!(info.mode&0o077)&&info.size<=16384);
        const bytes=await readFile(request.responseFile),answer=JSON.parse(bytes);
        assert.equal(answer.schemaVersion,'agent-browser.native-image-selection-response.v1');assert.equal(answer.selectionRequestId,nonce);
        for(const key of ['taskId','observationRequestId','frameSha256','puzzleSha256'])assert.equal(answer[key],request[key]);
        assert.equal(answer.provenance?.kind,'agent_visual_inspection');assert.equal(answer.provenance?.actor,'root');assert.equal(answer.provenance?.sourceFrameSha256,request.frameSha256);
        await save('image471-selection-receipt-'+nonce+'.json',{selectionRequestId:nonce,frameSha256:request.frameSha256,puzzleSha256:request.puzzleSha256,responseSha256:createHash('sha256').update(bytes).digest('hex'),provenance:answer.provenance,selectedCandidateIds:answer.selectedCandidateIds,source:'Actual coordinator inspection of the exact Native PNG; no oracle labels supplied'});
        return answer;
      }
      await new Promise(resolve=>setTimeout(resolve,100));
    }
    throw Error('native_image_selection_timeout');
  }
  async function run(label,options={}) {
    const current=label==='success'?task:await nextTask(label),scene=label+'-'+randomUUID();oracle.create(scene,options);await stage(scene,options.unsupported);
    await prepareWindow(label,current);
    await waitForNativeCheckboxStability({readCapture:()=>workflow('browser_command',{taskId:current.taskId,action:'native_capture'}),save:value=>save('image471-'+label+'-readiness.json',value)});
    const originalId='image471:'+randomUUID();let inputCalls=0,workflowErrorSequence=0;
    const client=createNativeImageChallengeClient({
      callTool:async(name,args)=>{
        const input=name==='browser_command'&&args.action==='native_input';if(input){inputCalls++;if(options.changedFrame){await primary.evaluate(()=>{document.querySelector('#image471-marker').style.background='#ff0000'});await primary.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));}}
        let value;
        try{value=await workflow(name,args)}catch(error){
          // Preserve the public failure before the client's bounded fallback
          // interprets it. This observer grants no retry or input authority.
          try{
            await save('image471-'+label+'-workflow-error-'+(++workflowErrorSequence)+'.json',{name,action:args.action??null,taskId:current.taskId,requestId:args.requestId??args.params?.requestId??null,code:error?.code??error?.name,message:String(error?.message??error).slice(0,2048),retryPerformed:false});
            if(input)await save('image471-'+label+'-input-error.json',{requestId:originalId,code:error?.code??error?.name,message:String(error?.message??error).slice(0,2048),application:oracle.state(scene),retryPerformed:false});
          }finally{throw error;}
        }
        const content=[{type:'text',text:JSON.stringify({success:true,data:value})}];
        if(name==='browser_command'&&args.action==='native_capture'&&args.params?.classifyChallenges===true){
          const sha=value.capture?.capture?.sha256;assert.match(sha??'',/^[a-f0-9]{64}$/);
          const path=join(dirname(selectionDirectory),'native-original-ocr-'+sha+'.png'),info=await lstat(path);assert(info.isFile()&&!info.isSymbolicLink()&&info.uid===process.getuid()&&!(info.mode&0o077)&&info.size<=4194304);
          const png=await readFile(path);assert.equal(createHash('sha256').update(png).digest('hex'),sha);content.push({type:'image',mimeType:'image/png',data:png.toString('base64')});
        }
        return {content};
      },resolvePolicy:policy,selectCandidates:context=>selection(context,label),
      verifyClearance:async context=>{const response=await fetch(site+'/image471/'+scene+'/clearance'),value=await response.json();return {state:response.ok&&value.cleared?'verified':'unverified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'owned-image-clearance:'+scene};},
      verifyAccess:async context=>{const response=await fetch(site+'/image471/'+scene+'/protected'),value=await response.json();return {state:response.ok&&value.report==='ready'?'verified':'unverified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'owned-image-site-access:'+scene};},
    });
    if(options.lostReply)await writeFile(faultFile,JSON.stringify({state:'armed',kind:'drop_input_reply_once'}),{mode:0o600});
    const result=await client.resolve({taskId:current.taskId,requestId:originalId}),application=oracle.state(scene);
    await save('image471-'+label+'.json',{task:current,originalId,result,application,inputCalls,synthetic:true});
    if(options.changedFrame||options.unsupported){assert.notEqual(result.status,'verified_access_after_native_image');assert.equal(application.inputEvents,0);assert.equal(inputCalls,options.changedFrame?1:0);}
    else if(options.lostReply){assert.notEqual(result.status,'verified_access_after_native_image');assert.equal(application.inputEvents,4);const repeated=await client.resolve({taskId:current.taskId,requestId:originalId});assert.equal(oracle.state(scene).inputEvents,4);assert.equal(repeated.retryAllowed,false);const original=await workflow('browser_command',{taskId:current.taskId,action:'task_operation',params:{requestId:originalId}});assert.equal(original.operation.state,'uncertain');await save('image471-uncertain-original.json',{original,repeated});}
    else if(options.allowAccess===false){assert.notEqual(result.status,'verified_access_after_native_image');assert.equal(application.inputEvents,4);assert.equal(application.cleared,true);assert(application.backendAccessRequests>0);}
    else{assert.equal(result.status,'verified_access_after_native_image',JSON.stringify(result));assert.equal(application.inputEvents,4);assert.equal(application.selectEvents,3);assert.equal(application.verifyEvents,1);assert.equal(application.cleared,true);assert(application.browserAccessRequests>0&&application.backendAccessRequests>0);assert.equal(result.originalOperation.state,'owner_acknowledged');}
    assert.equal(result.retryAllowed,false);await continuity('image471-'+label);receipts.push({label,taskId:current.taskId,originalId,inputCalls,application,status:result.status});
  }
  await run('success');await run('access-denied',{allowAccess:false});await run('changed-frame',{changedFrame:true});await run('uncertain',{lostReply:true});await run('unsupported',{unsupported:true});
  await save('image471-acceptance.json',{passed:true,synthetic:true,actualNativeInput:true,actualNativeOwner:true,nativePerceptionDebuggerDependency:false,selectionSource:'Coordinator reasoning from exact captured Native PNG, digest-bound untrusted IDs; no canned provider success',supportedProfile:'hcaptcha-image-grid-3x3-v1',receipts,taskCompleted:false});
}
