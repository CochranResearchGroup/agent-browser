// @ts-check
import {clientFailureError} from './client-recovery.js';
/** Resource working limits, never review, publication or task permission. */
export const DEFAULT_TEACHING_CAPACITY=Object.freeze({maximumLineageRevisions:256,maximumRetainedContexts:4096,maximumCatalogEntries:4096});
const controllers=new WeakSet();
/** @param {import('./teaching-capacity.js').TeachingCapacityPolicy|import('./teaching-capacity.js').TeachingCapacityControl} [initial] */
export function createTeachingCapacity(initial={}) {
  if(typeof initial==='function'&&controllers.has(initial))return initial;
  let policy={...DEFAULT_TEACHING_CAPACITY},source='client_default';
  const invalid=()=>clientFailureError('teaching_capacity_policy_invalid',{source:'packages/client/src/teaching-capacity.js',owningFunction:'createTeachingCapacity'});
  const checked=value=>{
    if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).some(key=>!Object.prototype.hasOwnProperty.call(DEFAULT_TEACHING_CAPACITY,key)))throw invalid();
    for(const limit of Object.values(value))if(!Number.isSafeInteger(limit)||limit<1)throw invalid();
    return value;
  };
  policy={...policy,...checked(initial)};if(Object.keys(initial).length)source='caller_configuration';
  /** @param {import('./teaching-capacity.js').TeachingCapacityPolicy} [next] */
  const capacity=next=>{
    if(next!==undefined){policy={...policy,...checked(next)};source='live_update';}
    return {schemaVersion:/** @type {const} */('agent-browser.teaching-capacity.v1'),policy:{...policy},defaults:{...DEFAULT_TEACHING_CAPACITY},source,requiresRestart:false,permissionChanged:false,historyPreserved:true};
  };
  controllers.add(capacity);return capacity;
}
