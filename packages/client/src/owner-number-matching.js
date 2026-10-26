// @ts-check
/** @typedef {import('./owner-number-matching.d.ts').OwnerNumberMatchingOptions} Options */
/** @typedef {import('./owner-number-matching.d.ts').OwnerNumberMatchingDependencies} Dependencies */
/** @typedef {import('./owner-number-matching.d.ts').OwnerNumberMatchingCheckpoint} Checkpoint */
const contract='auth-receipts-number-matching-v1';
const receivingFields=/** @type {const} */ (['taskId','profileId','browserId','tabId','generation']);
const intentFields=/** @type {const} */ (['taskId','guideId','accountRef','organizationRef','query','accountHint','origin','targetId','purpose','expiresAt','idempotencyKey']);
const optionFields=new Set([...intentFields,...receivingFields,'requestId']);
/** @param {unknown} value */
const text=value=>typeof value==='string'&&value.length>0;
/** @param {unknown} error */
const code=error=>error&&typeof error==='object'&&'code' in error&&typeof error.code==='string'?error.code:'auth_unavailable';
/** @param {Record<string,any>|undefined} challenge */
function publicChallenge(challenge){
 if(!challenge)return undefined;
 return Object.fromEntries(['challengeId','requestId','status','provider','account','direction','number','expiresAt','parentAccepted','textDelivered','screenshotDelivered','heldFor','responseState'].filter(key=>challenge[key]!==undefined).map(key=>[key,challenge[key]]));
}

/** Advance only the original Auth private attempt through supported challenge
 * observation and receipt readback. Auth owns delivery, owner reverse input and
 * durable uncertainty; guarded SiteGuide identity readback owns completion.
 * @param {Dependencies} dependencies
 */
export function createOwnerNumberMatchingContinuation({auth,browser,checkpoints}){
 if(!text(auth?.callerTaskId)||typeof auth.helper!=='function'||typeof auth.followLogin!=='function'||typeof browser?.verifyLogin!=='function'||typeof checkpoints?.load!=='function'||typeof checkpoints.save!=='function')throw new TypeError('owner_number_matching_dependencies_required');
 const busy=new Set();
 return {
  /** Observe and reconcile once. Neither refresh nor timeout authorizes private
   * input, replacement requests or cancellation of the existing authority.
   * @param {Options} options
   */
  async advance(options){
   if(!options||Object.keys(options).some(key=>!optionFields.has(key))||!text(options.requestId)||intentFields.filter(key=>key!=='organizationRef').some(key=>key==='expiresAt'?!Number.isSafeInteger(options[key]):!text(options[key])))throw Error('owner_number_matching_options_invalid');
   const key=`number-matching:${options.taskId}/${options.guideId}`;
   if(busy.has(key))throw Error('owner_number_matching_busy');busy.add(key);
   /** @type {Checkpoint|null} */ let checkpoint=null;
   /** @type {Record<string,any>|undefined} */ let challenge;
   /** @param {string} status @param {string} reasonCode @param {string} [effectState] */
   const result=(status,reasonCode,effectState='unknown')=>({schemaVersion:'agent-browser.routine-result.v1',routine:'login',contract,taskId:options.taskId,guideId:options.guideId,requestId:options.requestId,actionId:options.requestId,status,reasonCode,effectState,...(checkpoint?.receiving??{}),...(challenge?{challenge:publicChallenge(challenge)}:{})});
   try{
    const owner=await checkpoints.load(`owner:${options.taskId}/${options.guideId}`);
    if(!owner||owner.contract!=='auth-receipts-owner-v1'||owner.requestId!==options.requestId)return result('waiting','owner_checkpoint_required');
    let intent;try{intent=JSON.parse(owner.intent);}catch{throw Error('checkpoint_binding_mismatch');}
    if(intent.callerTaskId!==auth.callerTaskId||intentFields.some(key=>intent[key]!==options[key])||receivingFields.some(key=>!text(owner.receiving[key])||options[key]!==undefined&&options[key]!==owner.receiving[key]))throw Error('checkpoint_binding_mismatch');
    const bound=/** @param {Record<string,any>} value */ value=>value?.requestId===options.requestId&&value.taskId===auth.callerTaskId&&value.sourceId===owner.selected.sourceId&&value.recordId===owner.selected.recordId&&value.origin===options.origin&&value.targetId===options.targetId&&value.expiresAt===options.expiresAt&&value.operation==='login'&&value.mode==='private';
    let request;try{request=await auth.helper('inspect_credential_request',{requestId:options.requestId});}catch(error){return result('waiting',code(error));}
    if(!bound(request))throw Error('owner_request_binding_mismatch');
    if(!request.attemptId)return result('waiting','private_attempt_required','no_effect');
    const saved=await checkpoints.load(key);
    if(saved&&(saved.contract!==contract||saved.requestId!==options.requestId||saved.attemptId!==request.attemptId||saved.ownerIntent!==owner.intent||receivingFields.some(field=>saved.receiving[field]!==owner.receiving[field])))throw Error('checkpoint_binding_mismatch');
    checkpoint={contract,ownerIntent:owner.intent,requestId:options.requestId,attemptId:request.attemptId,receiving:{...owner.receiving}};
    await checkpoints.save(key,checkpoint);
    const sameTarget=/** @param {Record<string,any>} value */ value=>receivingFields.every(field=>value?.[field]===owner.receiving[field]);
    let observed;try{observed=await browser.verifyLogin(options);}catch{return result('waiting','receiving_unavailable');}
    if(!sameTarget(observed))return result('adaptation_required','task_target_changed');
    let follow;try{follow=await auth.followLogin({requestId:options.requestId,intervalMs:1,maxObservations:1});}catch(error){return result('waiting',code(error));}
    request=follow.request;if(!bound(request)||request.attemptId!==checkpoint.attemptId)throw Error('owner_request_binding_mismatch');
    challenge=follow.challenge;
    if(!challenge&&['denied','revoked','cancelled','expired'].includes(request.state)){
     try{challenge=await auth.helper('refresh_credential_challenge',{requestId:options.requestId});}catch{return result('waiting','challenge_reconciliation_unavailable');}
    }
    if(challenge?.requestId!==undefined&&challenge.requestId!==options.requestId)throw Error('owner_request_binding_mismatch');
    if(challenge?.challengeId){checkpoint={...checkpoint,challengeId:challenge.challengeId};await checkpoints.save(key,checkpoint);}
    if(challenge?.heldFor==='cancellation')return result('waiting','number_matching_cancellation_required');
    if(['denied','revoked','cancelled','expired'].includes(request.state))return result('failed',`auth_${request.state}`);
    if(request.authentication!==undefined&&request.authentication!=='verified')return result('failed','authentication_failed');
    if(!challenge?.challengeId)return result('waiting',`number_matching_${follow.heldFor??challenge?.heldFor??'unavailable'}`);
    if(challenge.account!==options.accountHint)return result('adaptation_required','challenge_account_mismatch');
    if(!['browser_to_device','device_to_browser'].includes(challenge.direction))return result('waiting','number_matching_unsupported');
    if(['cancelled','expired','revoked','navigated','unavailable','superseded'].includes(challenge.status))return result('failed',`number_matching_${challenge.status}`);
    if(challenge.textDelivered!==true||challenge.screenshotDelivered!==true)return result('waiting','number_matching_delivery_required');
    if(request.authentication==='verified'){
     try{observed=await browser.verifyLogin(options);}catch{return result('waiting','receiving_unavailable');}
     if(!sameTarget(observed))return result('adaptation_required','task_target_changed');
     return result(observed.status,observed.reasonCode,observed.status==='completed'?'verified':'unknown');
    }
    if(challenge.responseState==='unknown')return result('waiting','number_matching_response_unknown');
    return result('waiting',challenge.responseState==='submitted'?'authentication_required':'number_matching_owner_required');
   }finally{busy.delete(key);}
  }
 };
}
