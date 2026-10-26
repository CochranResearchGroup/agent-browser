// @ts-check
/** @typedef {import('./owner-login-playbook.d.ts').OwnerLoginOptions} OwnerLoginOptions */
/** @typedef {import('./owner-login-playbook.d.ts').OwnerLoginDependencies} OwnerLoginDependencies */
/** @typedef {import('./owner-login-playbook.d.ts').OwnerLoginCheckpoint} OwnerLoginCheckpoint */

const contract='auth-receipts-owner-v1';
const receivingFields=/** @type {const} */ (['taskId','profileId','browserId','tabId','generation']);
const intentFields=/** @type {const} */ (['taskId','guideId','accountRef','organizationRef','query','accountHint','origin','targetId','purpose','expiresAt','idempotencyKey']);
/** @param {unknown} value */
const nonempty=value=>typeof value==='string'&&value.length>0;
/** @param {unknown} error */
const errorCode=error=>error&&typeof error==='object'&&'code' in error&&typeof error.code==='string'?error.code:undefined;

/** Compose owner credential requests with guarded SiteGuide routines. Auth owns
 * all policy, private use and receiver privacy; this client persists metadata
 * before dispatch and independently verifies the intended receiving account.
 * Legacy callback checkpoints are never translated into owner requests.
 * @param {OwnerLoginDependencies} dependencies
 */
export function createOwnerLoginPlaybook({browser,auth,checkpoints}){
 if(!browser||(/** @type {const} */ (['verifyLogin','prepareLogin','validatePlan'])).some(method=>typeof browser[method]!=='function')||!auth||(/** @type {const} */ (['requestLogin','advanceLogin','helper'])).some(method=>typeof auth[method]!=='function')||!nonempty(auth.callerTaskId)||!checkpoints?.load||!checkpoints?.save)throw new TypeError('owner_login_dependencies_required');
 const busy=new Set();
 return {
  /** Advance the original owner request once. Returned request IDs can be used
   * by supported Auth continuation routines; they are never credential values.
   * @param {OwnerLoginOptions} options
   */
  async advance(options){
   if(!options||intentFields.filter(field=>field!=='organizationRef').some(field=>field==='expiresAt'?!Number.isSafeInteger(options[field]):!nonempty(options[field])))throw Error('owner_login_options_invalid');
   if(options.organizationRef!==undefined&&!nonempty(options.organizationRef))throw Error('owner_login_options_invalid');
   try{if(new URL(options.origin).origin!==options.origin||!options.origin.startsWith('https://'))throw 0;}catch{throw Error('owner_login_origin_invalid');}
   const key=`owner:${options.taskId}/${options.guideId}`;
   if(busy.has(key))throw Error('owner_login_busy');busy.add(key);
   const intent=JSON.stringify({...Object.fromEntries(intentFields.map(field=>[field,options[field]])),callerTaskId:auth.callerTaskId});
   /** @type {OwnerLoginCheckpoint|null} */ let checkpoint=null;
   /** @param {string} status @param {string} reasonCode @param {string} effectState */
   const result=(status,reasonCode,effectState)=>({schemaVersion:'agent-browser.routine-result.v1',routine:'login',taskId:options.taskId,guideId:options.guideId,status,reasonCode,effectState,...(checkpoint?{requestId:checkpoint.requestId,actionId:checkpoint.requestId,...checkpoint.receiving}:{}),contract});
   /** @param {Record<string,any>} value */
   const bound=value=>checkpoint&&value?.requestId===checkpoint.requestId&&value.taskId===auth.callerTaskId&&value.sourceId===checkpoint.selected.sourceId&&value.recordId===checkpoint.selected.recordId&&value.origin===options.origin&&value.targetId===options.targetId&&value.expiresAt===options.expiresAt&&value.mode==='private'&&value.operation==='login';
   /** @param {Record<string,any>} value */
   const sameTarget=value=>{const current=checkpoint;return !!current&&receivingFields.every(field=>value?.[field]===current.receiving[field]);};
   const verify=async()=>{
    let observed;try{observed=await browser.verifyLogin(options);}catch{return result('waiting','receiving_unavailable','unknown');}
    if(!sameTarget(observed))return result('adaptation_required','task_target_changed','unknown');
    return {...observed,...result(observed.status,observed.reasonCode,observed.status==='completed'?'verified':'unknown')};
   };
   try{
    checkpoint=await checkpoints.load(key);
    if(checkpoint&&(checkpoint.contract!==contract||checkpoint.intent!==intent))throw Error('checkpoint_binding_mismatch');
    if(checkpoint){const current=checkpoint;if(receivingFields.some(field=>options[field]!==undefined&&options[field]!==current.receiving[field]))throw Error('checkpoint_binding_mismatch');}
    if(!checkpoint){
     if(await checkpoints.load(`${options.taskId}/${options.guideId}`))return result('waiting','legacy_checkpoint_requires_reconciliation','unknown');
     let observed;try{observed=await browser.verifyLogin(options);}catch{return result('waiting','receiving_unavailable','no_effect');}
     if(receivingFields.some(field=>!nonempty(observed[field])||options[field]!==undefined&&options[field]!==observed[field]))return result('adaptation_required','receiving_binding_required','no_effect');
     if(observed.status==='completed')return {...observed,...result('completed',observed.reasonCode,'verified')};
     if(!['identifier_required','password_required','login_required'].includes(observed.reasonCode))return {...observed,...result(observed.status,observed.reasonCode,'no_effect')};
     let preparation;try{preparation=await browser.prepareLogin(options);}catch{return result('waiting','receiving_unavailable','no_effect');}
     if(!preparation.plan)return {...preparation,...result(preparation.status,preparation.reasonCode,'no_effect')};
     let discovery;try{discovery=await auth.helper('discover_credentials',{query:options.query,accountHint:options.accountHint});}catch(error){return result('waiting',errorCode(error)??'auth_unavailable','no_effect');}
     if(discovery.state!=='unique'||!discovery.selected)return result('waiting',discovery.state==='ambiguous'?'account_ambiguous':'credential_unavailable','no_effect');
     const selected=discovery.selected;
     if(selected.origin!==options.origin||!nonempty(selected.sourceId)||!nonempty(selected.recordId))return result('waiting','credential_recipient_mismatch','no_effect');
     checkpoint={contract,intent,receiving:Object.fromEntries(receivingFields.map(field=>[field,observed[field]])),selected:{sourceId:selected.sourceId,recordId:selected.recordId},preparedPlan:preparation.plan};
     await checkpoints.save(key,checkpoint);
    }
    if(!checkpoint.requestId){
     let requested;try{requested=await auth.requestLogin({...checkpoint.selected,idempotencyKey:options.idempotencyKey,origin:options.origin,targetId:options.targetId,purpose:options.purpose,expiresAt:options.expiresAt});}catch(error){return result('waiting',errorCode(error)??'auth_unavailable','no_effect');}
     checkpoint={...checkpoint,requestId:requested.requestId};
     if(!bound(requested))throw Error('owner_request_binding_mismatch');
     await checkpoints.save(key,checkpoint);
    }
    const requestId=checkpoint.requestId;if(!requestId)throw Error('owner_request_binding_mismatch');
    let request;try{request=await auth.helper('inspect_credential_request',{requestId:checkpoint.requestId});}catch(error){return result('waiting',errorCode(error)??'auth_unavailable',checkpoint.advanceIntent?'unknown':'no_effect');}
    if(!bound(request))throw Error('owner_request_binding_mismatch');
    if(['denied','revoked','cancelled','expired'].includes(request.state))return result('failed',`auth_${request.state}`,request.attemptId?'unknown':'no_effect');
    if(request.attemptId||request.state==='unknown'||request.authentication!==undefined){
     if(request.authentication!=='verified'){
      try{request=await auth.helper('reconcile_credential_request',{requestId:checkpoint.requestId});}catch{return result('waiting','auth_reconciliation_unavailable','unknown');}
      if(!bound(request))throw Error('owner_request_binding_mismatch');
     }
     return request.authentication==='verified'?verify():result('waiting','authentication_required','unknown');
    }
    if(checkpoint.outcomeUnknown)return result('waiting','auth_outcome_uncertain','unknown');
    let validation;try{validation=await browser.validatePlan({...options,plan:checkpoint.preparedPlan});}catch{return result('waiting','receiving_unavailable','no_effect');}
    if(!sameTarget(validation))return result('adaptation_required','task_target_changed','no_effect');
    if(validation.status!=='completed')return {...validation,...result(validation.status,validation.reasonCode,'no_effect')};
    checkpoint={...checkpoint,advanceIntent:true};await checkpoints.save(key,checkpoint);
    try{request=await auth.advanceLogin({requestId});}catch(error){checkpoint={...checkpoint,outcomeUnknown:true};await checkpoints.save(key,checkpoint);return result('waiting',errorCode(error)??'auth_outcome_uncertain','unknown');}
    if(!bound(request))throw Error('owner_request_binding_mismatch');
    checkpoint={...checkpoint,advanceIntent:!!request.attemptId};await checkpoints.save(key,checkpoint);
    if(request.authentication==='verified')return verify();
    if(['denied','revoked','cancelled','expired'].includes(request.state))return result('failed',`auth_${request.state}`,request.attemptId?'unknown':'no_effect');
    return result('waiting',request.state==='pending'?'approval_required':request.heldFor==='notification'?'notification_required':request.attemptId?'authentication_required':'auth_not_ready',request.attemptId?'unknown':'no_effect');
   }finally{busy.delete(key);}
  }
 };
}
