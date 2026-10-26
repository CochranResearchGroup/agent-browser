import type {RoutineResult, VerifyLoginOptions} from './playbooks.js';
export interface LoginOptions extends VerifyLoginOptions {
  scope: Record<string, any> & {person:string;tenant:string;account:string;origin:string;source:string;capabilities:['login']};
  expiresAt:number;
  /** Attested ordinary Codex work, supplied by the hosted helper adapter. */
  callerTaskId?:string;
  /** Read back the existing hosted request without creating or repeating input. */
  reconcile?:boolean;
  /** Explicitly reviewed replacement, checked against fresh same-task evidence. */
  reviewedPlan?:Record<string, unknown>;
  authenticationMethod?:'password'|'extension'|'passkey';
}
export interface Checkpoint {
  binding:string;
  requestId:string;
  attempted:boolean;
  callerTaskId?:string;
  receivingTarget?:{provider:'agent-browser';taskId:string;profileId:string;browserId:string;tabId:string};
  profileId:string;
  browserId:string;
  tabId:string;
  knownTabIds?:string[];
  preparedPlan?:Record<string, unknown>;
  [field:string]:unknown;
}
export interface Dependencies {
  browser:{verifyLogin(options:VerifyLoginOptions):Promise<RoutineResult & Record<string, any>>;
    prepareLogin(options:VerifyLoginOptions):Promise<RoutineResult & Record<string, any>>;
    validatePlan(options:VerifyLoginOptions & {plan:Record<string, unknown>}):Promise<RoutineResult & Record<string, any>>;
    listTabs?(taskId:string):Promise<{tabs:Array<{tabId:string}>}>;
    selectTab?(taskId:string,tabId:string):Promise<unknown>};
  helper(operation:'request_login'|'inspect_request'|'authenticate'|'begin_authentication'|'reconcile_authentication',args:Record<string, unknown>):Promise<Record<string, any>>;
  checkpoints:{load(key:string):Promise<Checkpoint|null>;save(key:string,value:Checkpoint):Promise<void>};
}
export function createLoginPlaybook(dependencies:Dependencies): {
  advance(options:LoginOptions):Promise<Record<string, any>>;
};
export {createOwnerLoginPlaybook} from './owner-login-playbook.js';
export type {OwnerLoginOptions,OwnerLoginDependencies,OwnerLoginCheckpoint} from './owner-login-playbook.js';

export {createOwnerTotpContinuation} from './owner-totp-continuation.js';
export {createOwnerNumberMatchingContinuation} from './owner-number-matching.js';

export type {BrowserActionEvidence} from './owner-login-playbook.js';
