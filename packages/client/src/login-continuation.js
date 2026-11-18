// @ts-check
import {createHash} from 'node:crypto';
import {summarizeLoginProgress} from './login-progress.js';

/** Extend the host-owned continuation record without replacing its Native
 * episode or creating a second journal. All external callbacks are read-only
 * except the host's existing private record store and optional update callback.
 * @param {import('./login-continuation.d.ts').LoginContinuationOptions} options
 * @returns {import('./login-continuation.d.ts').LoginContinuation}
 */
export function createLoginContinuation({store,callerTaskId,inspectRequest,verify,readWake,onUpdate}) {
  const invalid=()=>Object.assign(Error('Original login continuation is unverified'),{code:'login_continuation_unverified'});
  if(!store||['load','save','withEpisode'].some(key=>typeof store[key]!=='function')||!reference(callerTaskId)||typeof inspectRequest!=='function'||typeof verify!=='function'||onUpdate!==undefined&&typeof onUpdate!=='function'||readWake!==undefined&&typeof readWake!=='function')throw invalid();
  return {async resume({episodeId}) {
    if(!reference(episodeId))throw invalid();
    return store.withEpisode(episodeId,async()=>{
      const record=await store.load(episodeId),checkpoint=record?.loginContinuation;
      if(!record||checkpoint?.schemaVersion!==1||!reference(checkpoint.workflowId)||!reference(checkpoint.requestId)||checkpoint.attemptId!==undefined&&!reference(checkpoint.attemptId)||!bindingFields.every(key=>reference(checkpoint.binding?.[key]))||!origin(checkpoint.binding?.origin)||!origin(checkpoint.requestedOutcome?.applicationOrigin)||!identity(checkpoint.requestedOutcome?.expectedAccount)||checkpoint.requestedOutcome.expectedOrganization!==undefined&&!identity(checkpoint.requestedOutcome.expectedOrganization)||!Array.isArray(checkpoint.unresolvedAttempts)||checkpoint.unresolvedAttempts.some(attempt=>!reference(attempt?.requestId)||!reference(attempt?.attemptId))||checkpoint.wakeIntentId!==undefined&&!reference(checkpoint.wakeIntentId)||record.episodeId!==undefined&&record.episodeId!==episodeId||record.taskId!==undefined&&record.taskId!==checkpoint.binding.taskId)throw invalid();
      const request=await inspectRequest({requestId:checkpoint.requestId});
      if(request?.requestId!==checkpoint.requestId||request.attemptId!==checkpoint.attemptId||request.taskId!==callerTaskId||request.targetId!==checkpoint.binding?.taskId||request.origin!==checkpoint.binding?.origin||request.operation!=='login'||request.mode!=='private')throw invalid();
      const unresolvedAttempts=structuredClone(checkpoint.unresolvedAttempts);
      if(request.state==='unknown'&&reference(request.attemptId)&&!unresolvedAttempts.some(attempt=>attempt.requestId===request.requestId&&attempt.attemptId===request.attemptId))unresolvedAttempts.push({requestId:request.requestId,attemptId:request.attemptId});
      /** @type {import('./login-progress.js').LoginProgress & Partial<Pick<import('./login-verification.js').LoginVerificationResult,'qualification'|'receivingSettlement'>>} */
      let progress=summarizeLoginProgress(request);
      if(checkpoint.attemptId&&!['denied','revoked','cancelled','expired'].includes(request.state)) {
        const verified=await verify({requestId:checkpoint.requestId,attemptId:checkpoint.attemptId,binding:structuredClone(checkpoint.binding),applicationOrigin:checkpoint.requestedOutcome.applicationOrigin,expectedAccount:checkpoint.requestedOutcome.expectedAccount,...(checkpoint.requestedOutcome.expectedOrganization===undefined?{}:{expectedOrganization:checkpoint.requestedOutcome.expectedOrganization})});
        if(verified?.requestId===checkpoint.requestId&&verified.outcome==='incomplete'&&['verification_incomplete','request_binding_unverified','completion_unverified','verification_unavailable'].includes(verified.category)&&(verified.category!=='verification_incomplete'||verified.qualification?.applicationAccess==='verified')&&Array.isArray(verified.evidenceReferences)&&verified.evidenceReferences.every(reference)) {
          const action=verified.nextAction;
          const nextAction=action&&['inspect_credential_request','inspect_authentication_completion'].includes(action.operation)&&action.args?.requestId===checkpoint.requestId&&Object.keys(action.args).length===1?{operation:action.operation,args:{requestId:checkpoint.requestId}}:progress.nextAction;
          progress={schemaVersion:1,outcome:'incomplete',category:verified.category,component:['auth','remote-view','application'].includes(verified.component)?verified.component:'application',waitingForUser:false,evidenceReferences:[...verified.evidenceReferences],nextAction,requestId:checkpoint.requestId,qualification:{applicationAccess:verified.qualification?.applicationAccess==='verified'?'verified':'unverified',authCompletion:verified.qualification?.authCompletion==='caller_observed'?'caller_observed':'unverified',installation:'unverified',otherLoginPaths:'unverified'}};
        }
        if(verified?.requestId===checkpoint.requestId&&verified.outcome==='verified'&&verified.category==='verified_access'&&verified.qualification?.applicationAccess==='verified'&&verified.qualification?.authCompletion==='caller_observed'&&Array.isArray(verified.evidenceReferences)&&verified.evidenceReferences.length>=2&&verified.evidenceReferences.every(reference)) {
          progress={schemaVersion:1,outcome:'verified',category:'verified_access',component:'application',waitingForUser:false,evidenceReferences:[...verified.evidenceReferences],nextAction:null,requestId:checkpoint.requestId,qualification:{applicationAccess:'verified',authCompletion:'caller_observed',installation:'unverified',otherLoginPaths:'unverified'},...(['settled','held','unverified'].includes(verified.receivingSettlement)?{receivingSettlement:verified.receivingSettlement}:{})};
        }
      }
      const wake=await wakeState(checkpoint.wakeIntentId,readWake);
      const publicAttempts=unresolvedAttempts.map(({requestId,attemptId})=>({requestId,attemptId}));
      const update={...progress,workflowId:checkpoint.workflowId,unresolvedAttempts:publicAttempts,wake,wakeCleanupRequired:progress.outcome==='verified'&&!['not_used','cancelled'].includes(wake.state)};
      const key=createHash('sha256').update(JSON.stringify({...update,evidenceReferences:[...update.evidenceReferences].sort(),unresolvedAttempts:[...publicAttempts].sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b)))})).digest('hex');
      const changed=checkpoint.notification?.key!==key;
      const notification=changed?{key,state:/** @type {const} */('intent')}:{key,state:['callback_acknowledged','unknown'].includes(checkpoint.notification.state)?checkpoint.notification.state:'unknown'};
      const next={...record,loginContinuation:{...checkpoint,progress,unresolvedAttempts,...(onUpdate?{notification}:{})}};
      await store.save(next);
      let emitted=false;
      if(changed&&onUpdate) {
        try {await onUpdate({...update,key});notification.state='callback_acknowledged';emitted=true;}
        catch {notification.state='unknown';}
        await store.save(next);
      }
      return {...update,notification:{key,state:onUpdate?notification.state:'not_configured',emitted}};
    });
  }};
}

/** @param {unknown} value */
const reference=value=>typeof value==='string'&&/^[A-Za-z0-9][A-Za-z0-9:._-]{0,199}$/.test(value);
const bindingFields=/** @type {const} */(['taskId','profileId','browserId','tabId','targetId']);
/** @param {unknown} value */
const identity=value=>typeof value==='string'&&value.trim().length>0&&value.length<=512&&!/[\r\n\x00]/.test(value);
/** @param {unknown} value */
function origin(value) {try {const url=new URL(/** @type {string} */(value));return url.protocol==='https:'&&url.origin===value;}catch{return false;}}

/** @param {string|undefined} intentId
 * @param {import('./login-continuation.d.ts').LoginContinuationOptions['readWake']} readWake
 * @returns {Promise<import('./login-continuation.d.ts').LoginContinuationResult['wake']>} */
async function wakeState(intentId,readWake) {
  if(!intentId)return {state:'not_used'};
  const unknown={intentId,state:/** @type {const} */('unverified')};
  if(!readWake)return unknown;
  try {
    const receipt=await readWake(intentId);
    if(receipt?.intentId!==intentId)return unknown;
    if(receipt.state==='cancelled')return {intentId,state:'cancelled'};
    if(receipt.state==='held')return {intentId,state:'held'};
    if(receipt.state!=='registered')return unknown;
    return {intentId,state:receipt.armed===true&&receipt.monitor?.ready===true?'ready':'registered'};
  }catch{return unknown;}
}
