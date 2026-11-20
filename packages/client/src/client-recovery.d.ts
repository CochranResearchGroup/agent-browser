export interface ClientRecoveryRead {
  tool?:string;interface?:string;method?:string;arguments?:Record<string,unknown>;
  fields?:string[];kind?:string;contract?:string;precondition?:string;
}
/** Local adapter guidance. The original owner's failure and plan are authoritative. */
export interface ClientRecovery {
  schemaVersion:'agent-browser.client-recovery.v1';coverage:'registered'|'unknown';code:string;
  packet:{code:string;source?:string;owningFunction?:string;taskId?:string;requestId?:string;episodeId?:string;ownerCode?:string};
  requiredActor:string;automaticExecutionAllowed:false;recipe?:string;registrySourceSha256?:string;
  normalization?:{ownerCode:string;ownerCodeCoverage:'not_asserted'};
  evidenceClass?:string;evidenceRequirement?:string;rationale?:string;
  inspect?:ClientRecoveryRead;verification?:ClientRecoveryRead;
  remediation?:string|Array<{instruction:string;precondition:string;interface?:string;call?:ClientRecoveryRead}>;
  resumeCondition?:string;maximumAutomaticAttempts?:number;unboundArguments?:string[];
  bindingState?:'requires_caller_binding'|'arguments_bound';inspection?:string;ownerRecoveryAvailable?:boolean;
}
export function clientRecoveryFor(code:string,context?:Record<string,unknown>):ClientRecovery;
export function clientFailureError(code:string,context?:Record<string,unknown>):Error&{code:string;source?:string;owningFunction?:string;clientRecovery:ClientRecovery};
export function withClientResultRecovery<T>(result:T,context?:Record<string,unknown>):T&{clientRecovery?:ClientRecovery};

export function withClientErrorRecovery<T>(error:T,context?:Record<string,unknown>,localCode?:string):T&{clientRecovery:ClientRecovery};
export function callClientOwner(client:{call:(operation:string,args:any)=>Promise<any>},operation:string,args:any,context:Record<string,unknown>):Promise<any>;
