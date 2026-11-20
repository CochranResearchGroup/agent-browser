import {withClientResultRecovery} from './client-recovery.js';
import {withClientErrorRecovery} from './client-recovery.js';
import { clientArgumentError } from './client-argument-error.js';
// @ts-check
/** @typedef {import('./owner-totp-continuation.d.ts').OwnerTotpOptions} OwnerTotpOptions */
/** @typedef {import('./owner-totp-continuation.d.ts').OwnerTotpDependencies} OwnerTotpDependencies */
/** @typedef {import('./owner-totp-continuation.d.ts').OwnerTotpCheckpoint} OwnerTotpCheckpoint */
const contract='auth-receipts-totp-v1';
const receivingFields=/** @type {const} */ (['taskId','profileId','browserId','tabId','generation']);
const intentFields=/** @type {const} */ (['taskId','guideId','accountRef','organizationRef','origin','targetId','purpose','expiresAt','idempotencyKey','originalRequestId']);
/** @param {unknown} value */
const nonempty=value=>typeof value==='string'&&value.length>0;
/** @param {unknown} error */
const errorCode=error=>error&&typeof error==='object'&&'code' in error&&typeof error.code==='string'?error.code:'auth_unavailable';

/** Advance Auth's public TOTP observation/continuation workflow. Private material,
 * decisions and receiving privacy stay with Auth. Durable browser metadata keeps
 * the original continuation/request across restart; unknown input is readback-only.
 * Password continuations use Auth's validated original-request custody transfer;
 * SSO continuations use the returned continue_authentication action.
 * @param {OwnerTotpDependencies} dependencies
 */
export function createOwnerTotpContinuation({auth,browser,checkpoints}){
 if(!auth||typeof auth.helper!=='function'||!nonempty(auth.callerTaskId)||!browser||(/** @type {const} */ (['verifyLogin','prepareLogin','validatePlan'])).some(method=>typeof browser[method]!=='function')||!checkpoints?.load||!checkpoints?.save)throw clientArgumentError('owner_totp_dependencies_required', "packages/client/src/owner-totp-continuation.js", "createOwnerTotpContinuation", ["auth", "browser", "checkpoints"]);
 const busy=new Set();
 return {
  /** Request no password and expose no factor value. The optional recordId must
   * select an Auth-returned candidate before a factor request has been created.
   * Auth binds account-less observations to the original password request and
   * creates the factor request through its observed continuation action.
   * @param {OwnerTotpOptions} options
   */
  async advance(options){
   if(!options||intentFields.filter(field=>!['organizationRef','originalRequestId'].includes(field)).some(field=>field==='expiresAt'?!Number.isSafeInteger(options[field]):!nonempty(options[field]))||(/** @type {const} */ (['organizationRef','originalRequestId','recordId'])).some(field=>options[field]!==undefined&&!nonempty(options[field])))throw withClientErrorRecovery(Error('owner_totp_options_invalid'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_totp_options_invalid');
   try{if(new URL(options.origin).origin!==options.origin||!options.origin.startsWith('https://'))throw 0;}catch{throw withClientErrorRecovery(Error('owner_totp_origin_invalid'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_totp_origin_invalid');}
   const key=`totp:${options.taskId}/${options.guideId}`;
   if(busy.has(key))throw withClientErrorRecovery(Error('owner_totp_busy'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_totp_busy');busy.add(key);
   const intent=JSON.stringify({...Object.fromEntries(intentFields.map(field=>[field,options[field]])),callerTaskId:auth.callerTaskId});
   /** @type {OwnerTotpCheckpoint|null} */ let checkpoint=null;
   /** @type {Record<string,any>|undefined} */ let originalRequest;
   /** @type {number|undefined} */ let originalDeadline;
   /** @param {string} status @param {string} reasonCode @param {string} effectState */
   const result=(status,reasonCode,effectState)=>withClientResultRecovery(({schemaVersion:'agent-browser.routine-result.v1',routine:'login',method:'totp',contract,taskId:options.taskId,guideId:options.guideId,status,reasonCode,effectState,...(options.originalRequestId?{originalRequestId:options.originalRequestId}:{}),...(checkpoint?{requestId:checkpoint.requestId,actionId:checkpoint.requestId,continuationId:checkpoint.continuationId,...checkpoint.receiving}:{})}),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:'result'});
   /** @param {Record<string,any>} value */
   const sameTarget=value=>{const current=checkpoint;return !!current&&receivingFields.every(field=>value?.[field]===current.receiving[field]);};
   /** @param {Record<string,any>} value */
   const bound=value=>!!checkpoint&&value?.requestId===checkpoint.requestId&&value.taskId===auth.callerTaskId&&value.sourceId==='mfa'&&value.recordId===checkpoint.recordId&&value.origin===options.origin&&value.targetId===options.targetId&&value.mode==='private'&&value.operation==='login'&&value.credentialType==='otp'&&value.expiresAt===checkpoint.expiresAt&&(originalRequest?value.continuationOf===originalRequest.requestId&&value.factorAccountId===originalRequest.recordId&&['organization','group','delegation'].every(field=>value[field]===originalRequest?.[field])&&value.authenticationBinding===undefined:['continuationOf','factorAccountId'].every(field=>value[field]===undefined)&&value.authenticationBinding?.continuationId===checkpoint.continuationId&&value.authenticationBinding?.method==='totp'&&value.authenticationBinding?.generation===checkpoint.receiving.generation)&&(checkpoint.authenticationBinding===undefined||JSON.stringify(value.authenticationBinding)===JSON.stringify(checkpoint.authenticationBinding));
   const verify=async()=>{
    let observed;try{observed=await browser.verifyLogin(options);}catch{return result('waiting','receiving_unavailable','unknown');}
    if(!sameTarget(observed))return result('adaptation_required','task_target_changed','unknown');
    return result(observed.status,observed.reasonCode,observed.status==='completed'?'verified':'unknown');
   };
   try{
    checkpoint=await checkpoints.load(key);
    if(checkpoint&&(checkpoint.contract!==contract||checkpoint.intent!==intent))throw withClientErrorRecovery(Error('checkpoint_binding_mismatch'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'checkpoint_binding_mismatch');
    if(checkpoint){const current=checkpoint;if(receivingFields.some(field=>options[field]!==undefined&&options[field]!==current.receiving[field])||options.recordId!==undefined&&current.recordId!==undefined&&options.recordId!==current.recordId)throw withClientErrorRecovery(Error('checkpoint_binding_mismatch'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'checkpoint_binding_mismatch');}
    if(options.originalRequestId){
     let original;try{original=await auth.helper('inspect_credential_request',{requestId:options.originalRequestId});}catch(error){return withClientResultRecovery(await (result('waiting',errorCode(error),checkpoint?.useIntent?'unknown':'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance",ownerError:error});}
     if(original.requestId!==options.originalRequestId||original.taskId!==auth.callerTaskId||original.origin!==options.origin||original.targetId!==options.targetId||original.operation!=='login'||original.mode!=='private'||!original.attemptId||['denied','revoked','cancelled','expired'].includes(original.state))return withClientResultRecovery(await (result('waiting','original_request_binding_mismatch',checkpoint?.useIntent?'unknown':'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     if(!nonempty(original.account)||!Number.isSafeInteger(original.expiresAt))return withClientResultRecovery(await (result('waiting','original_account_unavailable',checkpoint?.useIntent?'unknown':'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     originalRequest=original;originalDeadline=original.expiresAt;
    }
    if(!checkpoint){
     let observed;try{observed=await browser.verifyLogin(options);}catch{return withClientResultRecovery(await (result('waiting','receiving_unavailable','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});}
     if(receivingFields.some(field=>!nonempty(observed[field])||options[field]!==undefined&&options[field]!==observed[field]))return withClientResultRecovery(await (result('adaptation_required','receiving_binding_required','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     if(observed.status==='completed')return withClientResultRecovery(await (result('completed',observed.reasonCode,'verified')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     let preparation;try{preparation=await browser.prepareLogin(options);}catch{return withClientResultRecovery(await (result('waiting','receiving_unavailable','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});}
     if(!preparation.plan)return withClientResultRecovery(await (result(preparation.status,preparation.reasonCode,'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     let need;try{need=await auth.helper('observe_authentication',{origin:options.origin,targetId:options.targetId,...(originalRequest?{originalRequestId:originalRequest.requestId}:{})});}catch(error){return withClientResultRecovery(await (result('waiting',errorCode(error),'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance",ownerError:error});}
     if(need.method!=='totp')return withClientResultRecovery(await (result('waiting',need.method?'authentication_method_unsupported':need.heldFor??'authentication_observation_unavailable','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     if(need.state!=='authentication_required'||need.nextAction?.operation!=='continue_authentication'||!nonempty(need.continuationId)||need.nextAction.args?.continuationId!==need.continuationId||!Number.isSafeInteger(need.expiresAt)||need.expiresAt>Math.min(options.expiresAt,originalDeadline??options.expiresAt))return withClientResultRecovery(await (result('waiting',need.heldFor??'authentication_continuation_unavailable','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     const candidates=Array.isArray(need.candidates)?need.candidates.filter(candidate=>nonempty(candidate.recordId)).map(candidate=>({recordId:candidate.recordId,
      // Preserve only bounded choice labels, never arbitrary producer fields.
      ...Object.fromEntries(['sourceId','issuer','account','name'].filter(field=>typeof candidate[field]==='string'&&candidate[field].length<=512&&!/[\x00-\x1f\x7f]/.test(candidate[field])).map(field=>[field,candidate[field]])),
      ...(['totp','hotp'].includes(candidate.method)?{method:candidate.method}:{}),...(candidate.staged===true?{staged:true}:{})})):[];
     if(!candidates.length)return withClientResultRecovery(await (result('waiting','credential_unavailable','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     checkpoint={contract,intent,receiving:Object.fromEntries(receivingFields.map(field=>[field,observed[field]])),preparedPlan:preparation.plan,continuationId:need.continuationId,expiresAt:need.expiresAt,candidates};
     await checkpoints.save(key,checkpoint);
    }
    if(!checkpoint.recordId){
     const selected=options.recordId??(checkpoint.candidates.length===1?checkpoint.candidates[0].recordId:undefined);
     if(!selected)return withClientResultRecovery(await ({...result('waiting','factor_choice_required','no_effect'),candidates:checkpoint.candidates}),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     if(!checkpoint.candidates.some(candidate=>candidate.recordId===selected))return withClientResultRecovery(await (result('waiting','factor_choice_invalid','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
     checkpoint={...checkpoint,recordId:selected};await checkpoints.save(key,checkpoint);
    }
    if(!checkpoint.requestId){
     let requested;try{requested=await auth.helper('continue_authentication',{continuationId:checkpoint.continuationId,idempotencyKey:options.idempotencyKey,purpose:options.purpose,recordId:checkpoint.recordId});}catch(error){return withClientResultRecovery(await (result('waiting',errorCode(error),'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance",ownerError:error});}
     checkpoint={...checkpoint,requestId:requested.requestId};if(!bound(requested))throw withClientErrorRecovery(Error('owner_factor_binding_mismatch'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_factor_binding_mismatch');
     checkpoint={...checkpoint,...(requested.authenticationBinding?{authenticationBinding:structuredClone(requested.authenticationBinding)}:{})};await checkpoints.save(key,checkpoint);
    }
    const requestId=checkpoint.requestId;if(!requestId)throw withClientErrorRecovery(Error('owner_factor_binding_mismatch'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_factor_binding_mismatch');
    let request;try{request=await auth.helper('inspect_credential_request',{requestId});}catch(error){return withClientResultRecovery(await (result('waiting',errorCode(error),checkpoint.useIntent?'unknown':'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance",ownerError:error});}
    if(!bound(request))throw withClientErrorRecovery(Error('owner_factor_binding_mismatch'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_factor_binding_mismatch');
    if(['denied','revoked','cancelled','expired','unavailable'].includes(request.state))return withClientResultRecovery(await (result('failed',`auth_${request.state}`,request.attemptId?'unknown':'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
    if(request.attemptId||request.state==='unknown'||request.authentication!==undefined){
     if(request.authentication!=='verified'){try{request=await auth.helper('reconcile_credential_request',{requestId});}catch{return withClientResultRecovery(await (result('waiting','auth_reconciliation_unavailable','unknown')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});}if(!bound(request))throw withClientErrorRecovery(Error('owner_factor_binding_mismatch'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_factor_binding_mismatch');}
     return withClientResultRecovery(await (request.authentication==='verified'?verify():result(request.authentication==='rejected'?'failed':'waiting',request.authentication==='rejected'?'authentication_rejected':'factor_outcome_unknown','unknown')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
    }
    if(checkpoint.useIntent||checkpoint.outcomeUnknown)return withClientResultRecovery(await (result('waiting','factor_outcome_unknown','unknown')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
    if(request.state==='pending'){try{request=await auth.helper('receive_credential_approval',{requestId});}catch(error){return withClientResultRecovery(await (result('waiting',errorCode(error),'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance",ownerError:error});}if(!bound(request))throw withClientErrorRecovery(Error('owner_factor_binding_mismatch'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_factor_binding_mismatch');}
    if(request.state==='pending')return withClientResultRecovery(await (result('waiting','approval_required','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
    if(request.state!=='approved')return withClientResultRecovery(await (result('failed',`auth_${request.state}`,'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
    let validation;try{validation=await browser.validatePlan({...options,plan:checkpoint.preparedPlan});}catch{return withClientResultRecovery(await (result('waiting','receiving_unavailable','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});}
    if(!sameTarget(validation))return withClientResultRecovery(await (result('adaptation_required','task_target_changed','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
    if(validation.status!=='completed')return withClientResultRecovery(await (result(validation.status,validation.reasonCode,'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
    checkpoint={...checkpoint,useIntent:true};await checkpoints.save(key,checkpoint);
    try{request=await auth.helper('use_credential',{requestId});}catch(error){checkpoint={...checkpoint,outcomeUnknown:true};await checkpoints.save(key,checkpoint);return withClientResultRecovery(await (result('waiting',errorCode(error),'unknown')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance",ownerError:error});}
    if(!bound(request))throw withClientErrorRecovery(Error('owner_factor_binding_mismatch'),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"},'owner_factor_binding_mismatch');
    if(request.heldFor==='notification'&&!request.attemptId){checkpoint={...checkpoint,useIntent:false};await checkpoints.save(key,checkpoint);return withClientResultRecovery(await (result('waiting','notification_required','no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});}
    if(['denied','revoked','cancelled','expired','unavailable'].includes(request.state))return withClientResultRecovery(await (result('failed',`auth_${request.state}`,request.attemptId?'unknown':'no_effect')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
    return withClientResultRecovery(await (request.authentication==='verified'?verify():result(request.authentication==='rejected'?'failed':'waiting',request.authentication==='rejected'?'authentication_rejected':'factor_outcome_unknown','unknown')),{source:"packages/client/src/owner-totp-continuation.js",owningFunction:"createOwnerTotpContinuation.advance"});
   }finally{busy.delete(key);}
  }
 };
}
