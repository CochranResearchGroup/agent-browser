import type {RoutineResult} from './playbooks.d.ts';

export interface OwnerLoginOptions {
 taskId:string;guideId:string;accountRef:string;organizationRef?:string;
 query:string;accountHint:string;origin:string;targetId:string;purpose:string;
 expiresAt:number;idempotencyKey:string;
 profileId?:string;browserId?:string;tabId?:string;generation?:string;
 /** Read back an existing owner checkpoint; never create a request or private use. */
 reconcile?:boolean;
}
export interface OwnerLoginCheckpoint {
 contract:'auth-receipts-owner-v1';intent:string;
 issuer?:'auth-receipts';callerTaskId?:string;
 receiving:Record<string,string>;
 selected:{sourceId:string;recordId:string};preparedPlan:Record<string,unknown>;
 requestId?:string;advanceIntent?:boolean;outcomeUnknown?:boolean;refusalCode?:string;
}
export interface OwnerLoginDependencies {
 browser:{
  verifyLogin(options:OwnerLoginOptions):Promise<RoutineResult & Record<string,any>>;
  prepareLogin(options:OwnerLoginOptions):Promise<Record<string,any>>;
  validatePlan(options:OwnerLoginOptions & {plan:Record<string,unknown>}):Promise<Record<string,any>>;
 };
 auth:{callerTaskId:string;
  helper(operation:string,args:Record<string,unknown>):Promise<Record<string,any>>;
  requestLogin(options:{idempotencyKey:string;sourceId:string;recordId:string;origin:string;targetId:string;purpose:string;expiresAt:number}):Promise<Record<string,any>>;
  advanceLogin(options:{requestId:string}):Promise<Record<string,any>>;
 };
 checkpoints:{load(key:string):Promise<OwnerLoginCheckpoint|null>;save(key:string,value:OwnerLoginCheckpoint):Promise<void>};
}
/** Owner-authority login; legacy checkpoints remain under their original issuer. */
export function createOwnerLoginPlaybook(dependencies:OwnerLoginDependencies):{
 advance(options:OwnerLoginOptions):Promise<Record<string,any>>;
};
