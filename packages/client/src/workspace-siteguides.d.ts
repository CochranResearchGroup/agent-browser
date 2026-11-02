import type {TeachingReviewOptions} from './teaching-review.js';
import type {TeachingEvidenceClient,TeachingSelection} from './teaching-evidence.js';
/** Configuration is receiver-owned. Never construct these ports from request JSON. */
export interface WorkspaceSiteGuidesOptions extends TeachingReviewOptions {
 browser:TeachingReviewOptions['browser']&{inspectGuide?:(taskId:string,guideId:string)=>Promise<any>};
 isCurrentTask?:(taskId:string)=>Promise<boolean>;
 catalogTaskId?:string;
 context?:Record<string,unknown>&{allowedOrigins?:string[]};
 evidence?:TeachingEvidenceClient;
 listSources?:(taskId:string)=>Promise<Array<{sourceKey:string;recordingId:string;narrationSourceId:string;state:'not_checked'}>>;
 authorizeTask?:(taskId:string,mode:'read'|'use')=>Promise<boolean>;
 resolveSelection?:(request:{taskId:string;sourceKey:string})=>Promise<TeachingSelection>;
}
export function createWorkspaceSiteGuides(options:WorkspaceSiteGuidesOptions):{
 dispatch(request:Record<string,any>):Promise<any>;
};
