// @ts-check

/** Verify application access through the same independent readback seam used by
 * native saved login. This verifier never creates requests or submits input.
 * @param {import('./login-verification.d.ts').LoginVerifierOptions} options
 * @returns {import('./login-verification.d.ts').LoginVerifier}
 */
export function createLoginVerifier({client,verifyAccess,callerTaskId}) {
  if(typeof client?.call!=='function'||typeof verifyAccess!=='function'||!reference(callerTaskId))throw TypeError('login_verifier_configuration_required');
  return {async verify(input) {
    if(!reference(input?.requestId)||!reference(input.attemptId)||!bindingFields.every(key=>reference(input.binding?.[key]))||!origin(input.binding?.origin)||!origin(input.applicationOrigin)||!identity(input.expectedAccount)||input.expectedOrganization!==undefined&&!identity(input.expectedOrganization))throw TypeError('login_verification_binding_required');
    const context=Object.freeze({requestId:input.requestId,attemptId:input.attemptId,binding:Object.freeze({...input.binding}),applicationOrigin:input.applicationOrigin,expectedAccount:input.expectedAccount,...(input.expectedOrganization===undefined?{}:{expectedOrganization:input.expectedOrganization})});
    /** @type {import('./login-verification.d.ts').LoginVerificationResult} */
    const result={
      schemaVersion:1,outcome:'incomplete',category:'verification_incomplete',
      component:'application',waitingForUser:false,evidenceReferences:[],nextAction:{operation:'inspect_credential_request',args:{requestId:context.requestId}},
      requestId:context.requestId,
      qualification:{applicationAccess:'unverified',authCompletion:'unverified',installation:'unverified',otherLoginPaths:'unverified'},
    };
    const args={requestId:context.requestId};
    const matches=request=>request?.requestId===context.requestId&&request.attemptId===context.attemptId&&request.taskId===callerTaskId&&request.targetId===context.binding.taskId&&request.origin===context.binding.origin&&request.operation==='login'&&request.mode==='private'&&!['denied','revoked','cancelled','expired'].includes(request.state);
    try {
      const before=structuredClone(await client.call('inspect_credential_request',args));
      if(!matches(before))return {...result,category:'request_binding_unverified'};
      const proof=structuredClone(await verifyAccess(context));
      if(proof?.state!=='verified'||proof.accountMatched!==true||context.expectedOrganization!==undefined&&proof.organizationMatched!==true||proof.applicationOrigin!==context.applicationOrigin||!bindingFields.every(key=>proof[key]===context.binding[key])||!reference(proof.evidenceRef))return result;
      result.qualification.applicationAccess='verified';result.evidenceReferences=[proof.evidenceRef];
      const record=await client.call('inspect_authentication_completion',args),completion=record?.completion;
      if(record?.requestId!==context.requestId||record.state!=='completed'||record.conflict||completion?.outcome!=='succeeded'||completion.evidenceAuthority!=='caller_observed'||completion.actor?.taskId!==callerTaskId||!reference(before.clientId)||completion.actor.clientId!==before.clientId||completion.observation?.applicationOrigin!==context.applicationOrigin||!identity(completion.observation.accountEvidence)||!reference(completion.returnId)||!Array.isArray(completion.observation.evidenceRefs)||!completion.observation.evidenceRefs.includes(proof.evidenceRef))return {...result,category:'completion_unverified'};
      const after=await client.call('inspect_credential_request',args);
      if(!matches(after)||after.state!=='completed')return {...result,category:'request_binding_unverified'};
      result.qualification.authCompletion='caller_observed';
      result.receivingSettlement=['settled','held'].includes(record.receivingSettlement?.status)?record.receivingSettlement.status:'unverified';
      return {...result,outcome:'verified',category:'verified_access',nextAction:null,evidenceReferences:[proof.evidenceRef,completion.returnId]};
    } catch {
      return {...result,category:'verification_unavailable'};
    }
  }};
}

const bindingFields=/** @type {const} */(['taskId','profileId','browserId','tabId','targetId']);
/** @param {unknown} value */
const reference=value=>typeof value==='string'&&/^[A-Za-z0-9][A-Za-z0-9:._-]{0,199}$/.test(value);
/** @param {unknown} value */
const identity=value=>typeof value==='string'&&value.trim().length>0&&value.length<=512&&!/[\r\n\x00]/.test(value);
/** @param {unknown} value */
function origin(value) {
  try {const url=new URL(/** @type {string} */(value));return url.protocol==='https:'&&url.origin===value;}catch{return false;}
}
