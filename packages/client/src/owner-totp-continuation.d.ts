export interface OwnerTotpOptions {
 taskId:string;guideId:string;accountRef:string;organizationRef?:string;
 origin:string;targetId:string;purpose:string;expiresAt:number;idempotencyKey:string;
 profileId?:string;browserId?:string;tabId?:string;generation?:string;
 originalRequestId?:string;recordId?:string;
}
export interface OwnerTotpCheckpoint {
 contract:'auth-receipts-totp-v1';intent:string;receiving:Record<string,string>;
 preparedPlan:Record<string,unknown>;continuationId:string;expiresAt:number;
 candidates:Array<{recordId:string}>;recordId?:string;requestId?:string;
 authenticationBinding?:Record<string,unknown>;useIntent?:boolean;outcomeUnknown?:boolean;
}
export interface OwnerTotpDependencies {
 auth:{callerTaskId:string;helper(operation:string,args:Record<string,unknown>):Promise<Record<string,any>>};
 browser:{
  verifyLogin(options:OwnerTotpOptions):Promise<Record<string,any>>;
  prepareLogin(options:OwnerTotpOptions):Promise<Record<string,any>>;
  validatePlan(options:OwnerTotpOptions & {plan:Record<string,unknown>}):Promise<Record<string,any>>;
 };
 checkpoints:{load(key:string):Promise<OwnerTotpCheckpoint|null>;save(key:string,value:OwnerTotpCheckpoint):Promise<void>};
}
/** Compose Auth's observed TOTP need into its original private factor request.
 * An originalRequestId uses Auth's validated continuationOf request contract
 * to transfer original receiving custody. SSO uses continue_authentication. */
export function createOwnerTotpContinuation(dependencies:OwnerTotpDependencies):{
 advance(options:OwnerTotpOptions):Promise<Record<string,any>>;
};
