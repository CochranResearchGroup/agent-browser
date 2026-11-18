import type {LoginProgress} from './login-progress.js';

export interface LoginDiscoveryIntent {
  origin:string;
  accountHint:string;
  extensionState?:'signed_in'|'signed_out'|'unknown';
  requestId?:string;
}
export interface LoginDiscoveryOptions {
  client:{call(operation:string,args:Record<string,unknown>):Promise<Record<string,any>>};
  /** Optional authenticated host-owned reader of a supported owner metadata view.
   * It must check current caller identity and policy applicability server-side.
   * No broad rule inventory is requested when this reader is absent. Its result
   * is metadata, never authority for private use or an expired request replay. */
  readPermission?(scope:LoginPermissionScope):Promise<{
    state:'authorized'|'denied'|'visibility_unavailable';
    kind?:'standing'|'request';
    current?:boolean;
    scope?:LoginPermissionScope;
    evidenceRef?:string;
  }>;
}
export interface LoginPermissionScope {
  origin:string;
  accountHint:string;
  sourceId:string;
  recordId:string;
  requestId?:string;
}
export interface LoginDiscoveryResult extends LoginProgress {
  extensionState:'signed_in'|'signed_out'|'unknown';
  sourceAvailability:string;
  selected?:{sourceId:string;recordId:string};
  permission:{state:string;kind?:'standing'|'request';evidenceRef?:string};
}
export interface LoginDiscoveryClient {
  inspect(intent:LoginDiscoveryIntent):Promise<LoginDiscoveryResult>;
}
export function createLoginDiscovery(options:LoginDiscoveryOptions):LoginDiscoveryClient;
