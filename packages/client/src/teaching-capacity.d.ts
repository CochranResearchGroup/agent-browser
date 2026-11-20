export interface TeachingCapacityPolicy {maximumLineageRevisions?:number;maximumRetainedContexts?:number;maximumCatalogEntries?:number;}
export interface TeachingCapacitySnapshot {
 schemaVersion:'agent-browser.teaching-capacity.v1';policy:Required<TeachingCapacityPolicy>;defaults:Required<TeachingCapacityPolicy>;
 source:string;requiresRestart:false;permissionChanged:false;historyPreserved:true;
}
/** Use an existing owner's capacity function to share its live resource policy. */
export interface TeachingCapacityControl {(next?:TeachingCapacityPolicy):TeachingCapacitySnapshot;}
export const DEFAULT_TEACHING_CAPACITY:Readonly<Required<TeachingCapacityPolicy>>;
export function createTeachingCapacity(initial?:TeachingCapacityPolicy|TeachingCapacityControl):TeachingCapacityControl;
