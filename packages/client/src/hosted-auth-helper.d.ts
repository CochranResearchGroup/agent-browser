export type AuthOperationContext={
  operationId:string;
  recoveryId:string;
  binding:{taskId:string;browserId:string;profileId:string;tabId:string;origin:string;controlGeneration:string};
};
export interface OwnerAuthHelper {
  callerTaskId:string;
  installation:{product:'auth-receipts';contractVersion:1;identity:string;sourceRevision:string};
  requestLogin(options:{idempotencyKey:string;sourceId:string;recordId:string;origin:string;targetId:string;purpose:string;expiresAt:number}):Promise<Record<string,any>>;
  advanceLogin(options:{requestId:string}):Promise<Record<string,any>>;
  /** Follow original private custody; no credential input or legacy fallback. */
  followLogin(options:{requestId:string;intervalMs?:number;maxObservations?:number;signal?:AbortSignal;onUpdate?:(summary:Record<string,unknown>)=>void|Promise<void>}):Promise<{
    request?:Record<string,any>;
    challenge?:Record<string,any>;
    /** Read-only factor observation bound by Auth to the original login. */
    authentication?:Record<string,unknown>;
    followState:'completed'|'failed'|'held'|'timed_out'|'aborted';
    heldFor?:string;
  }>;
  helper(operation:string,args:Record<string,unknown>):Promise<Record<string,any>>;
}
/** Connect the genuine current Codex caller to the verified standalone Auth SDK. */
export function connectHostedAuthHelper(options?:{installationRoot?:string;discoveryFile?:string}):Promise<OwnerAuthHelper>;
/** Connect an explicitly enrolled service consumer. Auth validates its exact
 * operation/recovery/browser binding. No Codex caller or discovery fallback. */
export function connectEnrolledAuthHelper(options:{installationRoot?:string;discoveryFile:string;operationContext:AuthOperationContext}):Promise<OwnerAuthHelper>;
