import type {LoginProgress} from './login-progress.js';
import type {LoginVerificationInput,LoginVerificationResult} from './login-verification.js';

export interface UnresolvedLoginAttempt {requestId:string;attemptId:string;}
export interface LoginContinuationCheckpoint {
  schemaVersion:1;
  workflowId:string;
  requestId:string;
  attemptId?:string;
  binding:LoginVerificationInput['binding'];
  requestedOutcome:{applicationOrigin:string;expectedAccount:string;expectedOrganization?:string};
  unresolvedAttempts:UnresolvedLoginAttempt[];
  wakeIntentId?:string;
  progress?:LoginProgress;
  notification?:{key:string;state:'intent'|'callback_acknowledged'|'unknown'};
}
export interface LoginWakeReadback {
  /** Trusted host adapter's normalized subset of its supported Wake receipt. */
  intentId:string;
  state:'registered'|'held'|'cancelled';
  armed?:boolean;
  monitor?:{ready?:boolean};
}
export interface LoginContinuationResult extends LoginProgress {
  workflowId:string;
  unresolvedAttempts:UnresolvedLoginAttempt[];
  wake:{intentId?:string;state:'not_used'|'unverified'|'registered'|'held'|'ready'|'cancelled'};
  wakeCleanupRequired:boolean;
  qualification?:LoginVerificationResult['qualification'];
  receivingSettlement?:LoginVerificationResult['receivingSettlement'];
  notification:{key:string;state:'intent'|'callback_acknowledged'|'unknown'|'not_configured';emitted:boolean};
}
export interface LoginContinuationOptions {
  store:{withEpisode<T>(episodeId:string,run:()=>Promise<T>):Promise<T>;load(episodeId:string):Promise<Record<string,any>|null>;save(record:Record<string,any>):Promise<void>};
  callerTaskId:string;
  inspectRequest(input:{requestId:string}):Promise<Record<string,any>>;
  verify(input:LoginVerificationInput):Promise<LoginVerificationResult>;
  readWake?(intentId:string):Promise<LoginWakeReadback>;
  /** Callback acknowledgement is not proof that a human received a notification. */
  onUpdate?(update:Omit<LoginContinuationResult,'notification'>&{key:string}):Promise<void>;
}
export interface LoginContinuation {resume(input:{episodeId:string}):Promise<LoginContinuationResult>;}
/** Extend an existing host-owned record's loginContinuation namespace only. */
export function createLoginContinuation(options:LoginContinuationOptions):LoginContinuation;
