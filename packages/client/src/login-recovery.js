// @ts-check
import {summarizeLoginProgress} from './login-progress.js';

/** Existing Auth operations retain all private-input and recovery authority.
 * This adapter never invokes use_credential or infers no effect.
 * @param {{client:{call(operation:string,args:Record<string,any>):Promise<any>}}} options */
export function createLoginRecovery({client}) {
  const invalid=()=>Object.assign(Error('Original login recovery is unverified'),{code:'login_recovery_unverified'});
  if(typeof client?.call!=='function')throw invalid();
  /** @param {string} requestId */
  const inspect=async requestId=>{
    if(typeof requestId!=='string'||!/^[A-Za-z0-9][A-Za-z0-9:._-]{0,199}$/.test(requestId))throw invalid();
    const result=await client.call('inspect_credential_request',{requestId});
    if(result?.requestId!==requestId)throw invalid();
    return result;
  };
  return {
    /** Read and reconcile the same owner request. No replacement is created.
     * @param {{requestId:string}} options */
    async reconcile({requestId}) {
      const original=await inspect(requestId);
      if(original.running===true)return summarizeLoginProgress(original);
      if(original.state!=='unknown'&&!original.attemptId)return summarizeLoginProgress(original);
      const result=await client.call('reconcile_credential_request',{requestId});
      if(result?.requestId!==requestId||result.attemptId!==original.attemptId)throw invalid();
      return summarizeLoginProgress(result);
    },
    /** Record an observed business failure through Auth's completion contract.
     * Time and observation come from qualified caller evidence, never this host's clock.
     * @param {import('./login-recovery.d.ts').FailureObservation} options */
    async recordFailure(options) {
      const original=await inspect(options.requestId);
      if(original.running!==false)throw invalid();
      const result=await client.call('complete_authentication',{...options,outcome:'failed'});
      if(result?.requestId!==options.requestId||result.conflict||result.completion?.outcome!=='failed')throw invalid();
      const current=await client.call('inspect_authentication_completion',{requestId:options.requestId});
      if(current?.requestId!==options.requestId||current.state!=='completed'||current.completion?.outcome!=='failed')throw invalid();
      return {requestId:options.requestId,previousInputOutcome:current.completion.previousState,
        settlement:current.receivingSettlement?.status==='settled'?'settled':'held',retryAllowed:false};
    },
    /** Ask the owner to prepare a distinct challenge after settled business failure.
     * The owner validates current receiving readiness, permission and request time.
     * Preparation is not credential use; existing owner helpers own the next step.
     * @param {{previousRequestId:string,intent:Record<string,any>}} options */
    async prepareFresh({previousRequestId,intent}) {
      const original=await inspect(previousRequestId);
      const completion=await client.call('inspect_authentication_completion',{requestId:previousRequestId});
      const account=original.recipientAccount ?? original.account;
      if(original.state!=='completed'||original.running!==false||completion?.requestId!==previousRequestId||completion.state!=='completed'||completion.completion?.outcome!=='failed'||completion.receivingSettlement?.status!=='settled'||
        !intent||typeof account!=='string'||!account||intent.accountHint!==account||intent.origin!==original.origin||intent.targetId!==original.targetId||typeof intent.idempotencyKey!=='string'||!intent.idempotencyKey)throw invalid();
      const prepared=await client.call('prepare_credential_login',intent);
      const request=prepared?.request;
      if(typeof request?.requestId!=='string'||!/^[A-Za-z0-9][A-Za-z0-9:._-]{0,199}$/.test(request.requestId)||request.requestId===previousRequestId||request.origin!==original.origin||request.targetId!==original.targetId||(request.recipientAccount??request.account)!==account)throw invalid();
      // Preparation may produce review, denial, or an already attempted original
      // for this idempotency key. Preserve that owner's result, never repeat use.
      return {...summarizeLoginProgress(prepared),previousRequestId};
    },
  };
}
