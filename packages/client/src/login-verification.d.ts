import type {SavedLoginBinding} from './native-saved-login.js';
import type {LoginProgress} from './login-progress.js';

export interface LoginVerificationInput {
  requestId:string;
  attemptId:string;
  binding:SavedLoginBinding & {origin:string};
  applicationOrigin:string;
  expectedAccount:string;
  expectedOrganization?:string;
}
export interface LoginAccessProof extends SavedLoginBinding {
  state:string;
  accountMatched:boolean;
  organizationMatched?:boolean;
  applicationOrigin:string;
  evidenceRef:string;
}
export interface LoginVerificationResult extends LoginProgress {
  qualification:{applicationAccess:'verified'|'unverified';authCompletion:'caller_observed'|'unverified';installation:'unverified';otherLoginPaths:'unverified'};
  receivingSettlement?:'settled'|'held'|'unverified';
}
export interface LoginVerifierOptions {
  client:{call(operation:string,args:Record<string,unknown>):Promise<Record<string,any>>};
  callerTaskId:string;
  /** Independent receiving-application account readback, not a caller success marker. */
  verifyAccess(input:LoginVerificationInput):Promise<LoginAccessProof>;
}
export interface LoginVerifier {verify(input:LoginVerificationInput):Promise<LoginVerificationResult>;}
/** Read-only application and owner-completion verification; no credential effects. */
export function createLoginVerifier(options:LoginVerifierOptions):LoginVerifier;
