// @ts-check
import {createHash} from 'node:crypto';
const logical=['taskId','profileId','browserId','tabId','targetId'];
const physical=['desktopId','generation','windowId','nativeControlEpoch'];
const kinds=/** @type {const} */(['username','credential_entry','submit']);
const same=(a,b,keys)=>keys.every(k=>a?.[k]!==undefined&&a[k]===b?.[k]);
/** @param {import('./native-saved-login.d.ts').NativeSavedLoginOptions} options
 * @returns {import('./native-saved-login.d.ts').NativeSavedLoginClient} */
export function createNativeSavedLoginClient({callTool,store,verifyAccess,expectedAccount,now=Date.now}){
 if(typeof callTool!=='function'||typeof verifyAccess!=='function'||!store||['load','save','withEpisode'].some(k=>typeof store[k]!=='function')||typeof expectedAccount!=='string'||!expectedAccount||expectedAccount.length>254)throw TypeError('native_saved_login_configuration_required');
 const accountDigest=createHash('sha256').update(expectedAccount).digest('hex');
 return {async resolve({taskId,episodeId}){
  if(typeof taskId!=='string'||!taskId||typeof episodeId!=='string'||!/^[-A-Za-z0-9_.:]{1,96}$/.test(episodeId))throw TypeError('native_saved_login_stable_identity_required');
  return store.withEpisode(episodeId,async()=>{
   /** @type {import('./native-saved-login.d.ts').SavedLoginResult} */
   const result={taskId,episodeId,status:'human_required',reason:'native_saved_login_unverified',retryAllowed:false};
   let episode=await store.load(episodeId);const fresh=episode===null;
   if(!episode){const start=now();episode={schemaVersion:1,episodeId,taskId,accountDigest,createdAtMs:start,deadlineAtMs:start+60000,inputs:[],captureSequence:0};await store.save(episode);}
   if(episode.schemaVersion!==1||episode.taskId!==taskId||episode.episodeId!==episodeId||episode.accountDigest!==accountDigest||episode.deadlineAtMs!==episode.createdAtMs+60000||episode.inputs.length>3)return {...result,reason:'native_saved_login_retained_identity_changed'};
   const checkpoint=episode;
   const fail=reason=>{throw Error(reason)};
   function remaining(){const time=now();if(time<checkpoint.createdAtMs||time>=checkpoint.deadlineAtMs)fail('native_saved_login_budget_exhausted');return checkpoint.deadlineAtMs-time;}
   async function bounded(work){const timeoutMs=remaining(),controller=new AbortController();let timer;try{return await Promise.race([work({signal:controller.signal,timeoutMs}),new Promise((_,reject)=>{timer=setTimeout(()=>{controller.abort();reject(Error('native_saved_login_transport_unknown'))},timeoutMs)})]);}finally{clearTimeout(timer)}}
   const missing=value=>value?.code==='browser_task_operation_not_found'||value?.message==='browser_task_operation_not_found'||value?.data?.message==='browser_task_operation_not_found';
   function missingError(error){if(missing(error))return true;const message=error?.message;if(typeof message!=='string'||message.length>8192)return false;const start=message.indexOf('{'),end=message.lastIndexOf('}');if(start<0||end<start)return false;try{return missing(JSON.parse(message.slice(start,end+1)))}catch{return false}}
   async function call(action,args={}){
    let reply;try{reply=await bounded(context=>callTool('browser_command',{taskId,action,...args},context))}catch(error){if(action==='task_operation'&&missingError(error))return null;throw error}
    const text=reply?.content?.find(x=>x.type==='text')?.text;if(typeof text!=='string'||text.length>2*1024*1024)fail('native_saved_login_response_unverified');
    let envelope;try{envelope=JSON.parse(text)}catch{fail('native_saved_login_response_unverified')}
    if(reply?.isError||envelope.success!==true){if(action==='task_operation'&&envelope.error?.code==='browser_task_operation_not_found')return null;fail('native_saved_login_operation_refused')};return envelope.data;
   }
   async function briefing(){const data=await call('task_briefing');if(!same(data,data,logical)||!logical.every(k=>typeof data[k]==='string'&&data[k].length>0)||data.taskId!==taskId||data.control?.owner!=='agent'||!Number.isSafeInteger(data.control?.epoch))fail('native_saved_login_task_control_changed');if(checkpoint.binding&&!same(data,checkpoint.binding,logical))fail('native_saved_login_task_binding_changed');if(checkpoint.controlEpoch!==undefined&&checkpoint.controlEpoch!==data.control.epoch)fail('native_saved_login_task_control_changed');return data;}
   async function original(input){const data=await call('task_operation',{params:{requestId:input.requestId}});if(!data)return null;const op=data.operation;if(op?.taskId!==taskId||op.requestId!==input.requestId||op.action!=='native_input')fail('native_saved_login_original_unverified');const refs=op.evidenceReferences;if(op.state!=='owner_acknowledged'||refs?.nativeLoginEpisode!==episodeId||refs.nativeLoginKind!==input.kind||refs.nativeDelivery!=='dispatched'||refs.nativeLoginStep!==String(kinds.indexOf(input.kind))||(input.targetRef&&refs.nativeLoginTargetRef!==input.targetRef)||(input.observationRequestId&&refs.nativeLoginObservationRequestId!==input.observationRequestId))fail('native_saved_login_original_uncertain');return op;}
   try{
    for(let index=0;index<3;index++){
     const kind=kinds[index],requestId=episodeId+':input'+(index+1),retained=checkpoint.inputs[index];
     if(retained){if(retained.requestId!==requestId||retained.kind!==kind)fail('native_saved_login_retained_identity_changed');if(!await original(retained))fail('native_saved_login_original_uncertain');retained.state='acknowledged';await store.save(checkpoint);continue;}
     /** @type {import('./native-saved-login.d.ts').SavedLoginInput} */
     const proposed={requestId,kind,state:'intent'};
     if(await original(proposed))fail(fresh?'native_saved_login_retained_episode_required':'native_saved_login_original_uncertain');
     const before=await briefing();if(!checkpoint.binding){checkpoint.binding=/** @type {import('./native-saved-login.d.ts').SavedLoginBinding} */(Object.fromEntries(logical.map(k=>[k,before[k]])));checkpoint.controlEpoch=before.control.epoch;await store.save(checkpoint);}
     // Cover the complete window using overlapping owner-sized tiles. Each
     // input still binds one freshly recaptured tile; frames are never stitched.
     async function capture(region){
      checkpoint.captureSequence=(checkpoint.captureSequence??0)+1;await store.save(checkpoint);
      const observationRequestId=episodeId+':observation'+(index+1)+':'+checkpoint.captureSequence;
      const frame=await call('native_capture',{requestId:observationRequestId,params:{loginControls:{profile:'english-chrome-v1',expectedAccount},...(region?{region}:{})}});
      const controls=frame?.loginControls,binding=frame?.binding;
      if(frame?.source!=='remote_view_native'||controls?.schemaVersion!=='agent-browser.native-login-controls.v1'||!['observed','not_found'].includes(controls.status)||!Array.isArray(controls.controls)||!same(binding,binding,physical)||typeof binding.desktopId!=='string'||!binding.desktopId||!['generation','windowId','nativeControlEpoch'].every(k=>Number.isSafeInteger(binding[k])&&binding[k]>=0)||!/^[a-f0-9]{64}$/.test(controls.frameSha256??'')||!Number.isSafeInteger(binding.observedAtMs)||now()-binding.observedAtMs>10000||binding.observedAtMs>now())fail('native_saved_login_frame_unverified');
      if(checkpoint.physical&&!same(binding,checkpoint.physical,physical))fail('native_saved_login_native_binding_changed');
      if(!checkpoint.physical){checkpoint.physical=Object.fromEntries(physical.map(k=>[k,binding[k]]));await store.save(checkpoint);}
      return {controls,binding,observationRequestId};
     }
     const initial=await capture();
     const window=initial.binding.window;
     const validBounds=b=>b&&['x','y','width','height'].every(k=>Number.isSafeInteger(b[k]))&&b.x>=0&&b.y>=0&&b.width>0&&b.height>0;
     const regions=[];
     {
      if(!validBounds(window)||window.x+window.width>initial.binding.displayWidth||window.y+window.height>initial.binding.displayHeight)fail('native_saved_login_frame_unverified');
      if(window.width*window.height>1000000){
       const height=Math.min(window.height,1000000),width=Math.min(window.width,Math.floor(1000000/height));
       if(Math.ceil(window.width/width)*Math.ceil(window.height/height)>16)fail('native_saved_login_surface_unsupported');
       for(let y=0;y<window.height;y+=height)for(let x=0;x<window.width;x+=width)regions.push({x:window.x+Math.min(x,window.width-width),y:window.y+Math.min(y,window.height-height),width,height});
      }
     }
     const frames=regions.length?[]:[initial];
     for(const region of regions){const frame=await capture(region);if(!same(frame.binding.region,region,['x','y','width','height'])||!same(frame.binding.window,window,['x','y','width','height'])||!same(frame.binding,initial.binding,['displayWidth','displayHeight']))fail('native_saved_login_native_binding_changed');frames.push(frame);}
     const targetKey=c=>JSON.stringify([c.kind,c.bounds,c.configuredAccountMatch,c.fieldValueState]);
     const choices=new Map();
     for(const frame of frames){
      const controls=frame.controls;
      if(kind==='submit'){
       const user=controls.controls.filter(c=>c.kind==='username'),password=controls.controls.filter(c=>c.kind==='password');
       if(user.length!==1||user[0].configuredAccountMatch!==true||password.length!==1||password[0].fieldValueState!=='masked')continue;
      }
      for(const candidate of controls.controls.filter(c=>c.kind===kind&&(kind!=='credential_entry'||c.configuredAccountMatch===true))){
       if(typeof candidate.targetRef!=='string'||!candidate.targetRef)fail('native_saved_login_target_ambiguous');
       const key=regions.length?targetKey(candidate):candidate.targetRef;
       if(!choices.has(key))choices.set(key,{candidate,frame});
      }
     }
     if(choices.size!==1)fail(kind==='submit'?'native_saved_login_filled_account_unverified':'native_saved_login_target_ambiguous');
     let {candidate,frame}=choices.values().next().value;
     if(regions.length){
      const fresh=await capture(frame.binding.region);
      if(!same(fresh.binding.window,window,['x','y','width','height'])||!same(fresh.binding,initial.binding,['displayWidth','displayHeight'])||!same(fresh.binding.region,frame.binding.region,['x','y','width','height']))fail('native_saved_login_native_binding_changed');
      const eligible=fresh.controls.controls.filter(c=>c.kind===kind&&(kind!=='credential_entry'||c.configuredAccountMatch===true));
      const matches=eligible.filter(c=>targetKey(c)===targetKey(candidate));
      if(eligible.length!==1||matches.length!==1)fail('native_saved_login_target_ambiguous');
      if(kind==='submit'){
       const user=fresh.controls.controls.filter(c=>c.kind==='username'),password=fresh.controls.controls.filter(c=>c.kind==='password');
       if(user.length!==1||user[0].configuredAccountMatch!==true||password.length!==1||password[0].fieldValueState!=='masked')fail('native_saved_login_filled_account_unverified');
      }
      candidate=matches[0];frame=fresh;
     }
     const candidates=[candidate],observationRequestId=frame.observationRequestId;
     const after=await briefing();if(!same(before,after,logical)||before.control.epoch!==after.control.epoch)fail('native_saved_login_task_control_changed');remaining();
     proposed.targetRef=candidates[0].targetRef;proposed.observationRequestId=observationRequestId;checkpoint.inputs.push(proposed);await store.save(checkpoint);
     try{
      const input=await call('native_input',{requestId,params:{loginTarget:{observationRequestId,targetRef:candidates[0].targetRef,episodeId}}});
      if(input?.source!=='remote_view_native'||input.delivery!=='dispatched')fail('native_saved_login_input_unknown');
      if(!await original(proposed))fail('native_saved_login_original_uncertain');proposed.state='acknowledged';await store.save(checkpoint);
     }catch{proposed.state='unknown';await store.save(checkpoint);return {...result,status:'uncertain',reason:'native_saved_login_input_requires_reconciliation'};}
    }
    const after=await briefing();
    const proof=await bounded(({signal})=>verifyAccess({taskId,episodeId,binding:checkpoint.binding,expectedAccount,signal}));
    if(proof?.state!=='verified'||proof.accountMatched!==true||!same(proof,after,logical)||typeof proof.evidenceRef!=='string'||!proof.evidenceRef||proof.evidenceRef.length>256)fail('native_saved_login_access_unverified');
    return {...result,status:'verified_access_after_native_saved_login',reason:'independent_account_access_verified',evidenceRef:proof.evidenceRef};
   }catch(error){const reason=error instanceof Error&&/^native_saved_login_[a-z_]+$/.test(error.message)?error.message:'native_saved_login_observation_unavailable';return {...result,status:reason.includes('uncertain')||checkpoint.inputs.some(input=>input.state!=='acknowledged')?'uncertain':'human_required',reason};}
  });
 }};
}
