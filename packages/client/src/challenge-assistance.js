import {isDeepStrictEqual} from 'node:util';
/** Compose existing task, Native capture/handoff and cooperative control APIs.
 * Site-specific verification ports must independently observe the application.
 * No input, solve, viewer grant, credential operation or retry is added here.
 */
export function createChallengeAssistanceClient({callTool,verifyClearance,verifyAccess}) {
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
  async function observe(taskId,expected) {
    if(typeof taskId!=='string'||!taskId.trim())fail('challenge_task_required');
    const before=await briefing(taskId);
    if(before.taskId!==taskId||!same(before,before)||expected&&!same(before,expected))fail('challenge_task_identity_changed');
    const frame=await call('browser_command',{taskId,action:'native_capture',params:{classifyChallenges:true}});
    const after=await briefing(taskId);
    if(!same(before,after)||before.control?.epoch!==after.control?.epoch||before.control?.owner!==after.control?.owner)fail('challenge_context_changed');
    const c=frame.capture,sha=c?.capture?.sha256;
    if(frame.source!=='remote_view_native'||!c||typeof c.desktopId!=='string'||!Number.isSafeInteger(c.generation)||
      !Number.isSafeInteger(c.windowId)||!c.processIdentity||!/^([a-f0-9]{64})$/.test(sha??'')||
      frame.challenge?.frameSha256!==sha||frame.challenge?.corpusVersion!=='native-challenge-corpus.v1'||
      !Number.isFinite(frame.evidenceAgeMs)||frame.evidenceAgeMs<0||frame.evidenceAgeMs>10000)fail('challenge_native_evidence_unverified');
    return {binding:identity(after),physical:physical(frame),classification:frame.challenge,
      frameSha256:sha,observedAtMs:frame.observedAtMs??null,control:after.control};
  }
  function intervention(capsule,reason){return {...structuredClone(capsule),status:'intervention',reason};}
  function verified(proof,taskId,frameSha256) {
    return proof?.state==='verified'&&proof.taskId===taskId&&proof.frameSha256===frameSha256&&
      typeof proof.evidenceRef==='string'&&proof.evidenceRef.length>0&&proof.evidenceRef.length<=256;
  }
  return {
    observe:taskId=>observe(taskId),
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
}
