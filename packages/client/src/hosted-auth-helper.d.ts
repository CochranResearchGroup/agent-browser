/** Connect to the verified standalone Auth Receipts SDK without exposing private bearer material. */
export function connectHostedAuthHelper(options?:{installationRoot?:string;discoveryFile?:string}):Promise<{
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
}>;
