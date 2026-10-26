/** Opt-in extension of the owned Native fixture. Native canvas input is real;
 * the challenge and protected HTTPS application are synthetic and task-local.
 */
import assert from 'node:assert/strict';
import {createChallengeAssistanceClient} from '../packages/client/src/challenge-assistance.js';

export async function qualifyNativeChallengeAssistance({workflow,task,page,primary,nativeOrigin,site,save,consumerControl,alreadyStaged=false,browserPosture}) {
  assert.equal(new URL(site).protocol,'https:','Challenge application must use owned HTTPS');
  if(!alreadyStaged)await primary.evaluate(()=>{
    document.querySelector('#draft').inert=true;document.querySelector('#control').inert=true;
    const overlay=document.createElement('section');overlay.id='challenge469';
    overlay.style.cssText='position:fixed;inset:0;z-index:999999;background:white;color:black;padding:20px;font:24px sans-serif';
    overlay.innerHTML='<div style="position:absolute;right:10px;bottom:10px;background:#00ff00;width:80px;height:40px"></div><h1 style="font-size:26px">Select all images containing buses</h1><p>Owned unsupported image puzzle</p><div style="height:80px;background:#ddd">Synthetic image grid</div><button id="complete469" style="font:22px sans-serif">Complete verification</button>';
    document.body.append(overlay);
    overlay.querySelector('button').onclick=async()=>{
      const response=await fetch('/challenge/complete',{method:'POST'});
      if(!response.ok)return;
      const report=await fetch('/protected-report');
      if(!report.ok)return;
      overlay.innerHTML='<h1 style="font:32px sans-serif">Report ready</h1><p>Protected report accessed</p>';
      document.querySelector('#draft').inert=false;document.querySelector('#control').inert=false;
    };
  });
  const callTool=async(name,args)=>({content:[{type:'text',text:JSON.stringify({success:true,data:await workflow(name,args)})}]});
  const read=async path=>{const response=await fetch(site+path,{redirect:'error'});return {status:response.status,value:await response.json()};};
  const evidence=label=>async context=>{
    const result=await read(label==='clearance'?'/challenge/evidence':'/protected-report');
    const ok=label==='clearance'?result.status===200&&result.value.completionRequests===1:result.status===200&&result.value.report==='ready';
    return {state:ok?'verified':'unverified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'owned-https-'+label};
  };
  const client=createChallengeAssistanceClient({callTool,verifyClearance:evidence('clearance'),verifyAccess:evidence('access')});
  const displayed=page.frames().find(candidate=>candidate.url().startsWith(nativeOrigin+'/'));assert(displayed);
  await displayed.waitForFunction(()=>{
    for(const canvas of document.querySelectorAll('canvas')){
      if(canvas.width<100||canvas.height<100)continue;
      try{const context=canvas.getContext('2d');let markers=0;
        for(let x=10;x<canvas.width;x+=20)for(let y=10;y<canvas.height;y+=20){
          const pixel=context.getImageData(x,y,1,1).data;
          if(pixel[1]>220&&pixel[0]<40&&pixel[2]<40)markers++;
        }
        if(markers>=2)return true;
      }catch{}
    }return false;
  },null,{timeout:15000});
  const capsule=await client.prepare({taskId:task.taskId});
  assert.equal(capsule.classification.kind,'image_puzzle');
  assert.equal(capsule.handoffUrl,task.handoffUrl,'The same opaque task handoff must be reused');
  assert.equal((await consumerControl()).control.owner,'agent','Viewing is not assistance');
  const before=await read('/challenge/evidence');assert.equal(before.value.completionRequests,0);
  const denied=await read('/protected-report');assert.equal(denied.status,403);
  await save('challenge469-before.json',{capsule,protectedStatus:denied.status,application:before.value});
  // The existing Native toolbar coordinates the two existing owners.
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Take control',exact:true}).click();
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Give control back',exact:true}).waitFor();
  const human=await client.recordTakeover(capsule);
  const native=await consumerControl();assert.equal(native.native.owner,'operator');
  const frame=page.frames().find(candidate=>candidate.url().startsWith(nativeOrigin+'/'));assert(frame);
  await frame.waitForFunction(()=>document.querySelector('#connection-status')?.dataset.state==='connected');
  await frame.waitForFunction(()=>['active','released'].includes(document.querySelector('#input-status')?.dataset.state));
  const surface=frame.locator('#desktop-surface');await surface.focus();
  // Only the challenge button is focusable in the owned overlay. These events
  // pass through the actual Native canvas/tunnel, not app DOM/CDP dispatch.
  await page.keyboard.press('Tab');await page.keyboard.press('Enter');
  if(primary)await primary.waitForFunction(()=>document.querySelector('#challenge469 h1')?.textContent==='Report ready');
  else {
    const deadline=Date.now()+15000;let complete=false;
    while(Date.now()<deadline){if((await read('/challenge/evidence')).value.completionRequests===1){complete=true;break;}await new Promise(resolve=>setTimeout(resolve,100));}
    assert(complete,'Independent HTTPS application must observe actual Native input');
  }
  const applied=await read('/challenge/evidence');assert.equal(applied.value.completionRequests,1);
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Give control back',exact:true}).click();
  await page.frameLocator('#desktop-frame').getByRole('button',{name:'Take control',exact:true}).waitFor();
  const returned=await consumerControl();assert.equal(returned.control.owner,'agent');assert.equal(returned.native.owner,'agent');
  const result=await client.continueAfterReturn(human);
  assert.equal(result.status,'verified_access_after_return');
  assert.equal((await read('/challenge/evidence')).value.completionRequests,1);
  const receipt={passed:true,synthetic:true,actualNativeViewer:true,actualNativeInput:true,hiddenDebuggerUsedForPerception:false,
    fixtureAppOwnedCdp:browserPosture??'Explicit staging and independent observation only; not a CDP-free browser-posture claim',
    taskId:task.taskId,handoffId:capsule.handoffId,originalBinding:capsule.binding,
    completionRequests:applied.value.completionRequests,clearance:result.verification.clearance,protectedAccess:result.verification.access,
    sourceFrameSha256:capsule.classification.frameSha256,returnedFrameSha256:result.verification.frameSha256,
    taskCompleted:false};
  await save('challenge469-acceptance.json',receipt);
  return receipt;
}

export async function probeCurrentNativeObservationAge({workflow,task,consumerControl,save}) {
    // Characterize the CURRENT public Native seam, not the archived desktop
    // transaction. Never send button-down or a challenge-solving click here.
    const source=await workflow('browser_command',{taskId:task.taskId,action:'native_capture',params:{classifyChallenges:true}});
    const control=await consumerControl();assert.equal(control.native.owner,'agent');
    const start=Date.now(),replies=[];
    for(let index=0;index<9;index++) {
      await new Promise(resolve=>setTimeout(resolve,120));
      replies.push(await workflow('browser_command',{taskId:task.taskId,action:'native_input',params:{controlEpoch:control.native.epoch,
        actions:[{kind:'move',x:source.windowBounds.x+5+index,y:source.windowBounds.y+5}]}}));
    }
    const after=await workflow('browser_command',{taskId:task.taskId,action:'native_capture',params:{classifyChallenges:true}});
    assert(replies.every(reply=>reply.delivery==='dispatched'),'Every motion requires the actual owner dispatch acknowledgement');
    assert(Date.now()-start>750,'Probe must cross the historical age boundary');
    assert.equal(after.capture.windowId,source.capture.windowId);
    assert.equal(after.capture.generation,source.capture.generation);
    assert.deepEqual(after.capture.processIdentity,source.capture.processIdentity);
    await save('challenge470-current-observation-age.json',{currentPublicNativeSeam:true,sourceObservedAtMs:source.observedAtMs,
      elapsedMs:Date.now()-start,motionRequests:9,buttonEvents:0,replies,windowPreserved:true,generationPreserved:true,
      legacyDefectReproduced:false,limitation:'Current native_input reobserves its window per request; it does not consume the historical desktop transaction observation or its 750ms budget. No click attempted and no timing constant changed.'});
}
