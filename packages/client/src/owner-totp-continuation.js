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
 if(!auth||typeof auth.helper!=='function'||!nonempty(auth.callerTaskId)||!browser||(/** @type {const} */ (['verifyLogin','prepareLogin','validatePlan'])).some(method=>typeof browser[method]!=='function')||!checkpoints?.load||!checkpoints?.save)throw new TypeError('owner_totp_dependencies_required');
 const busy=new Set();
 return {
  /** Request no password and expose no factor value. The optional recordId must
   * select an Auth-returned candidate before a factor request has been created.
   * Auth binds account-less observations to the original password request and
   * creates the factor request through its observed continuation action.
   * @param {OwnerTotpOptions} options
   */
  async advance(options){
   if(!options||intentFields.filter(field=>!['organizationRef','originalRequestId'].includes(field)).some(field=>field==='expiresAt'?!Number.isSafeInteger(options[field]):!nonempty(options[field]))||(/** @type {const} */ (['organizationRef','originalRequestId','recordId'])).some(field=>options[field]!==undefined&&!nonempty(options[field])))throw Error('owner_totp_options_invalid');
   try{if(new URL(options.origin).origin!==options.origin||!options.origin.startsWith('https://'))throw 0;}catch{throw Error('owner_totp_origin_invalid');}
   const key=`totp:${options.taskId}/${options.guideId}`;
   if(busy.has(key))throw Error('owner_totp_busy');busy.add(key);
   const intent=JSON.stringify({...Object.fromEntries(intentFields.map(field=>[field,options[field]])),callerTaskId:auth.callerTaskId});
   /** @type {OwnerTotpCheckpoint|null} */ let checkpoint=null;
   /** @type {Record<string,any>|undefined} */ let originalRequest;
   /** @type {number|undefined} */ let originalDeadline;
   /** @param {string} status @param {string} reasonCode @param {string} effectState */
   const result=(status,reasonCode,effectState)=>({schemaVersion:'agent-browser.routine-result.v1',routine:'login',method:'totp',contract,taskId:options.taskId,guideId:options.guideId,status,reasonCode,effectState,...(options.originalRequestId?{originalRequestId:options.originalRequestId}:{}),...(checkpoint?{requestId:checkpoint.requestId,actionId:checkpoint.requestId,continuationId:checkpoint.continuationId,...checkpoint.receiving}:{})});
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
    if(checkpoint&&(checkpoint.contract!==contract||checkpoint.intent!==intent))throw Error('checkpoint_binding_mismatch');
    if(checkpoint){const current=checkpoint;if(receivingFields.some(field=>options[field]!==undefined&&options[field]!==current.receiving[field])||options.recordId!==undefined&&current.recordId!==undefined&&options.recordId!==current.recordId)throw Error('checkpoint_binding_mismatch');}
    if(options.originalRequestId){
     let original;try{original=await auth.helper('inspect_credential_request',{requestId:options.originalRequestId});}catch(error){return result('waiting',errorCode(error),checkpoint?.useIntent?'unknown':'no_effect');}
     if(original.requestId!==options.originalRequestId||original.taskId!==auth.callerTaskId||original.origin!==options.origin||original.targetId!==options.targetId||original.operation!=='login'||original.mode!=='private'||!original.attemptId||['denied','revoked','cancelled','expired'].includes(original.state))return result('waiting','original_request_binding_mismatch',checkpoint?.useIntent?'unknown':'no_effect');
     if(!nonempty(original.account)||!Number.isSafeInteger(original.expiresAt))return result('waiting','original_account_unavailable',checkpoint?.useIntent?'unknown':'no_effect');
     originalRequest=original;originalDeadline=original.expiresAt;
    }
    if(!checkpoint){
     let observed;try{observed=await browser.verifyLogin(options);}catch{return result('waiting','receiving_unavailable','no_effect');}
     if(receivingFields.some(field=>!nonempty(observed[field])||options[field]!==undefined&&options[field]!==observed[field]))return result('adaptation_required','receiving_binding_required','no_effect');
     if(observed.status==='completed')return result('completed',observed.reasonCode,'verified');
     let preparation;try{preparation=await browser.prepareLogin(options);}catch{return result('waiting','receiving_unavailable','no_effect');}
     if(!preparation.plan)return result(preparation.status,preparation.reasonCode,'no_effect');
     let need;try{need=await auth.helper('observe_authentication',{origin:options.origin,targetId:options.targetId,...(originalRequest?{originalRequestId:originalRequest.requestId}:{})});}catch(error){return result('waiting',errorCode(error),'no_effect');}
     if(need.method!=='totp')return result('waiting',need.method?'authentication_method_unsupported':need.heldFor??'authentication_observation_unavailable','no_effect');
     if(need.state!=='authentication_required'||need.nextAction?.operation!=='continue_authentication'||!nonempty(need.continuationId)||need.nextAction.args?.continuationId!==need.continuationId||!Number.isSafeInteger(need.expiresAt)||need.expiresAt>Math.min(options.expiresAt,originalDeadline??options.expiresAt))return result('waiting',need.heldFor??'authentication_continuation_unavailable','no_effect');
     const candidates=Array.isArray(need.candidates)?need.candidates.filter(candidate=>nonempty(candidate.recordId)).map(candidate=>({recordId:candidate.recordId})):[];
     if(!candidates.length)return result('waiting','credential_unavailable','no_effect');
     checkpoint={contract,intent,receiving:Object.fromEntries(receivingFields.map(field=>[field,observed[field]])),preparedPlan:preparation.plan,continuationId:need.continuationId,expiresAt:need.expiresAt,candidates};
     await checkpoints.save(key,checkpoint);
    }
    if(!checkpoint.recordId){
     const selected=options.recordId??(checkpoint.candidates.length===1?checkpoint.candidates[0].recordId:undefined);
     if(!selected)return {...result('waiting','factor_choice_required','no_effect'),candidates:checkpoint.candidates};
     if(!checkpoint.candidates.some(candidate=>candidate.recordId===selected))return result('waiting','factor_choice_invalid','no_effect');
     checkpoint={...checkpoint,recordId:selected};await checkpoints.save(key,checkpoint);
    }
    if(!checkpoint.requestId){
     let requested;try{requested=await auth.helper('continue_authentication',{continuationId:checkpoint.continuationId,idempotencyKey:options.idempotencyKey,purpose:options.purpose,recordId:checkpoint.recordId});}catch(error){return result('waiting',errorCode(error),'no_effect');}
     checkpoint={...checkpoint,requestId:requested.requestId};if(!bound(requested))throw Error('owner_factor_binding_mismatch');
     checkpoint={...checkpoint,...(requested.authenticationBinding?{authenticationBinding:structuredClone(requested.authenticationBinding)}:{})};await checkpoints.save(key,checkpoint);
    }
    const requestId=checkpoint.requestId;if(!requestId)throw Error('owner_factor_binding_mismatch');
    let request;try{request=await auth.helper('inspect_credential_request',{requestId});}catch(error){return result('waiting',errorCode(error),checkpoint.useIntent?'unknown':'no_effect');}
    if(!bound(request))throw Error('owner_factor_binding_mismatch');
    if(['denied','revoked','cancelled','expired','unavailable'].includes(request.state))return result('failed',`auth_${request.state}`,request.attemptId?'unknown':'no_effect');
    if(request.attemptId||request.state==='unknown'||request.authentication!==undefined){
     if(request.authentication!=='verified'){try{request=await auth.helper('reconcile_credential_request',{requestId});}catch{return result('waiting','auth_reconciliation_unavailable','unknown');}if(!bound(request))throw Error('owner_factor_binding_mismatch');}
     return request.authentication==='verified'?verify():result(request.authentication==='rejected'?'failed':'waiting',request.authentication==='rejected'?'authentication_rejected':'factor_outcome_unknown','unknown');
    }
    if(checkpoint.useIntent||checkpoint.outcomeUnknown)return result('waiting','factor_outcome_unknown','unknown');
    if(request.state==='pending'){try{request=await auth.helper('receive_credential_approval',{requestId});}catch(error){return result('waiting',errorCode(error),'no_effect');}if(!bound(request))throw Error('owner_factor_binding_mismatch');}
    if(request.state==='pending')return result('waiting','approval_required','no_effect');
    if(request.state!=='approved')return result('failed',`auth_${request.state}`,'no_effect');
    let validation;try{validation=await browser.validatePlan({...options,plan:checkpoint.preparedPlan});}catch{return result('waiting','receiving_unavailable','no_effect');}
    if(!sameTarget(validation))return result('adaptation_required','task_target_changed','no_effect');
    if(validation.status!=='completed')return result(validation.status,validation.reasonCode,'no_effect');
    checkpoint={...checkpoint,useIntent:true};await checkpoints.save(key,checkpoint);
    try{request=await auth.helper('use_credential',{requestId});}catch(error){checkpoint={...checkpoint,outcomeUnknown:true};await checkpoints.save(key,checkpoint);return result('waiting',errorCode(error),'unknown');}
    if(!bound(request))throw Error('owner_factor_binding_mismatch');
    if(request.heldFor==='notification'&&!request.attemptId){checkpoint={...checkpoint,useIntent:false};await checkpoints.save(key,checkpoint);return result('waiting','notification_required','no_effect');}
    if(['denied','revoked','cancelled','expired','unavailable'].includes(request.state))return result('failed',`auth_${request.state}`,request.attemptId?'unknown':'no_effect');
    return request.authentication==='verified'?verify():result(request.authentication==='rejected'?'failed':'waiting',request.authentication==='rejected'?'authentication_rejected':'factor_outcome_unknown','unknown');
   }finally{busy.delete(key);}
  }
 };
}
