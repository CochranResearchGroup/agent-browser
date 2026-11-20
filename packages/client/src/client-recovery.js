// @ts-check
import {CLIENT_RECOVERY_CODES,CLIENT_RECOVERY_RECIPES,CLIENT_RECOVERY_SOURCE_SHA256} from './client-recovery.generated.js';
const validCode=value=>typeof value==='string'&&/^[a-z][a-z0-9_]{0,127}$/.test(value);
/** Local adapter recovery is separate from the authoritative owner's plan.
 * Binding supplies arguments only; it never asserts that authorization or a
 * recipe's evidence preconditions are satisfied and never executes a call.
 * @param {string} code @param {Record<string, any>} [context]
 */
export function clientRecoveryFor(code,context={}) {
  const safeCode=validCode(code)?code:'client_unclassified_failure';
  const entry=/** @type {Record<string, any>} */(CLIENT_RECOVERY_CODES)[safeCode];
  const packet={code:safeCode,...Object.fromEntries(['source','owningFunction','taskId','requestId','episodeId'].filter(key=>typeof context[key]==='string'&&context[key].length<=4000).map(key=>[key,context[key]]))};
  if(!entry)return {schemaVersion:'agent-browser.client-recovery.v1',coverage:'unknown',code:safeCode,packet,
    requiredActor:'caller_with_original_context',automaticExecutionAllowed:false,
    inspection:'Read the preserved owner failure/occurrence and the exact original operation before further input. Unknown coverage does not establish no-effect or permission to retry.',
    remediation:'Use the current owner-supported recovery/discovery interfaces with the original identifiers. Keep this safe code/source packet for diagnosis; do not log arguments, credentials or response bodies.'};
  const missing=new Set();
  const bind=value=>{
    if(typeof value==='string'&&/^\$[A-Za-z][A-Za-z0-9]*$/.test(value)){
      const key=value.slice(1);if(context[key]===undefined){missing.add(key);return value;}return context[key];
    }
    if(Array.isArray(value))return value.map(bind);
    if(value&&typeof value==='object')return Object.fromEntries(Object.entries(value).map(([key,child])=>[key,bind(child)]));
    return value;
  };
  const recipe=bind(/** @type {Record<string, any>} */(CLIENT_RECOVERY_RECIPES)[entry.recipe]);
  return {schemaVersion:'agent-browser.client-recovery.v1',coverage:'registered',code:safeCode,recipe:entry.recipe,
    registrySourceSha256:CLIENT_RECOVERY_SOURCE_SHA256,evidenceClass:entry.evidenceClass,packet,
    ...recipe,unboundArguments:[...missing],bindingState:missing.size?'requires_caller_binding':'arguments_bound',automaticExecutionAllowed:false};
}
/** @param {string} code @param {Record<string, any>} [context] */
export function clientFailureError(code,context={}) {
  const recovery=clientRecoveryFor(code,context);
  return Object.assign(new Error(code),{code:recovery.code,source:context.source,owningFunction:context.owningFunction,clientRecovery:recovery,
    logging:{state:'local_only',ownerOccurrenceId:null,recordableFields:['code','source','owningFunction']}});
}
/** Attach local recourse only to an explicit failure result. Preserve every
 * owner field, including failure, occurrenceId and owner continuation.
 * @template T @param {T} result @param {Record<string, any>} [context] @returns {T}
 */
export function withClientResultRecovery(result,context={}) {
  const original=/** @type {any} */(result),owner=context.ownerError;
  const preserved=owner&&typeof owner==='object'?Object.fromEntries(['failure','occurrenceId','requestId','recoveryPlan','executableNextAction','clientRecovery'].filter(key=>owner[key]!==undefined).map(key=>[key,owner[key]])):{};
  const value=original&&typeof original==='object'?{...preserved,...original}:original,reason=value?.reason??value?.reasonCode??(value?.outcome==='incomplete'?value.category:undefined);
  if(!value||typeof value!=='object'||!validCode(reason)||(typeof value.status!=='string'&&value.outcome!=='incomplete')||value.status?.startsWith('verified')||value.status==='completed')return result;
  const selected=value.clientRecovery?.coverage==='registered'?value.clientRecovery.code:reason;
  const recovery=clientRecoveryFor(selected,{...context,...value.clientRecovery?.packet,taskId:value.taskId??context.taskId,episodeId:value.episodeId??context.episodeId,requestId:value.originalOperation?.requestId??value.requestId??value.recovery?.inspect?.arguments?.requestId??value.clientRecovery?.packet?.requestId??context.requestId});
  return {...value,clientRecovery:{...recovery,...(selected!==reason?{normalization:{ownerCode:reason,ownerCodeCoverage:'not_asserted'}}:{}),ownerRecoveryAvailable:!!(value.failure||value.clientRecovery?.ownerRecoveryAvailable)}};
}

/** Keep an owner error intact while adding separate local adapter recourse.
 * @param {any} error @param {Record<string,any>} [context] @param {string} [localCode]
 */
export function withClientErrorRecovery(error,context={},localCode) {
  const target=error&&typeof error==='object'?error:new Error('client_unclassified_failure');
  if(target.clientRecovery)return target;
  const code=localCode??(validCode(target.code)?target.code:'client_unclassified_failure');
  const recovery=clientRecoveryFor(code,context);
  if(validCode(target.code)&&target.code!==code)recovery.packet.ownerCode=target.code;
  const addition={clientRecovery:{...recovery,ownerRecoveryAvailable:!!(target.failure||target.recoveryPlan||target.executableNextAction||target.nextAction)},...(!Object.prototype.hasOwnProperty.call(target,'source')?{source:context.source}:{}),...(!Object.prototype.hasOwnProperty.call(target,'owningFunction')?{owningFunction:context.owningFunction}:{}),...(!Object.prototype.hasOwnProperty.call(target,'code')?{code:recovery.code}:{})};
  if(Object.isExtensible(target))return Object.assign(target,addition);
  return Object.assign(Object.create(Object.getPrototypeOf(target),Object.getOwnPropertyDescriptors(target)),addition);
}
/** Call the same verified Auth client without substituting its object/identity.
 * Only opaque original IDs and public operation names join local recourse.
 * @param {{call:(operation:string,args:any)=>Promise<any>}} client
 * @param {string} operation @param {any} args @param {Record<string,any>} context
 */
export async function callClientOwner(client,operation,args,context) {
  try{return await client.call(operation,args)}catch(error){
    throw withClientErrorRecovery(error,{...context,requestId:typeof args?.requestId==='string'?args.requestId:undefined},'client_auth_owner_failure');
  }
}
