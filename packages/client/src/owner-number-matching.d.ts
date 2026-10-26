import type {OwnerLoginOptions,OwnerLoginCheckpoint} from './owner-login-playbook.d.ts';

export interface OwnerNumberMatchingOptions extends OwnerLoginOptions {requestId:string}
export interface OwnerNumberMatchingCheckpoint {
 contract:'auth-receipts-number-matching-v1';ownerIntent:string;requestId:string;attemptId:string;
 receiving:Record<string,string>;challengeId?:string;
}
export interface OwnerNumberMatchingDependencies {
 auth:{callerTaskId:string;
  helper(operation:string,args:Record<string,unknown>):Promise<Record<string,any>>;
  followLogin(options:{requestId:string;intervalMs?:number;maxObservations?:number}):Promise<Record<string,any>>;
 };
 browser:{verifyLogin(options:OwnerLoginOptions):Promise<Record<string,any>>};
 checkpoints:{
  load(key:string):Promise<OwnerLoginCheckpoint|OwnerNumberMatchingCheckpoint|null>;
  save(key:string,value:OwnerNumberMatchingCheckpoint):Promise<void>;
 };
}
/** Follow an existing owner's private attempt. This interface accepts neither
 * factor values nor approval decisions and creates no credential request. */
export function createOwnerNumberMatchingContinuation(dependencies:OwnerNumberMatchingDependencies):{
 advance(options:OwnerNumberMatchingOptions):Promise<Record<string,any>>;
};
