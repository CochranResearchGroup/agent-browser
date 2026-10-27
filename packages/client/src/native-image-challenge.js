import {createHash} from 'node:crypto';
import {isDeepStrictEqual} from 'node:util';
import {createChallengeAssistanceClient} from './challenge-assistance.js';
const PROFILE='hcaptcha-image-grid-3x3-v1',CONFIG='agent-browser:remote-view-native-image-static-3x3:v1';
const fields=['taskId','profileId','browserId','tabId','targetId'];
/** One receiving-backend image-selection port, existing owner authority and two
 * independent application verifiers. A selection is never input authority.
 * @param {import('./native-image-challenge.d.ts').NativeImageOptions} options
 * @returns {import('./native-image-challenge.d.ts').NativeImageClient}
 */
export function createNativeImageChallengeClient({callTool,selectCandidates,resolvePolicy,verifyClearance,verifyAccess,captureRegion,now=Date.now}) {
  if([callTool,selectCandidates,resolvePolicy,verifyClearance,verifyAccess].some(port=>typeof port!=='function'))throw TypeError('native_image_ports_required');
  if(captureRegion && (!['x','y','width','height'].every(key=>Number.isSafeInteger(captureRegion[key])) || captureRegion.width<=0 || captureRegion.height<=0 || captureRegion.x<0 || captureRegion.y<0))throw TypeError('native_image_capture_region_invalid');
  const regionRequest=captureRegion?structuredClone(captureRegion):null;
  const human=createChallengeAssistanceClient({callTool,resolvePolicy,verifyClearance,verifyAccess,now});
  const fail=code=>{throw Error(code)};
  const identity=value=>({taskId:value.taskId,profileId:value.profileId,browserId:value.browserId,tabId:value.tabId,targetId:value.targetId});
  const same=(a,b)=>fields.every(key=>typeof a?.[key]==='string'&&a[key]===b?.[key]);
  async function call(name,args) {
    const reply=/** @type {any} */(await callTool(name,args));
    const text=reply?.content?.find(block=>block.type==='text')?.text;
    if(reply?.isError||typeof text!=='string'||text.length>8*1024*1024)fail('native_image_workflow_response_invalid');
    let envelope;try{envelope=JSON.parse(text)}catch{fail('native_image_workflow_response_invalid')}
    if(envelope.success!==true)fail('native_image_workflow_refused');
    return {data:envelope.data,images:reply.content.filter(block=>block.type==='image')};
  }
  async function briefing(taskId) {const {data}=await call('browser_command',{taskId,action:'task_briefing'});if(!same(data,data)||data.taskId!==taskId||data.control?.owner!=='agent'||!Number.isSafeInteger(data.control.epoch))fail('native_image_task_control_unverified');return data;}
  async function original(taskId,requestId) {
    const missing=value=>value?.code==='browser_task_operation_not_found'||value?.message==='browser_task_operation_not_found'||value?.data?.message==='browser_task_operation_not_found';
    let response;
    try{
      const reply=/** @type {any} */(await callTool('browser_command',{taskId,action:'task_operation',params:{requestId}}));
      const text=reply?.content?.find(block=>block.type==='text')?.text;
      if(typeof text!=='string'||text.length>8*1024*1024)fail('native_image_original_unverified');
      let envelope;try{envelope=JSON.parse(text)}catch{fail('native_image_original_unverified')}
      if(reply?.isError||envelope.success!==true){if(missing(envelope.error))return null;fail('native_image_original_unverified')}
      response={data:envelope.data};
    }catch(error){
      if(missing(error))return null;
      const message=error?.message;if(typeof message==='string'&&message.length<=8192){const start=message.indexOf('{');if(start>=0)try{if(missing(JSON.parse(message.slice(start))))return null}catch{}}
      throw error;
    }
    const operation=response.data?.operation;
    if(operation?.taskId!==taskId||operation.requestId!==requestId||operation.action!=='native_input')fail('native_image_original_unverified');return operation;
  }
  function physical(frame){const c=frame.capture;return {desktopId:c?.desktopId,generation:c?.generation,windowId:c?.windowId,processIdentity:c?.processIdentity};}
  function image(frame,images) {
    const sha=frame.capture?.capture?.sha256,region=frame.region??frame.capture?.region;
    if(frame.source!=='remote_view_native'||typeof frame.capture?.desktopId!=='string'||!Number.isSafeInteger(frame.capture?.generation)||!Number.isSafeInteger(frame.capture?.windowId)||!frame.capture?.processIdentity||(regionRequest?!isDeepStrictEqual(region,regionRequest):frame.partialWindow!==false)||!/^([a-f0-9]{64})$/.test(sha??'')||images.length!==1||images[0].mimeType!=='image/png'||typeof images[0].data!=='string'||images[0].data.length>5592408||!/^[A-Za-z0-9+/]*={0,2}$/.test(images[0].data))fail('native_image_frame_unverified');
    if(regionRequest && (frame.partialWindow!==!isDeepStrictEqual(region,frame.windowBounds) || region.x<frame.windowBounds?.x || region.y<frame.windowBounds?.y || region.x+region.width>frame.windowBounds.x+frame.windowBounds.width || region.y+region.height>frame.windowBounds.y+frame.windowBounds.height))fail('native_image_frame_unverified');
    const bytes=Buffer.from(images[0].data,'base64');
    if(bytes.length<33||bytes.length>4194304||!bytes.subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10]))||bytes.readUInt32BE(16)!==region?.width||bytes.readUInt32BE(20)!==region?.height||createHash('sha256').update(bytes).digest('hex')!==sha)fail('native_image_frame_unverified');
    return sha;
  }
  async function sourcePolicy(taskId,binding,frame) {
    const record=await resolvePolicy({taskId,binding:identity(binding),frameSha256:frame.capture.capture.sha256,classification:frame.challenge});
    let origin;try{origin=new URL(record?.origin)}catch{fail('native_image_policy_unverified')}
    const site=record?.sitePolicy,provider=record?.provider,profile=record?.profile,task=record?.taskPolicy;
    if(record?.taskId!==taskId||!same(record.binding,binding)||typeof record.sourceRef!=='string'||!record.sourceRef||record.sourceRef.length>256||origin.protocol!=='https:'||origin.origin!==record.origin||!(site?.originPattern===record.origin||site?.origins?.includes(record.origin))||!['provider_allowed','provider_preferred'].includes(site?.challengePolicy)||site.requiresCdpFree===true||site.interactionMode==='manual'||profile?.id!==binding.profileId||!profile.sitePolicyIds?.includes(site.id)||task?.taskId!==taskId||!Number.isSafeInteger(task.revision)||task.policy?.deniedActions?.includes('native_input')||provider?.kind!=='captcha'||provider.enabled!==true||provider.configRef!==CONFIG||!provider.capabilities?.includes('captcha_solve')||!Array.isArray(site.allowedChallengeProviders)||!site.allowedChallengeProviders.includes(provider.id))fail('native_image_policy_unverified');
  }
  const verified=(proof,context)=>proof?.state==='verified'&&proof.taskId===context.taskId&&proof.frameSha256===context.frameSha256&&typeof proof.evidenceRef==='string'&&proof.evidenceRef.length>0&&proof.evidenceRef.length<=256;
  return {
    async resolve({taskId,requestId}) {
      if(typeof requestId!=='string'||requestId.length>112||!/^[-A-Za-z0-9_.:]+$/.test(requestId))fail('native_image_stable_request_id_required');
      let retained=null;
      /** @type {(reason:string)=>Promise<import("./native-image-challenge.d.ts").NativeImageResult>} */
      const assist=async reason=>{try{const {verification:_previousVerification,...capsule}=await human.prepare({taskId});return {...capsule,status:capsule.status==='awaiting_operator'?'awaiting_operator':'intervention',reason,originalOperation:retained,retryAllowed:false}}catch{return {taskId,status:'intervention',reason,originalOperation:retained,retryAllowed:false}}};
      try {
        retained=await original(taskId,requestId);if(retained)return assist('native_image_original_exists_do_not_replay');
        const before=await briefing(taskId),observationRequestId=requestId+':observation';
        const captured=await call('browser_command',{taskId,action:'native_capture',requestId:observationRequestId,params:{classifyChallenges:true,registeredActionId:PROFILE,...(regionRequest?{imageCaptureScope:"complete_puzzle_region",region:regionRequest}:{})}});
        const frame=captured.data,sha=image(frame,captured.images),observation=frame.nativeImageObservation;
        const after=await briefing(taskId);
        if(!same(before,after)||before.control.epoch!==after.control.epoch||frame.challenge?.kind!=='image_puzzle'||frame.challenge.family!=='hcaptcha'||(regionRequest?observation?.captureScope!=='complete_puzzle_region':frame.challenge.completeWindow!==true)||frame.challenge.frameSha256!==sha||frame.challenge.automatedAttemptAuthorized!==false||observation?.registeredActionId!==PROFILE||observation.frameSha256!==sha||!/^([a-f0-9]{64})$/.test(observation.puzzleSha256??'')||observation.desktopId!==physical(frame).desktopId||observation.generation!==physical(frame).generation||observation.windowId!==physical(frame).windowId||!Number.isSafeInteger(observation.nativeControlEpoch)||!Number.isFinite(observation.observedAtMs)||now()<observation.observedAtMs||now()-observation.observedAtMs>=60000||!Array.isArray(observation.candidates)||observation.candidates.length!==9||observation.candidates.some(candidate=>typeof candidate.candidateId!=='string'||candidate.candidateId.length>128)||new Set(observation.candidates.map(candidate=>candidate.candidateId)).size!==9||!isDeepStrictEqual(observation.window,frame.windowBounds)||!isDeepStrictEqual(observation.region,regionRequest??frame.windowBounds)||(!regionRequest&&(observation.captureScope??'complete_window')!=='complete_window'))fail('native_image_observation_unqualified');
        await sourcePolicy(taskId,after,frame);
        const selectionBudget=Math.min(45000,observation.observedAtMs+60000-now());
        if(selectionBudget<=0)fail('native_image_selection_expired');
        const controller=new AbortController();let timer;
        let proposal;
        try{proposal=await Promise.race([
          Promise.resolve().then(()=>selectCandidates({taskId,binding:identity(after),requestId,observationRequestId,observation:structuredClone(observation),images:captured.images,deadlineAtMs:now()+selectionBudget,signal:controller.signal})),
          new Promise((_,reject)=>{timer=setTimeout(()=>{controller.abort();reject(Error('native_image_selection_timeout'))},selectionBudget)}),
        ])}finally{clearTimeout(timer)}
        const selected=proposal?.selectedCandidateIds;
        if(proposal?.taskId!==taskId||proposal.observationRequestId!==observationRequestId||proposal.frameSha256!==sha||proposal.puzzleSha256!==observation.puzzleSha256||!Array.isArray(selected)||selected.length<1||selected.length>4||new Set(selected).size!==selected.length||selected.some(id=>!observation.candidates.some(candidate=>candidate.candidateId===id))||now()-observation.observedAtMs>=60000)fail('native_image_selection_unqualified');
        const current=await briefing(taskId);if(!same(after,current)||after.control.epoch!==current.control.epoch)fail('native_image_context_changed');
        const input=await call('browser_command',{taskId,action:'native_input',requestId,params:{imageAttempt:{observationRequestId,selectedCandidateIds:selected}}});
        retained=await original(taskId,requestId);
        if(input.data?.source!=='remote_view_native'||input.data.delivery!=='dispatched'||retained?.state!=='owner_acknowledged')return assist('native_image_input_uncertain_or_refused');
        const observed=await call('browser_command',{taskId,action:'native_capture',params:{classifyChallenges:true,...(regionRequest?{region:regionRequest}:{})}}),fresh=observed.data,freshSha=image(fresh,observed.images);
        if(!isDeepStrictEqual(physical(frame),physical(fresh))||!isDeepStrictEqual(frame.windowBounds,fresh.windowBounds)||fresh.nativeControlEpoch!==observation.nativeControlEpoch||fresh.challenge?.kind!=='unknown'||(!regionRequest&&fresh.challenge.completeWindow!==true)||fresh.challenge.frameSha256!==freshSha||fresh.challenge.automatedAttemptAuthorized!==false)fail('native_image_after_state_unverified');
        const context={taskId,binding:identity(after),frameSha256:freshSha,classification:fresh.challenge};
        const clearance=await verifyClearance(context);if(!verified(clearance,context))fail('native_image_clearance_unverified');
        const access=await verifyAccess(context);if(!verified(access,context)||access.evidenceRef===clearance.evidenceRef)fail('native_image_site_access_unverified');
        const latest=await briefing(taskId);if(!same(after,latest)||latest.control.epoch!==after.control.epoch)fail('native_image_context_changed');
        const continued=(await call('browser_continue',{taskId})).data;if(!same(after,continued))fail('native_image_continuation_changed');
        return {taskId,status:'verified_access_after_native_image',binding:identity(after),originalOperation:retained,verification:{clearance:clearance.evidenceRef,access:access.evidenceRef,frameSha256:freshSha},continuation:continued,retryAllowed:false};
      }catch(error){try{retained=await original(taskId,requestId)}catch{}const message=String(error?.message??'');return assist(/^native_image_[a-z_]+$/.test(message)?message:'native_image_workflow_unavailable_observe_original');}
    },
  };
}
