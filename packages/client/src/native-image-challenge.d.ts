import type {NativeChallengePolicySource,ChallengeVerificationContext,ChallengeVerification,ChallengeBinding} from './challenge-assistance.js';
export interface NativeImageBounds {x:number;y:number;width:number;height:number;}
export interface NativeImageObservation {
  captureScope?:'complete_window'|'complete_puzzle_region';
  desktopId:string;generation:number;windowId:number;processDigest:string;
  window:NativeImageBounds;displayWidth:number;displayHeight:number;region:NativeImageBounds;
  frameSha256:string;nativeControlEpoch:number;observedAtMs:number;registeredActionId:'hcaptcha-image-grid-3x3-v1'|'hcaptcha-image-grid-3x3-two-round-v1';
  tileImageSha256s?:string[];
  puzzleSha256:string;candidates:Array<{candidateId:string;bounds:NativeImageBounds}>;verify:NativeImageBounds;
}
export interface NativeImageSelectionContext {
  taskId:string;binding:ChallengeBinding;requestId:string;observationRequestId:string;
  observation:NativeImageObservation;images:Array<{type:'image';mimeType:string;data:string}>;
  deadlineAtMs:number;roundIndex:number;signal:AbortSignal;
}
/** Untrusted proposed IDs. The owner reopens its own observation and policy. */
export interface NativeImageSelection {
  taskId:string;observationRequestId:string;frameSha256:string;puzzleSha256:string;selectedCandidateIds:string[];
}
export interface NativeImageOptions {
  callTool:(name:string,args:Record<string,unknown>)=>Promise<{isError?:boolean;content:Array<{type:string;text?:string;mimeType?:string;data?:string}>}>;
  selectCandidates:(context:NativeImageSelectionContext)=>Promise<NativeImageSelection>;
  resolvePolicy:(context:{taskId:string;binding:ChallengeBinding;frameSha256:string;classification:Record<string,unknown>})=>Promise<NativeChallengePolicySource>;
  verifyClearance:(context:ChallengeVerificationContext)=>Promise<ChallengeVerification>;
  verifyAccess:(context:ChallengeVerificationContext)=>Promise<ChallengeVerification>;
  captureRegion?:NativeImageBounds;
  now?:()=>number;
}
export interface NativeImageResult {
  taskId:string;status:'verified_access_after_native_image'|'awaiting_operator'|'intervention';reason?:string;
  originalOperation:Record<string,unknown>|null;roundOperations?:Array<Record<string,unknown>>;retryAllowed:false;
  binding?:ChallengeBinding;verification?:{clearance:string;access:string;frameSha256:string};continuation?:Record<string,unknown>;
}
export interface NativeImageClient {resolve(input:{taskId:string;requestId:string}):Promise<NativeImageResult>;}
export function createNativeImageChallengeClient(options:NativeImageOptions):NativeImageClient;

/** Opt-in whole-grid refresh only; two rounds share one attempt and original deadline. */
export function createNativeImageRoundsChallengeClient(options:NativeImageOptions):NativeImageClient;
