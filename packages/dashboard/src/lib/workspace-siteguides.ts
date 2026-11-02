export interface TeachingContext {
  taskId: string;
  profileId: string;
  sitePolicyId: string;
  account: string;
  organization: string;
}
export interface SiteGuideSummary {
  guideId: string; rootGuideId: string; name: string; purpose: string;
  sourceTaskId: string; allowedOrigins: string[]; review: string; repeat: string;
  source: {state: string; replayable: boolean; reason?: string};
  ordinaryReuseAllowed: boolean;
  history: Array<{guideId:string;sha256:string;revision:number;operationId?:string}>;
}
export interface TeachingView {
  guideId: string; rootGuideId: string; sha256: string; contentSha256: string;
  guide: {name: string; teaching: {sourceTaskId:string;playbook:{goal:string};[key:string]:unknown};[key:string]:unknown};
  source: {state:string;replayable:boolean;reason?:string};
  review: {state:string}; validation: {state:string}; ordinaryReuseAllowed:boolean;
  steps: Array<{stepRef:string;explanation:string;expectedResult:{items:string[]};uncertainty:string[];evidence:Array<{id:string;summary:string;replayable:boolean}>}>;
  round: Array<{id:string;question:string;recommendation:string;questionSha256:string}>;
  answers: Array<{questionId:string;answer:string;verification:string}>;
  blockers:string[];
}
export type SiteGuidesTransport = (request:Record<string,unknown>)=>Promise<any>;
/** Same authenticated dashboard session. Never sends credentials or source paths. */
export const requestSiteGuides: SiteGuidesTransport = async request => {
 const response=await fetch('/api/dashboard/siteguides',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(request)});
 const result=await response.json();
 if(!response.ok||result.success!==true)throw Error(result.error??'SiteGuides unavailable');
 return result.data;
};
/** Existing authenticated operator-confirmation owner, with teaching-specific purpose. */
export async function confirmTeachingReceipt(ref:string,request:Record<string,unknown>) {
 const response=await fetch('/api/app-intelligence/operator/confirm',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({confirmationId:ref,action:{kind:'operator_confirmation',confirmationId:ref,label:'Confirm exact teaching content',request}})});
 const result=await response.json();if(!response.ok||result.success!==true)throw Error(result.error??'Teaching confirmation unconfirmed; read the original receipt before continuing.');
 return ref;
}
export const teachingOperationId=()=>`teaching-${crypto.randomUUID()}`;
