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
  const value=/** @type {any} */(result);
  if(!value||typeof value!=='object'||typeof value.reason!=='string'||!validCode(value.reason)||typeof value.status!=='string'||value.status.startsWith('verified')||value.status==='completed')return result;
  return {...value,clientRecovery:clientRecoveryFor(value.reason,{...context,...value.clientRecovery?.packet,taskId:value.taskId??context.taskId,episodeId:value.episodeId??context.episodeId,requestId:value.originalOperation?.requestId??value.recovery?.inspect?.arguments?.requestId??value.clientRecovery?.packet?.requestId??context.requestId})};
}
