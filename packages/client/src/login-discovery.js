import {withClientErrorRecovery} from './client-recovery.js';
import { callClientOwner } from './client-recovery.js';
// @ts-check
import {summarizeLoginProgress} from './login-progress.js';

/** Inspect supported Auth metadata. An enrolled source may perform its own
 * authorized session recovery during discovery; this adapter never submits
 * target credentials or creates an approval.
 * @param {import('./login-discovery.d.ts').LoginDiscoveryOptions} options
 * @returns {import('./login-discovery.d.ts').LoginDiscoveryClient}
 */
export function createLoginDiscovery({client,readPermission}) {
  if(typeof client?.call!=='function' || readPermission!==undefined && typeof readPermission!=='function')throw invalid();
  return {
    /** @param {import('./login-discovery.d.ts').LoginDiscoveryIntent} intent */
    async inspect(intent) {
      if(!validIntent(intent))throw invalid();
      let request;
      const base=()=>({
        ...summarizeLoginProgress(request??{}),
        extensionState:intent.extensionState ?? 'unknown',
        sourceAvailability:'unknown',
        permission:{state:'visibility_unavailable'},
      });
      let discovery;
      try {
        const capabilities=await callClientOwner(client,'auth_capabilities',{},{source:"packages/client/src/login-discovery.js",owningFunction:"createLoginDiscovery.inspect"});
        if(capabilities?.contractVersion!==1 || capabilities.credentialOperations!==true || capabilities.sharedOwnerAuthority!==true || capabilities.legacyCredentialEffects!==false)return {...base(),category:'capability_unavailable'};
        if(intent.requestId) {
          request=await callClientOwner(client,'inspect_credential_request',{requestId:intent.requestId},{source:"packages/client/src/login-discovery.js",owningFunction:"createLoginDiscovery.inspect"});
          if(request?.requestId!==intent.requestId || request.origin!==intent.origin || typeof (request.recipientAccount??request.account)!=='string' || (request.recipientAccount??request.account).trim().toLowerCase()!==intent.accountHint.trim().toLowerCase()) {
            request={requestId:intent.requestId};return {...base(),category:'request_binding_mismatch'};
          }
          if(['denied','revoked','cancelled'].includes(request.state))return base();
        }
        discovery=await callClientOwner(client,'discover_credentials',{origin:intent.origin,accountHint:intent.accountHint,credentialType:'password'},{source:"packages/client/src/login-discovery.js",owningFunction:"createLoginDiscovery.inspect"});
      } catch(error) {
        return {...base(),category:error?.code==='auth_not_authorized'?'authority_denied':'capability_unavailable',...(error?.failure?{failure:error.failure}:{}),...(error?.clientRecovery?{clientRecovery:error.clientRecovery}:{}),dependencyFailure:{component:'auth_owner',code:typeof error?.code==='string'&&/^auth_[a-z0-9_]+$/.test(error.code)?error.code:'client_auth_owner_failure'}};
      }
      if(!discovery || !Array.isArray(discovery.candidates) || !Array.isArray(discovery.sources) || discovery.unavailable!==undefined && !Array.isArray(discovery.unavailable))return {...base(),category:'capability_unavailable'};
      const selected=discovery.state==='unique' && discovery.selected
        && reference(discovery.selected.sourceId) && reference(discovery.selected.recordId)
        && discovery.selected.origin===intent.origin
        && typeof discovery.selected.account==='string'
        && discovery.selected.account.trim().toLowerCase()===intent.accountHint.trim().toLowerCase()
        && discovery.candidates.some(candidate=>candidate?.sourceId===discovery.selected.sourceId && candidate.recordId===discovery.selected.recordId && candidate.origin===discovery.selected.origin && candidate.account===discovery.selected.account)
        ? {sourceId:discovery.selected.sourceId,recordId:discovery.selected.recordId}:undefined;
      if(selected && request && (request.sourceId!==selected.sourceId || request.recordId!==selected.recordId))return {...base(),category:'request_binding_mismatch'};
      const sourceAvailability=discovery.sources?.length
        ? discovery.sources.some(source=>reference(source?.sourceId) && !discovery.unavailable?.includes(source.sourceId))?'available':'unavailable'
        : 'unknown';
      let category=selected?'credential_ready':sourceAvailability==='unavailable'?'source_unavailable':discovery.state==='partial'?'discovery_incomplete':'credential_choice_required';
      /** @type {import('./login-discovery.d.ts').LoginDiscoveryResult['permission']} */
      let permission={state:'visibility_unavailable'};
      if(selected && readPermission) {
        const scope={origin:intent.origin,accountHint:intent.accountHint,...selected,...(intent.requestId?{requestId:intent.requestId}:{})};
        try {
          const observed=await readPermission(scope);
          if(observed?.scope && Object.keys(scope).every(key=>observed.scope[key]===scope[key]) && observed.current===true && reference(observed.evidenceRef)) {
            if(observed.state==='authorized' && ['standing','request'].includes(observed.kind??''))permission={state:'owner_authorized',kind:observed.kind,evidenceRef:observed.evidenceRef};
            else if(observed.state==='denied')permission={state:'denied',evidenceRef:observed.evidenceRef};
          }
        } catch { /* A hidden owner view proves no absence or denial. */ }
      }
      const progress=base();
      if(['input_outcome_unknown','receiving_target_unavailable'].includes(progress.category))category=progress.category;
      if(permission.state==='denied')category='authority_denied';
      return {
        ...progress, category,
        extensionState:intent.extensionState ?? 'unknown', sourceAvailability,
        ...(selected?{selected}:{}),
        permission,
      };
    },
  };
}

function invalid() {return withClientErrorRecovery(Object.assign(Error('Invalid login discovery intent'),{code:'auth_invalid_request'}),{source:"packages/client/src/login-discovery.js",owningFunction:"invalid"},'auth_invalid_request');}
/** @param {unknown} value */
function reference(value) {return typeof value==='string' && /^[A-Za-z0-9][A-Za-z0-9:._-]{0,199}$/.test(value);}
/** @param {import('./login-discovery.d.ts').LoginDiscoveryIntent} intent */
function validIntent(intent) {
  if(!intent || Object.keys(intent).some(key=>!['origin','accountHint','extensionState','requestId'].includes(key)) || typeof intent.accountHint!=='string' || !intent.accountHint.trim() || intent.accountHint.length>512 || intent.extensionState!==undefined && !['signed_in','signed_out','unknown'].includes(intent.extensionState) || intent.requestId!==undefined && !reference(intent.requestId))return false;
  try {const origin=new URL(intent.origin);return origin.protocol==='https:' && origin.origin===intent.origin && !origin.username && !origin.password;}catch{return false;}
}
