export interface SavedLoginBinding {taskId:string;profileId:string;browserId:string;tabId:string;targetId:string;}
export interface SavedLoginInput {requestId:string;kind:'username'|'credential_entry'|'submit';state:'intent'|'acknowledged'|'unknown';targetRef?:string;observationRequestId?:string;}
export interface SavedLoginEpisode {schemaVersion:1;episodeId:string;taskId:string;accountDigest:string;createdAtMs:number;deadlineAtMs:number;timeoutMs?:number;inputs:SavedLoginInput[];captureSequence?:number;binding?:SavedLoginBinding;physical?:Record<string,unknown>;controlEpoch?:number;}
export interface SavedLoginResult {taskId:string;episodeId:string;status:'verified_access_after_native_saved_login'|'human_required'|'uncertain';reason:string;retryAllowed:false;evidenceRef?:string;}
export interface NativeSavedLoginOptions {
 callTool:(name:string,args:Record<string,any>,context?:{signal:AbortSignal;timeoutMs:number})=>Promise<any>;
 /** Host-owned durable store. withEpisode serializes this identity across processes. */
 store:{withEpisode:<T>(episodeId:string,run:()=>Promise<T>)=>Promise<T>;load:(episodeId:string)=>Promise<SavedLoginEpisode|null>;save:(episode:SavedLoginEpisode)=>Promise<void>};
 /** Independent intended-account access readback, never a model success marker. */
 verifyAccess:(context:{taskId:string;episodeId:string;binding:SavedLoginBinding;expectedAccount:string;signal:AbortSignal})=>Promise<SavedLoginBinding&{state:string;accountMatched:boolean;evidenceRef:string}>;
 /** Per invocation, including read-only proof. Defaults to 300000 ms. */
 timeoutMs?:number;
 expectedAccount:string;now?:()=>number;
}
export interface NativeSavedLoginClient {resolve(input:{taskId:string;episodeId:string;allowAcknowledgedResume?:boolean}):Promise<SavedLoginResult>;}
/** Covers the window with bounded owner tiles and freshly recaptures the selected tile.
 * At most three owner-guarded inputs. Explicit resume requires a publicly verified
 * acknowledged prefix and preserves original episode clocks; unknown input never replays. */
export function createNativeSavedLoginClient(options:NativeSavedLoginOptions):NativeSavedLoginClient;
