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

/** Current browser/extension observations supplied by an authorized caller.
 * This describes evidence; it cannot grant input, credential or retry authority. */
export interface NativeLoginGuidanceContext {
 binding:SavedLoginBinding;
 /** Exact HTTP(S) origin only: no account, URL path, query, token or fragment. */
 origin:string;
 accountSelection?:'resolved'|'ambiguous'|'unknown';
 nativeInputState?:'ready'|'disabled'|'unavailable'|'unknown';
 savedCredentialState?:'unchecked'|'available'|'not_found'|'unsupported';
 savedLoginRoutine?:'applicable'|'unsupported'|'unchecked';
 lastPassState?:'not_observed'|'active'|'authentication_required'|'absent'|'ambiguous';
 authState?:'available'|'unavailable'|'unqualified'|'unknown';
 uncertainInput?:boolean;
}
export interface NativeLoginPathway {
 id:'browser_saved'|'lastpass'|'auth_receipts';state:string;
 interaction:'native_capture/native_input'|'existing_auth_helper';steps:string[];
}
export interface NativeLoginGuidance {
 schemaVersion:'agent-browser.native-login-guidance.v1';taskId:string;profileId:string;origin:string;binding:SavedLoginBinding;
 contextEvidence:'caller_supplied_requires_current_task_observation';browserEffects:'none';credentialPermission:'not_granted';
 nextStep:{pathway:string;operation:string;reason:string};pathways:NativeLoginPathway[];requirements:string[];
}
/** Bounded advisory steps for this current browser context; no browser or Auth calls.
 * Reuses existing Native/saved-login ownership and does not recognize icon pixels. */
export function describeNativeLoginPathways(context:NativeLoginGuidanceContext):NativeLoginGuidance;
