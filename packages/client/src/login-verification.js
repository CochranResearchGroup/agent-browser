import {withClientResultRecovery} from './client-recovery.js';
import {withClientErrorRecovery} from './client-recovery.js';
import { callClientOwner, clientRecoveryFor } from './client-recovery.js';
import { clientArgumentError } from './client-argument-error.js';
// @ts-check

/** Verify application access through the same independent readback seam used by
 * native saved login. This verifier never creates requests or submits input.
 * @param {import('./login-verification.d.ts').LoginVerifierOptions} options
 * @returns {import('./login-verification.d.ts').LoginVerifier}
 */
export function createLoginVerifier({client,verifyAccess,callerTaskId}) {
  if(typeof client?.call!=='function'||typeof verifyAccess!=='function'||!reference(callerTaskId))throw clientArgumentError('login_verifier_configuration_required', "packages/client/src/login-verification.js", "createLoginVerifier", ["client", "verifyAccess", "callerTaskId"]);
  return {async verify(input) {
    if(!reference(input?.requestId)||!reference(input.attemptId)||!bindingFields.every(key=>reference(input.binding?.[key]))||!origin(input.binding?.origin)||!origin(input.applicationOrigin)||!identity(input.expectedAccount)||input.expectedOrganization!==undefined&&!identity(input.expectedOrganization))throw clientArgumentError('login_verification_binding_required', "packages/client/src/login-verification.js", "createLoginVerifier.verify", ["requestId", "attemptId", "binding", "applicationOrigin", "expectedAccount", "expectedOrganization"]);
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
    let dependencyComponent='auth_owner';
    try {
      const before=structuredClone(await callClientOwner(client,'inspect_credential_request',args,{source:"packages/client/src/login-verification.js",owningFunction:"createLoginVerifier.verify"}));
      if(!matches(before))return withClientResultRecovery(await ({...result,category:'request_binding_unverified'}),{source:"packages/client/src/login-verification.js",owningFunction:"verify"});
      dependencyComponent='application_verifier';
      const proof=structuredClone(await verifyAccess(context));
      if(proof?.state!=='verified'||proof.accountMatched!==true||context.expectedOrganization!==undefined&&proof.organizationMatched!==true||proof.applicationOrigin!==context.applicationOrigin||!bindingFields.every(key=>proof[key]===context.binding[key])||!reference(proof.evidenceRef))return withClientResultRecovery(await (result),{source:"packages/client/src/login-verification.js",owningFunction:"verify"});
      result.qualification.applicationAccess='verified';result.evidenceReferences=[proof.evidenceRef];
      dependencyComponent='auth_owner';
      const record=await callClientOwner(client,'inspect_authentication_completion',args,{source:"packages/client/src/login-verification.js",owningFunction:"createLoginVerifier.verify"}),completion=record?.completion;
      if(record?.requestId!==context.requestId||record.state!=='completed'||record.conflict||completion?.outcome!=='succeeded'||completion.evidenceAuthority!=='caller_observed'||completion.actor?.taskId!==callerTaskId||!reference(before.clientId)||completion.actor.clientId!==before.clientId||completion.observation?.applicationOrigin!==context.applicationOrigin||!identity(completion.observation.accountEvidence)||!reference(completion.returnId)||!Array.isArray(completion.observation.evidenceRefs)||!completion.observation.evidenceRefs.includes(proof.evidenceRef))return withClientResultRecovery(await ({...result,category:'completion_unverified'}),{source:"packages/client/src/login-verification.js",owningFunction:"verify"});
      const after=await callClientOwner(client,'inspect_credential_request',args,{source:"packages/client/src/login-verification.js",owningFunction:"createLoginVerifier.verify"});
      if(!matches(after)||after.state!=='completed')return withClientResultRecovery(await ({...result,category:'request_binding_unverified'}),{source:"packages/client/src/login-verification.js",owningFunction:"verify"});
      result.qualification.authCompletion='caller_observed';
      result.receivingSettlement=['settled','held'].includes(record.receivingSettlement?.status)?record.receivingSettlement.status:'unverified';
      return withClientResultRecovery(await ({...result,outcome:'verified',category:'verified_access',nextAction:null,evidenceReferences:[proof.evidenceRef,completion.returnId]}),{source:"packages/client/src/login-verification.js",owningFunction:"verify"});
    } catch(error) {
      const code=typeof error?.code==='string'&&/^[a-z][a-z0-9_]{0,127}$/.test(error.code)?error.code:'client_application_verification_unavailable';
      return withClientResultRecovery(await ({...result,category:'verification_unavailable',dependencyFailure:{component:dependencyComponent,code,...(typeof error?.failure?.occurrenceId==='string'?{occurrenceId:error.failure.occurrenceId}:{})},...(error?.failure?{failure:error.failure}:{}),clientRecovery:error?.clientRecovery??clientRecoveryFor('client_application_verification_unavailable',{source:'packages/client/src/login-verification.js',owningFunction:'createLoginVerifier.verify',requestId:context.requestId})}),{source:"packages/client/src/login-verification.js",owningFunction:"verify"});
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
