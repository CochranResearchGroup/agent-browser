import type {LoginProgress} from './login-progress.js';
export interface FailureObservation {
  requestId:string;idempotencyKey:string;observedAt:number;
  observation:{applicationOrigin:string;accountEvidence:string;evidenceRefs:string[]};
}
export interface LoginRecovery {
  reconcile(options:{requestId:string}):Promise<LoginProgress>;
  recordFailure(options:FailureObservation):Promise<{requestId:string;previousInputOutcome:string;settlement:string;retryAllowed:false}>;
  prepareFresh(options:{previousRequestId:string;intent:Record<string,any>}):Promise<LoginProgress & {previousRequestId:string}>;
}
export function createLoginRecovery(options:{client:{call(operation:string,args:Record<string,any>):Promise<any>}}):LoginRecovery;
