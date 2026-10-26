export type CallTool = (name: string, args: Record<string, unknown>) => Promise<{
  content: Array<{type: string; text?: string}>;
}>;
export interface VerifyLoginOptions {
  taskId: string;
  guideId: string;
  accountRef: string;
  organizationRef: string;
  maxObservationAgeMs?: number;
}
export interface RoutineResult {
  schemaVersion: 'agent-browser.routine-result.v1';
  guideId: string;
  routine: 'verify_login'|'prepare_login'|'validate_plan';
  status: 'completed' | 'waiting' | 'adaptation_required' | 'failed';
  reasonCode: string;
  effectState: 'no_effect';
  taskId: string;
  profileId: string;
  browserId: string;
  tabId: string;
  accountVerified: boolean;
  organizationVerified: boolean;
  observationElapsedMs: number;
  guideDigest?:string;
  plan?:Record<string, unknown>|null;
}
export interface CollectPostOptions {
  taskId:string;guideId:string;maxItems?:number;maxPages?:number;maxScrolls?:number;maxObservationAgeMs?:number;
}
export interface PostResult {
  schemaVersion:'agent-browser.routine-result.v1';routine:'collect_posts';
  taskId:string;guideId:string;profileId:string;browserId:string;tabId:string;
  status:'completed'|'partial'|'waiting'|'adaptation_required'|'failed';
  reasonCode:string;effectState:'no_effect'|'unknown';
  items:Array<{id:string;title:string;url:string}>;noResultsVerified:boolean;
  pagesUsed:number;scrollsUsed:number;malformedItems:number;
  budgets:{maxItems:number;maxPages:number;maxScrolls:number;maxObservationAgeMs:number};
}
export function createPlaybookClient(callTool: CallTool): {
  listTabs(taskId:string):Promise<{taskId:string;currentTabId:string;tabs:Array<{tabId:string}>}>;
  selectTab(taskId:string,tabId:string):Promise<unknown>;
  discover(operation?: 'siteguides' | 'inspect_guide' | 'verify_login' | 'prepare_login' | 'validate_plan'|'collect_posts'): Promise<unknown>;
  listGuides(taskId: string): Promise<{guides: string[]}>;
  inspectGuide(taskId: string, guideId: string): Promise<unknown>;
  inspectTeachingDraft(taskId: string, guideId: string): Promise<import('./teaching-draft.js').TeachingSiteGuide>;
  validateTeachingCandidate(options: {taskId:string; guide:import('./teaching-draft.js').TeachingSiteGuide}):Promise<{valid:true;guideId:string;taskId:string;state:'draft';executionAllowed:false;operatorReviewQualified:false}>;
  verifyLogin(options: VerifyLoginOptions): Promise<RoutineResult>;
  prepareLogin(options:VerifyLoginOptions):Promise<RoutineResult>;
  validatePlan(options:VerifyLoginOptions & {plan:Record<string, unknown>}):Promise<RoutineResult>;
  collectPosts(options:CollectPostOptions):Promise<PostResult>;
};
