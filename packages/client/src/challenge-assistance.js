import {isDeepStrictEqual} from 'node:util';
/** Compose existing task, Native capture/handoff and cooperative control APIs.
 * Site-specific verification ports must independently observe the application.
 * Optional automated input requires trusted receiving-app policy records and an
 * owner-minted registered observation. No viewer or credential authority is added.
 */
export function createChallengeAssistanceClient({callTool,verifyClearance,verifyAccess,resolvePolicy,now=Date.now,wait=ms=>new Promise(resolve=>setTimeout(resolve,ms))}) {
  if([callTool,verifyClearance,verifyAccess].some(port=>typeof port!=='function'))throw new TypeError('challenge_assistance_ports_required');
  const fields=['taskId','profileId','browserId','tabId','targetId'];
  const fail=code=>{throw new Error(code);};
  const identity=value=>Object.fromEntries(fields.map(key=>[key,value[key]]));
  const same=(left,right)=>fields.every(key=>typeof left[key]==='string'&&left[key]===right[key]);
  const physical=value=>({desktopId:value.capture.desktopId,generation:value.capture.generation,
    windowId:value.capture.windowId,processIdentity:value.capture.processIdentity});
  const samePhysical=(left,right)=>isDeepStrictEqual(left,right);
  async function call(name,args) {
    const reply=await callTool(name,args);
    const text=reply?.content?.find(block=>block.type==='text')?.text;
    if(reply?.isError||typeof text!=='string'||text.length>8*1024*1024)fail('challenge_workflow_response_invalid');
    let envelope;
    try{envelope=JSON.parse(text);}catch{fail('challenge_workflow_response_invalid');}
    if(envelope.success!==true)fail('challenge_workflow_refused_observe_before_retry');
    return envelope.data;
  }
  const briefing=taskId=>call('browser_command',{taskId,action:'task_briefing'});
  async function observe(taskId,expected,captureOptions={}) {
    if(typeof taskId!=='string'||!taskId.trim())fail('challenge_task_required');
    const before=await briefing(taskId);
    if(before.taskId!==taskId||!same(before,before)||expected&&!same(before,expected))fail('challenge_task_identity_changed');
    const frame=await call('browser_command',{taskId,action:'native_capture',...captureOptions,params:{classifyChallenges:true,...captureOptions.params}});
    const after=await briefing(taskId);
    if(!same(before,after)||before.control?.epoch!==after.control?.epoch||before.control?.owner!==after.control?.owner)fail('challenge_context_changed');
    const c=frame.capture,sha=c?.capture?.sha256;
    const declaredVisual=frame.challenge?.corpusVersion==='native-challenge-corpus.v2'&&frame.challenge.kind==='checkbox'&&
      frame.challenge.family==='hcaptcha'&&frame.challenge.registeredProfileId==='hcaptcha-checkbox-p181-v2'&&
      frame.challenge.completeWindow===true&&frame.challenge.minimumOcrConfidence===70&&frame.challenge.automatedAttemptAuthorized===false&&
      Array.isArray(frame.challenge.ruleIds)&&frame.challenge.ruleIds.length===1&&frame.challenge.ruleIds[0]==='registered_hcaptcha_joined_prompt_visual_v1';
    if(frame.source!=='remote_view_native'||!c||typeof c.desktopId!=='string'||!Number.isSafeInteger(c.generation)||
      !Number.isSafeInteger(c.windowId)||!c.processIdentity||!/^([a-f0-9]{64})$/.test(sha??'')||
      frame.challenge?.frameSha256!==sha||frame.challenge?.corpusVersion!=='native-challenge-corpus.v1'&&!declaredVisual||
      !Number.isFinite(frame.evidenceAgeMs)||frame.evidenceAgeMs<0||frame.evidenceAgeMs>10000)fail('challenge_native_evidence_unverified');
    return {binding:identity(after),physical:physical(frame),classification:frame.challenge,
      frameSha256:sha,observedAtMs:frame.observedAtMs??null,control:after.control,
      nativeControlEpoch:frame.nativeControlEpoch??frame.nativeObservation?.nativeControlEpoch,nativeObservation:frame.nativeObservation};
  }
  function intervention(capsule,reason){return {...structuredClone(capsule),status:'intervention',reason};}
  function verified(proof,taskId,frameSha256) {
    return proof?.state==='verified'&&proof.taskId===taskId&&proof.frameSha256===frameSha256&&
      typeof proof.evidenceRef==='string'&&proof.evidenceRef.length>0&&proof.evidenceRef.length<=256;
  }
  async function original(taskId,requestId) {
    const missing=value=>value?.code==='browser_task_operation_not_found'||value?.message==='browser_task_operation_not_found'||value?.data?.message==='browser_task_operation_not_found';
    let reply;
    try { reply=await callTool('browser_command',{taskId,action:'task_operation',params:{requestId}}); }
    catch(error) {
      if(missing(error))return null;
      // Stdio clients can include the JSON-RPC error envelope in Error.message;
      // SDK clients retain data.message. Neither a generic refusal nor an
      // unavailable read establishes that no original operation exists.
      const message=error?.message;
      if(typeof message==='string'&&message.length<=8192){
        const start=message.indexOf('{');
        if(start>=0)try{if(missing(JSON.parse(message.slice(start))))return null;}catch{}
      }
      throw error;
    }
    const text=reply?.content?.find(block=>block.type==='text')?.text;
    if(typeof text!=='string'||text.length>8*1024*1024)fail('challenge_original_operation_unverified');
    let envelope;
    try{envelope=JSON.parse(text);}catch{fail('challenge_original_operation_unverified');}
    if(reply?.isError||envelope.success!==true) {
      if(missing(envelope.error))return null;
      fail('challenge_original_operation_unverified');
    }
    const operation=envelope.data?.operation;
    if(operation?.taskId!==taskId||operation.requestId!==requestId||operation.action!=='native_input')fail('challenge_original_operation_unverified');
    return operation;
  }
  async function policy(observation,passive=false) {
    if(typeof resolvePolicy!=='function')fail('challenge_receiving_policy_required');
    // This port belongs to the receiving backend. It must read its existing
    // SitePolicy source and current addressed task-policy owner, never caller labels.
    const record=await resolvePolicy({taskId:observation.binding.taskId,binding:observation.binding,
      frameSha256:observation.frameSha256,classification:observation.classification});
    let origin;try{origin=new URL(record?.origin);}catch{fail('challenge_policy_origin_unverified');}
    const site=record.sitePolicy,task=record.taskPolicy,provider=record.provider,profile=record.profile;
    if(record.taskId!==observation.binding.taskId||!same(record.binding,observation.binding)||
      typeof record.sourceRef!=='string'||!record.sourceRef||record.sourceRef.length>256||
      origin.protocol!=='https:'||origin.origin!==record.origin||
      !(site?.originPattern===record.origin||site?.origins?.includes(record.origin))||
      !['provider_allowed','provider_preferred',...(passive?['avoid_first']:[])].includes(site?.challengePolicy)||
      profile?.id!==observation.binding.profileId||!profile.sitePolicyIds?.includes(site.id)||
      !passive&&(site.requiresCdpFree===true||site.interactionMode==='manual')||
      task?.taskId!==observation.binding.taskId||!Number.isSafeInteger(task.revision)||task.revision<0||
      task.policy?.deniedActions?.includes('native_input')||
      !passive&&(!provider||provider.kind!=='captcha'||provider.enabled!==true||
        provider.configRef!=='agent-browser:remote-view-native:v1'||!provider.capabilities?.includes('captcha_solve')||
        typeof provider.id!=='string'||!provider.id||!Array.isArray(site.allowedChallengeProviders)||
        site.allowedChallengeProviders.length>0&&!site.allowedChallengeProviders.includes(provider.id)))fail('challenge_policy_does_not_authorize_attempt');
    return structuredClone(record);
  }
  async function verifyCurrent(observation) {
    if(observation.control.owner!=='agent'||['stale','ambiguous','image_puzzle','access_denied','authentication_required','loading','passive_check','checkbox'].includes(observation.classification.kind))fail('challenge_current_observation_unverified');
    const context={taskId:observation.binding.taskId,binding:observation.binding,frameSha256:observation.frameSha256,classification:observation.classification};
    const clearance=await verifyClearance(context);
    if(!verified(clearance,context.taskId,context.frameSha256))fail('challenge_clearance_unverified');
    const access=await verifyAccess(context);
    if(!verified(access,context.taskId,context.frameSha256)||access.evidenceRef===clearance.evidenceRef)fail('site_access_unverified');
    const latest=await briefing(context.taskId);
    if(!same(latest,observation.binding)||latest.control?.owner!=='agent'||latest.control.epoch!==observation.control.epoch)fail('challenge_control_changed_during_verification');
    const continued=await call('browser_continue',{taskId:context.taskId});
    if(!same(continued,observation.binding))fail('challenge_continuation_identity_changed');
    return {clearance:clearance.evidenceRef,access:access.evidenceRef,frameSha256:context.frameSha256};
  }
  const api={
    observe:taskId=>observe(taskId),
    /** One durable attempt across registered checkbox families per task. Passive
     * checks get at most three observations and ten seconds, with no input.
     * Any refusal/unknown outcome falls back to the existing assistance flow.
     */
    async resolve({taskId,requestId}) {
      if(typeof requestId!=='string'||requestId.length>112||!/^[-A-Za-z0-9_.:]+$/.test(requestId))fail('challenge_stable_request_id_required');
      let retained=null;
      const assist=async reason=>{
        try{return {...await api.prepare({taskId}),reason,originalOperation:retained,retryAllowed:false};}
        catch{return {taskId,status:'intervention',reason,originalOperation:retained,retryAllowed:false,nextAction:'existing_human_assistance'};}
      };
      try {
        retained=await original(taskId,requestId);
        if(retained)return assist('original_attempt_retained_do_not_retry');
        let observation=await observe(taskId);
        if(observation.control.owner!=='agent')return assist('challenge_control_not_agent');
        if(observation.classification.kind==='passive_check') {
          const admitted=await policy(observation,true),deadline=now()+10000,initial=observation;
          for(let count=1;count<3&&observation.classification.kind==='passive_check'&&now()<deadline;count++) {
            await wait(250);
            observation=await observe(taskId,observation.binding);
            if(!samePhysical(initial.physical,observation.physical)||initial.control.epoch!==observation.control.epoch||
              initial.nativeControlEpoch!==observation.nativeControlEpoch)return assist('native_context_changed_during_passive_wait');
          }
          if(observation.classification.kind==='passive_check'||now()>=deadline)return assist('passive_wait_budget_exhausted');
          if(!isDeepStrictEqual(await policy(observation,true),admitted))return assist('challenge_policy_changed');
          return {taskId,status:'verified_access_after_passive_wait',binding:observation.binding,
            verification:await verifyCurrent(observation),taskCompleted:false,retryAllowed:false};
        }
        if(observation.classification.kind!=='checkbox')return assist('challenge_not_registered_checkbox');
        const admitted=await policy(observation),initial=observation;
        const registeredActionId=observation.classification.registeredProfileId;
        observation=await observe(taskId,initial.binding,{requestId:requestId+':observation',params:{registeredActionId}});
        const evidence=observation.nativeObservation;
        if(!samePhysical(initial.physical,observation.physical)||initial.control.epoch!==observation.control.epoch||
          evidence?.registeredActionId!==registeredActionId||evidence.frameSha256!==observation.frameSha256||
          evidence.desktopId!==observation.physical.desktopId||evidence.generation!==observation.physical.generation||
          evidence.windowId!==observation.physical.windowId||!Number.isSafeInteger(evidence.nativeControlEpoch)||
          !Number.isSafeInteger(evidence.observedAtMs)||now()<evidence.observedAtMs||now()-evidence.observedAtMs>10000)return assist('registered_target_unqualified');
        if(!isDeepStrictEqual(await policy(observation),admitted))return assist('challenge_policy_changed');
        const latest=await briefing(taskId);
        if(!same(latest,observation.binding)||latest.control?.owner!=='agent'||latest.control.epoch!==observation.control.epoch)return assist('challenge_control_changed');
        let delivered;
        try { delivered=await call('browser_command',{taskId,action:'native_input',requestId,
          params:{nativeAttempt:{observationRequestId:requestId+':observation',registeredActionId}}}); }
        catch { retained=await original(taskId,requestId);return assist('native_input_uncertain_or_refused'); }
        retained=delivered.taskOperation;
        if(retained?.requestId!==requestId||retained.taskId!==taskId||retained.action!=='native_input'||
          retained.state!=='owner_acknowledged'||delivered.delivery!=='dispatched')return assist('native_input_not_acknowledged');
        let ownerPolicy;
        try{const encoded=retained.evidenceReferences?.nativeSitePolicy;if(typeof encoded!=='string'||encoded.length>4096)throw Error();ownerPolicy=JSON.parse(encoded);}catch{return assist('native_owner_policy_unverified');}
        if(ownerPolicy.providerId!==admitted.provider.id||ownerPolicy.sitePolicyId!==admitted.sitePolicy.id||
          ownerPolicy.registeredActionId!==registeredActionId||ownerPolicy.family!==observation.classification.family||
          !/^[a-f0-9]{64}$/.test(ownerPolicy.digest??''))return assist('native_owner_policy_changed');
        let current;
        const verificationDeadline=now()+10000;
        for(let count=0;count<3;count++) {
          current=await observe(taskId,observation.binding);
          if(!samePhysical(current.physical,observation.physical)||current.control.epoch!==observation.control.epoch||
            current.nativeControlEpoch!==observation.nativeObservation.nativeControlEpoch)return assist('native_context_changed_after_dispatch');
          if(!['checkbox','passive_check','loading'].includes(current.classification.kind)||now()>=verificationDeadline)break;
          if(count<2)await wait(250);
        }
        return {taskId,status:'verified_access_after_native',binding:current.binding,originalOperation:retained,
          verification:await verifyCurrent(current),taskCompleted:false,retryAllowed:false};
      } catch(error) { return assist(error?.message??'challenge_intervention_required'); }
    },
    /** A ready link is preparation, never human use or successful assistance. */
    async prepare({taskId}) {
      const observation=await observe(taskId);
      if(observation.control.owner!=='agent')fail('challenge_control_not_agent');
      const handoff=await call('browser_handoff',{taskId});
      let url;
      try{url=new URL(handoff.handoffUrl);}catch{fail('challenge_handoff_invalid');}
      if(handoff.taskId!==taskId||handoff.operatorVisible?.state!=='ready'||url.protocol!=='https:'||
        url.username||url.password||url.search||url.hash||
        !['/remote-view/','/agent-browser/handoff/'].some(prefix=>url.pathname===prefix+handoff.handoffId))fail('challenge_handoff_invalid');
      return {schemaVersion:1,status:'awaiting_operator',taskId,binding:observation.binding,
        physical:observation.physical,classification:observation.classification,
        handoffId:handoff.handoffId,handoffUrl:handoff.handoffUrl,initialEpoch:observation.control.epoch};
    },
    /** The operator uses the existing Native toolbar. This method only observes it. */
    async recordTakeover(capsule) {
      const current=await briefing(capsule.taskId);
      if(!same(current,capsule.binding))fail('challenge_task_identity_changed');
      const result=await call('browser_control',{taskId:capsule.taskId,operation:'status'});
      if(result.taskId!==capsule.taskId||result.browserId!==capsule.binding.browserId||
        result.control?.owner!=='human'||!Number.isSafeInteger(result.control.epoch)||
        result.control.epoch<=capsule.initialEpoch)fail('challenge_explicit_takeover_not_observed');
      return {...structuredClone(capsule),status:'operator_has_control',humanEpoch:result.control.epoch};
    },
    /** Unknown or denied access remains intervention; continuation is read-only. */
    async continueAfterReturn(capsule) {
      if(capsule.status!=='operator_has_control'||!Number.isSafeInteger(capsule.humanEpoch))fail('challenge_takeover_receipt_required');
      const control=await call('browser_control',{taskId:capsule.taskId,operation:'status'});
      if(control.taskId!==capsule.taskId||control.browserId!==capsule.binding.browserId||
        control.control?.owner!=='agent'||control.control.epoch<=capsule.humanEpoch)return intervention(capsule,'explicit_return_not_observed');
      const current=await observe(capsule.taskId,capsule.binding);
      if(!samePhysical(current.physical,capsule.physical))return intervention(capsule,'native_target_changed');
      if(['stale','ambiguous'].includes(current.classification.kind))return intervention(capsule,'observation_unverified');
      const context={taskId:capsule.taskId,binding:current.binding,frameSha256:current.frameSha256,classification:current.classification};
      const clearance=await verifyClearance(context);
      if(!verified(clearance,capsule.taskId,current.frameSha256))return intervention(capsule,'challenge_clearance_unverified');
      const access=await verifyAccess(context);
      if(!verified(access,capsule.taskId,current.frameSha256)||access.evidenceRef===clearance.evidenceRef)return intervention(capsule,'site_access_unverified');
      const latest=await briefing(capsule.taskId);
      if(!same(latest,capsule.binding)||latest.control?.owner!=='agent'||latest.control.epoch!==current.control.epoch)return intervention(capsule,'control_changed_during_verification');
      const continued=await call('browser_continue',{taskId:capsule.taskId});
      if(!['taskId','profileId','browserId','tabId'].every(key=>continued[key]===capsule.binding[key]))fail('challenge_continuation_identity_changed');
      return {...structuredClone(capsule),status:'verified_access_after_return',
        verification:{clearance:clearance.evidenceRef,access:access.evidenceRef,frameSha256:current.frameSha256},
        result:continued,taskCompleted:false};
    },
  };
  return api;
}
