// @ts-check
import {createHash} from 'node:crypto';
const logical=['taskId','profileId','browserId','tabId','targetId'];
const physical=['desktopId','generation','windowId','nativeControlEpoch'];
const kinds=/** @type {const} */(['username','credential_entry','submit']);
const same=(a,b,keys)=>keys.every(k=>a?.[k]!==undefined&&a[k]===b?.[k]);
/** @param {import('./native-saved-login.d.ts').NativeSavedLoginOptions} options
 * @returns {import('./native-saved-login.d.ts').NativeSavedLoginClient} */
export function createNativeSavedLoginClient({callTool,store,verifyAccess,expectedAccount,now=Date.now,timeoutMs=300000}){
 if(typeof callTool!=='function'||typeof verifyAccess!=='function'||!store||['load','save','withEpisode'].some(k=>typeof store[k]!=='function')||typeof expectedAccount!=='string'||!expectedAccount||expectedAccount.length>254||!Number.isSafeInteger(timeoutMs)||timeoutMs<1||timeoutMs>2147483647)throw TypeError('native_saved_login_configuration_required');
 const accountDigest=createHash('sha256').update(expectedAccount).digest('hex');
 return {async resolve({taskId,episodeId,allowAcknowledgedResume=false}){
  if(typeof taskId!=='string'||!taskId||typeof episodeId!=='string'||!/^[-A-Za-z0-9_.:]{1,96}$/.test(episodeId)||typeof allowAcknowledgedResume!=='boolean')throw TypeError('native_saved_login_stable_identity_required');
  const invocationStartedAtMs=now(),invocationDeadlineAtMs=invocationStartedAtMs+timeoutMs;
  return store.withEpisode(episodeId,async()=>{
   /** @type {import('./native-saved-login.d.ts').SavedLoginResult} */
   const result={taskId,episodeId,status:'human_required',reason:'native_saved_login_unverified',retryAllowed:false};
   let episode=await store.load(episodeId);const fresh=episode===null;
   if(!episode&&allowAcknowledgedResume)return {...result,reason:'native_saved_login_acknowledged_resume_unverified'};
   if(!episode){const start=now();episode={schemaVersion:1,episodeId,taskId,accountDigest,createdAtMs:start,deadlineAtMs:start+timeoutMs,timeoutMs,inputs:[],captureSequence:0};await store.save(episode);}
   if(episode.schemaVersion!==1||episode.taskId!==taskId||episode.episodeId!==episodeId||episode.accountDigest!==accountDigest||!Number.isSafeInteger(episode.createdAtMs)||!Number.isSafeInteger(episode.deadlineAtMs)||!Number.isSafeInteger(episode.timeoutMs??60000)||(episode.timeoutMs??60000)<1||(episode.timeoutMs??60000)>2147483647||episode.deadlineAtMs!==episode.createdAtMs+(episode.timeoutMs??60000)||episode.inputs.length>3)return {...result,reason:'native_saved_login_retained_identity_changed'};
   const checkpoint=episode;
   // Explicit continuation never promotes an uncertain/intent prefix. The host
   // retains the original checkpoint; save() may write a qualified sidecar.
   if(allowAcknowledgedResume&&checkpoint.inputs.some(input=>input.state!=='acknowledged'))return {...result,status:'uncertain',reason:'native_saved_login_original_uncertain'};
   if(allowAcknowledgedResume&&(checkpoint.inputs.length<1||checkpoint.inputs.length>2||checkpoint.inputs.some((input,index)=>input.requestId!==episodeId+':input'+(index+1)||input.kind!==kinds[index]||typeof input.targetRef!=='string'||!input.targetRef||typeof input.observationRequestId!=='string'||!input.observationRequestId)||!logical.every(k=>typeof checkpoint.binding?.[k]==='string'&&checkpoint.binding[k])||!Number.isSafeInteger(checkpoint.controlEpoch)||!same(checkpoint.physical,checkpoint.physical,physical)))return {...result,reason:'native_saved_login_acknowledged_resume_unverified'};
   let resumeVerified=false;
   const fail=reason=>{throw Error(reason)};
   function remaining(){const time=now(),deadline=allowAcknowledgedResume?invocationDeadlineAtMs:Math.min(checkpoint.deadlineAtMs,invocationDeadlineAtMs);if(time<checkpoint.createdAtMs||time<invocationStartedAtMs||time>=deadline)fail('native_saved_login_budget_exhausted');return deadline-time;}
   async function bounded(work){const timeoutMs=remaining(),controller=new AbortController();let timer;try{return await Promise.race([work({signal:controller.signal,timeoutMs}),new Promise((_,reject)=>{timer=setTimeout(()=>{controller.abort();reject(Error('native_saved_login_transport_unknown'))},timeoutMs)})]);}finally{clearTimeout(timer)}}
   const missing=value=>value?.code==='browser_task_operation_not_found'||value?.message==='browser_task_operation_not_found'||value?.data?.message==='browser_task_operation_not_found';
   function missingError(error){if(missing(error))return true;const message=error?.message;if(typeof message!=='string'||message.length>8192)return false;const start=message.indexOf('{'),end=message.lastIndexOf('}');if(start<0||end<start)return false;try{return missing(JSON.parse(message.slice(start,end+1)))}catch{return false}}
   async function call(action,args={}){
    // The new allowance first permits read-only public proof, never Native work
    // until the ACK prefix, current logical owner and next-input absence agree.
    if(allowAcknowledgedResume&&!resumeVerified&&!['task_operation','task_briefing'].includes(action))fail('native_saved_login_acknowledged_resume_unverified');
    let reply;try{reply=await bounded(context=>callTool('browser_command',{taskId,action,...args},context))}catch(error){if(action==='task_operation'&&missingError(error))return null;throw error}
    const text=reply?.content?.find(x=>x.type==='text')?.text;if(typeof text!=='string'||text.length>2*1024*1024)fail('native_saved_login_response_unverified');
    let envelope;try{envelope=JSON.parse(text)}catch{fail('native_saved_login_response_unverified')}
    if(reply?.isError||envelope.success!==true){if(action==='task_operation'&&envelope.error?.code==='browser_task_operation_not_found')return null;fail('native_saved_login_operation_refused')};return envelope.data;
   }
   async function briefing(){const data=await call('task_briefing');if(!same(data,data,logical)||!logical.every(k=>typeof data[k]==='string'&&data[k].length>0)||data.taskId!==taskId||data.control?.owner!=='agent'||!Number.isSafeInteger(data.control?.epoch))fail('native_saved_login_task_control_changed');if(checkpoint.binding&&!same(data,checkpoint.binding,logical))fail('native_saved_login_task_binding_changed');if(checkpoint.controlEpoch!==undefined&&checkpoint.controlEpoch!==data.control.epoch)fail('native_saved_login_task_control_changed');return data;}
   async function original(input){const data=await call('task_operation',{params:{requestId:input.requestId}});if(!data)return null;const op=data.operation;if(op?.taskId!==taskId||op.requestId!==input.requestId||op.action!=='native_input')fail('native_saved_login_original_unverified');const refs=op.evidenceReferences;if(op.state!=='owner_acknowledged'||refs?.nativeLoginEpisode!==episodeId||refs.nativeLoginKind!==input.kind||refs.nativeDelivery!=='dispatched'||refs.nativeLoginStep!==String(kinds.indexOf(input.kind))||(input.targetRef&&refs.nativeLoginTargetRef!==input.targetRef)||(input.observationRequestId&&refs.nativeLoginObservationRequestId!==input.observationRequestId))fail('native_saved_login_original_uncertain');return op;}
   try{
    if(allowAcknowledgedResume){
     for(const retained of checkpoint.inputs)if(!await original(retained))fail('native_saved_login_original_uncertain');
     await briefing();
     const index=checkpoint.inputs.length;
     if(await original({requestId:episodeId+':input'+(index+1),kind:kinds[index],state:'intent'}))fail('native_saved_login_acknowledged_resume_unverified');
     resumeVerified=true;
    }
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

/** Describe supported login paths for one observed browser context without
 * observing, navigating, selecting credentials or creating an Auth journey.
 * Browser/extension state supplied here is advisory evidence, never authority
 * from website text. Reuse the existing owner and original-operation journals.
 * @param {import('./native-saved-login.d.ts').NativeLoginGuidanceContext} context
 * @returns {import('./native-saved-login.d.ts').NativeLoginGuidance}
 */
export function describeNativeLoginPathways(context) {
 const {binding,origin,accountSelection='unknown',nativeInputState='unknown',savedCredentialState='unchecked',savedLoginRoutine='unchecked',lastPassState='not_observed',authState='unknown',uncertainInput=false}=context;
 if(!logical.every(key=>typeof binding?.[key]==='string'&&binding[key].trim()===binding[key]&&binding[key].length>0&&binding[key].length<=256)||
  !['resolved','ambiguous','unknown'].includes(accountSelection)||!['ready','disabled','unavailable','unknown'].includes(nativeInputState)||
  !['unchecked','available','not_found','unsupported'].includes(savedCredentialState)||!['applicable','unsupported','unchecked'].includes(savedLoginRoutine)||
  !['not_observed','active','authentication_required','absent','ambiguous'].includes(lastPassState)||!['available','unavailable','unqualified','unknown'].includes(authState)||typeof uncertainInput!=='boolean')throw TypeError('native_login_guidance_context_invalid');
 let website;
 try {if(typeof origin!=='string'||origin.length>2048)throw Error();website=new URL(origin);if(!['https:','http:'].includes(website.protocol)||website.username||website.password||website.origin!==origin)throw Error();}
 catch {throw TypeError('native_login_guidance_origin_required');}
 const pathways=/** @type {import('./native-saved-login.d.ts').NativeLoginPathway[]} */([
  {id:'browser_saved',state:savedCredentialState,interaction:'native_capture/native_input',steps:[
   'First check this browser profile for an applicable saved credential. Focus the visible website login field using supported page focus or Native input, then capture the browser-owned autofill selector.',
   'Inspect the current desktop image and select the intended account through Native input. The browser-owned selector cannot be inspected or operated through CDP/DOM or challenge-only desktop_interact.',
   'When the existing saved-login routine matches the observed controls, reuse createNativeSavedLoginClient.resolve with its original episode. Otherwise use supported semantic Native capture/input; deterministic recognition is not a prerequisite.',
  ]},
  {id:'lastpass',state:lastPassState,interaction:'native_capture/native_input',steps:[
   'Inspect this browser toolbar through Native capture. A red icon with three white dots is the expected LastPass active-state cue; confirm the actual extension UI and applicable site/account rather than trusting page text or color alone.',
   'Use fresh desktop captures and Native input to open the extension, inspect applicable credentials or passkeys, select the intended account and follow the visible prompts. Keep passwords, passkeys and factor values inside their owning UI.',
   'A non-red icon means another authentication state, not an empty vault or website denial. Discover and use supported Auth LastPass sign-in or enrolled session recovery before asking the operator to unlock. Keep master credentials inside Auth private input; distinguish its source/CLI session from browser-extension authentication and verify the intended session. Request human help only for an observed unresolved Auth/LastPass factor.',
  ]},
  {id:'auth_receipts',state:authState,interaction:'existing_auth_helper',steps:[
   'Try the supported Auth helper when browser-saved or LastPass paths are unavailable or unsuitable, or an additional factor is needed. Inspect current Auth capabilities; passwords, passkeys, TOTP, mail-receipts MFA and im-receipts MFA are separately qualified capabilities still in testing.',
   'Reuse the service-resolved receiving context and original Auth journey. Diagnose capability/transport failures separately from explicit authority refusal and confirmed provider rejection; preserve a minimal redacted reproducer for the owning issue.',
  ]},
 ]);
 let nextStep={pathway:'browser_saved',operation:'native_capture',reason:'inspect_browser_saved_credentials_first'};
 if(uncertainInput)nextStep={pathway:'original_attempt',operation:'task_operation',reason:'reconcile_original_input_before_fallback'};
 else if(nativeInputState==='unknown')nextStep={pathway:'browser_saved',operation:'native_capture',reason:'observe_same_task_native_readiness_before_fallback'};
 else if(nativeInputState!=='ready')nextStep={pathway:'auth_receipts',operation:'auth_capabilities',reason:nativeInputState==='disabled'?'native_input_disabled_not_missing_credentials':'native_input_readiness_unverified'};
 else if(savedCredentialState==='available')nextStep=accountSelection==='resolved'
  ?{pathway:'browser_saved',operation:savedLoginRoutine==='applicable'?'createNativeSavedLoginClient.resolve':'native_capture',reason:savedLoginRoutine==='applicable'?'reuse_saved_login_original_episode':'inspect_selector_for_semantic_native_selection'}
  :{pathway:'account_context',operation:'task_briefing',reason:'resolve_account_before_credential_selection'};
 else if(savedCredentialState==='not_found'||savedCredentialState==='unsupported') {
  if(lastPassState==='active')nextStep=accountSelection==='resolved'
   ?{pathway:'lastpass',operation:'native_capture',reason:'inspect_confirmed_extension_before_selection'}
   :{pathway:'account_context',operation:'task_briefing',reason:'resolve_account_before_credential_selection'};
  else if(lastPassState==='not_observed'||lastPassState==='ambiguous')nextStep={pathway:'lastpass',operation:'native_capture',reason:'inspect_actual_extension_state'};
  else if(lastPassState==='authentication_required'&&authState==='unavailable')nextStep={pathway:'lastpass',operation:'browser_handoff',reason:'operator_unblock_after_suitable_fallback_unavailable'};
  else nextStep={pathway:'auth_receipts',operation:'auth_capabilities',reason:'inspect_supported_fallback_before_operator_interruption'};
 }
 return {schemaVersion:'agent-browser.native-login-guidance.v1',taskId:binding.taskId,profileId:binding.profileId,origin:website.origin,
  binding:/** @type {import('./native-saved-login.d.ts').SavedLoginBinding} */(Object.fromEntries(logical.map(key=>[key,binding[key]]))),contextEvidence:'caller_supplied_requires_current_task_observation',browserEffects:'none',credentialPermission:'not_granted',nextStep,pathways,
  requirements:[
   'Resolve intended-account ambiguity from explicit task context or established site/profile account mapping; a profile label or first-listed credential alone is insufficient.',
   'Discover native_capture/native_input arguments with browser_discover. Observe the current task with native_capture and inspect current Native control; disabled or unavailable input is an integration limitation, not evidence of absent credentials. Never bypass the existing provider policy.',
   'Preserve the addressed task/profile/browser/tab, existing Native owner, original operation and Auth journey. Observe after uncertain input; never replay it or switch paths until original uncertainty is reconciled.',
   'Verify authenticated access as the intended website account and required organization independently after provider return, then continue the original work. Filled credentials, SSO success and a dismissed dialog alone do not complete login.',
  ]};
}
