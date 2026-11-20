import { clientWaitBudget } from './client-wait-budget.js';
import { clientArgumentError } from './client-argument-error.js';
import { isResponseFailure, responseFailureError } from './response-failure.js';
import {createHash} from 'node:crypto';
import {isDeepStrictEqual} from 'node:util';
import {createChallengeAssistanceClient} from './challenge-assistance.js';

const fields=['taskId','profileId','browserId','tabId','targetId'];
/** One receiving-backend image-selection port, existing owner authority and two
 * independent application verifiers. A selection is never input authority.
 * @param {import('./native-image-challenge.d.ts').NativeImageOptions} options
 * @returns {import('./native-image-challenge.d.ts').NativeImageClient}
 */
export function createNativeImageChallengeClient(options) { return createImageClient(options, false); }
/** Two whole-grid static rounds share one attempt and caller wait allowance.
 * @param {import('./native-image-challenge.d.ts').NativeImageOptions} options
 * @returns {import('./native-image-challenge.d.ts').NativeImageClient}
 */
export function createNativeImageRoundsChallengeClient(options) { return createImageClient(options, true); }
/** @param {import('./native-image-challenge.d.ts').NativeImageOptions} options
 * @param {boolean} rounds
 * @returns {import('./native-image-challenge.d.ts').NativeImageClient} */
function createImageClient({callTool,selectCandidates,resolvePolicy,verifyClearance,verifyAccess,captureRegion,now=Date.now,timeoutMs=60000}, rounds) {
  const PROFILE=rounds?'hcaptcha-image-grid-3x3-two-round-v1':'hcaptcha-image-grid-3x3-v1',CONFIG=rounds?'agent-browser:remote-view-native-image-static-3x3-two-round:v1':'agent-browser:remote-view-native-image-static-3x3:v1';
  if([callTool,selectCandidates,resolvePolicy,verifyClearance,verifyAccess].some(port=>typeof port!=='function')||!Number.isSafeInteger(timeoutMs)||timeoutMs<1||timeoutMs>2147483647)throw clientArgumentError('native_image_ports_required', "packages/client/src/native-image-challenge.js", "createImageClient", ["callTool", "selectCandidates", "resolvePolicy", "verifyClearance", "verifyAccess", "timeoutMs"]);
  if(captureRegion && (!['x','y','width','height'].every(key=>Number.isSafeInteger(captureRegion[key])) || captureRegion.width<=0 || captureRegion.height<=0 || captureRegion.x<0 || captureRegion.y<0))throw clientArgumentError('native_image_capture_region_invalid', "packages/client/src/native-image-challenge.js", "createImageClient", ["captureRegion"]);
  const regionRequest=captureRegion?structuredClone(captureRegion):null;
  const human=createChallengeAssistanceClient({callTool,resolvePolicy,verifyClearance,verifyAccess,now});
  const fail=code=>{throw Error(code)};

  const identity=value=>({taskId:value.taskId,profileId:value.profileId,browserId:value.browserId,tabId:value.tabId,targetId:value.targetId});
  const same=(a,b)=>fields.every(key=>typeof a?.[key]==='string'&&a[key]===b?.[key]);
  async function call(name,args) {
    const reply=/** @type {any} */(await callTool(name,args));
    const text=reply?.content?.find(block=>block.type==='text')?.text;
    if(typeof text!=='string'||text.length>8*1024*1024)fail('native_image_workflow_response_invalid');
    let envelope;try{envelope=JSON.parse(text)}catch{fail('native_image_workflow_response_invalid')}
    if(reply?.isError||envelope.success!==true||isResponseFailure(envelope))throw responseFailureError(envelope,'native_image_workflow_refused');
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
      if(reply?.isError||envelope.success!==true||isResponseFailure(envelope)){if(missing(envelope.error)||missing(envelope.failure)||missing(envelope.data?.failure))return null;throw responseFailureError(envelope,'native_image_original_unverified')}
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
      const budget=clientWaitBudget(timeoutMs,'native_image_invocation_budget_exhausted');
      let retained=null, attemptedRequestId=requestId;
      /** @type {Array<Record<string,unknown>>} */
      const roundOperations=[];
      /** @type {(reason:string)=>Promise<import("./native-image-challenge.d.ts").NativeImageResult>} */
      const assist=async reason=>{try{const {verification:_previousVerification,...capsule}=await budget.run(()=>human.prepare({taskId}));return {...capsule,status:capsule.status==='awaiting_operator'?'awaiting_operator':'intervention',reason,originalOperation:retained,...(rounds?{roundOperations}:{}),retryAllowed:false}}catch{return {taskId,status:'intervention',reason,originalOperation:retained,...(rounds?{roundOperations}:{}),retryAllowed:false}}};
      try {
        retained=await budget.run(()=>original(taskId,requestId));if(retained){if(rounds){roundOperations.push(retained);const second=await budget.run(()=>original(taskId,requestId+':round2'));if(second)roundOperations.push(second)}return await assist('native_image_original_exists_do_not_replay');}
        let previous=null, nextCapture=null, originalBinding=null;
        for(let roundIndex=1;roundIndex<=(rounds?2:1);roundIndex++) {
        const roundRequestId=roundIndex===1?requestId:requestId+':round2';
        const before=await budget.run(()=>briefing(taskId)),observationRequestId=requestId+(roundIndex===1?':observation':':observation2');
        if(originalBinding&&!same(originalBinding,before))fail('native_image_context_changed');
        const captured=nextCapture??await budget.run(()=>call('browser_command',{taskId,action:'native_capture',requestId:observationRequestId,params:{classifyChallenges:true,registeredActionId:PROFILE,...(regionRequest?{imageCaptureScope:"complete_puzzle_region",region:regionRequest}:{})}}));
        const frame=captured.data,sha=image(frame,captured.images),observation=frame.nativeImageObservation;
        const after=await budget.run(()=>briefing(taskId));
        if(!same(before,after)||before.control.epoch!==after.control.epoch||frame.challenge?.kind!=='image_puzzle'||frame.challenge.family!=='hcaptcha'||(regionRequest?observation?.captureScope!=='complete_puzzle_region':frame.challenge.completeWindow!==true)||frame.challenge.frameSha256!==sha||frame.challenge.automatedAttemptAuthorized!==false||observation?.registeredActionId!==PROFILE||observation.frameSha256!==sha||!/^([a-f0-9]{64})$/.test(observation.puzzleSha256??'')||observation.desktopId!==physical(frame).desktopId||observation.generation!==physical(frame).generation||observation.windowId!==physical(frame).windowId||!Number.isSafeInteger(observation.nativeControlEpoch)||!Array.isArray(observation.candidates)||observation.candidates.length!==9||observation.candidates.some(candidate=>typeof candidate.candidateId!=='string'||candidate.candidateId.length>128)||new Set(observation.candidates.map(candidate=>candidate.candidateId)).size!==9||!isDeepStrictEqual(observation.window,frame.windowBounds)||!isDeepStrictEqual(observation.region,regionRequest??frame.windowBounds)||(!regionRequest&&(observation.captureScope??'complete_window')!=='complete_window'))fail('native_image_observation_unqualified');
        if(rounds && (!Array.isArray(observation.tileImageSha256s)||observation.tileImageSha256s.length!==9||observation.tileImageSha256s.some(hash=>!/^([a-f0-9]{64})$/.test(hash))))fail('native_image_observation_unqualified');
        if(previous && (previous.frameSha256===sha || previous.puzzleSha256===observation.puzzleSha256 || !isDeepStrictEqual(previous.window,observation.window) || !isDeepStrictEqual(previous.region,observation.region) || previous.nativeControlEpoch!==observation.nativeControlEpoch || previous.processDigest!==observation.processDigest || previous.tileImageSha256s.some((hash,index)=>hash===observation.tileImageSha256s[index])))fail('native_image_round_changed');
        originalBinding??=after;
        await budget.run(()=>sourcePolicy(taskId,after,frame));
        const selectionBudget=Math.min(45000,timeoutMs);
        budget.check();
        const controller=new AbortController();let timer;
        const abortSelection=()=>controller.abort();budget.signal.addEventListener('abort',abortSelection,{once:true});
        let proposal;
        try{proposal=await budget.run(()=>Promise.race([
          Promise.resolve().then(()=>selectCandidates({taskId,binding:identity(after),requestId,observationRequestId,observation:structuredClone(observation),images:captured.images,deadlineAtMs:now()+selectionBudget,timeoutMs:selectionBudget,roundIndex,signal:controller.signal})),
          new Promise((_,reject)=>{timer=setTimeout(()=>{controller.abort();reject(Error('native_image_selection_timeout'))},selectionBudget)}),
        ]))}finally{clearTimeout(timer);budget.signal.removeEventListener('abort',abortSelection)}
        const selected=proposal?.selectedCandidateIds;
        if(proposal?.taskId!==taskId||proposal.observationRequestId!==observationRequestId||proposal.frameSha256!==sha||proposal.puzzleSha256!==observation.puzzleSha256||!Array.isArray(selected)||selected.length<1||selected.length>4||new Set(selected).size!==selected.length||selected.some(id=>!observation.candidates.some(candidate=>candidate.candidateId===id)))fail('native_image_selection_unqualified');
        const current=await budget.run(()=>briefing(taskId));if(!same(after,current)||after.control.epoch!==current.control.epoch)fail('native_image_context_changed');
        attemptedRequestId=roundRequestId;
        const input=await budget.run(()=>call('browser_command',{taskId,action:'native_input',requestId:roundRequestId,params:{imageAttempt:{observationRequestId,selectedCandidateIds:selected,...(rounds?{originalRequestId:requestId}:{})}}}));
        retained=await budget.run(()=>original(taskId,requestId));
        const delivered=await budget.run(()=>original(taskId,roundRequestId));if(delivered)roundOperations.push(delivered);
        if(input.data?.source!=='remote_view_native'||input.data.delivery!=='dispatched'||delivered?.state!=='owner_acknowledged')return await assist('native_image_input_uncertain_or_refused');
        const observed=await budget.run(()=>call('browser_command',{taskId,action:'native_capture',...(rounds&&roundIndex===1?{requestId:requestId+':observation2'}:{}),params:{classifyChallenges:true,...(rounds&&roundIndex===1?{registeredActionId:PROFILE,...(regionRequest?{imageCaptureScope:'complete_puzzle_region'}:{})}:{}),...(regionRequest?{region:regionRequest}:{})}})),fresh=observed.data,freshSha=image(fresh,observed.images);
        if(rounds && roundIndex===1 && fresh.challenge?.kind==='image_puzzle') {
          if(!isDeepStrictEqual(physical(frame),physical(fresh))||!isDeepStrictEqual(frame.windowBounds,fresh.windowBounds)||fresh.nativeControlEpoch!==observation.nativeControlEpoch)fail('native_image_round_changed');
          previous=observation;nextCapture=observed;continue;
        }
        if(!isDeepStrictEqual(physical(frame),physical(fresh))||!isDeepStrictEqual(frame.windowBounds,fresh.windowBounds)||fresh.nativeControlEpoch!==observation.nativeControlEpoch||fresh.challenge?.kind!=='unknown'||(!regionRequest&&fresh.challenge.completeWindow!==true)||fresh.challenge.frameSha256!==freshSha||fresh.challenge.automatedAttemptAuthorized!==false)fail('native_image_after_state_unverified');
        const context={taskId,binding:identity(after),frameSha256:freshSha,classification:fresh.challenge};
        const clearance=await budget.run(()=>verifyClearance(context));if(!verified(clearance,context))fail('native_image_clearance_unverified');
        const access=await budget.run(()=>verifyAccess(context));if(!verified(access,context)||access.evidenceRef===clearance.evidenceRef)fail('native_image_site_access_unverified');
        const latest=await budget.run(()=>briefing(taskId));if(!same(after,latest)||latest.control.epoch!==after.control.epoch)fail('native_image_context_changed');
        const continued=(await budget.run(()=>call('browser_continue',{taskId}))).data;if(!same(after,continued))fail('native_image_continuation_changed');
        return {taskId,status:'verified_access_after_native_image',binding:identity(after),originalOperation:retained,...(rounds?{roundOperations}:{}),verification:{clearance:clearance.evidenceRef,access:access.evidenceRef,frameSha256:freshSha},continuation:continued,retryAllowed:false};
        }
        return await assist('native_image_round_budget_exhausted');
      }catch(error){try{retained=await budget.run(()=>original(taskId,requestId));const uncertain=await budget.run(()=>original(taskId,attemptedRequestId));if(uncertain&&!roundOperations.some(operation=>operation.requestId===attemptedRequestId))roundOperations.push(uncertain)}catch{}const message=String(error?.message??'');return {...await assist(/^native_image_[a-z_]+$/.test(message)?message:'native_image_workflow_unavailable_observe_original'),...(error?.failure?{failure:error.failure,occurrenceId:error.occurrenceId}:{}),recovery:{inspect:{interface:'@agent-browser/client/task-briefing#getTaskOperation',arguments:{taskId,requestId:attemptedRequestId}},resumeCondition:'Inspect the exact original operation before another attempt; unknown input never replays. Caller wait expiry is not owner evidence expiry.'}};}finally{budget.close()}
    },
  };
}
