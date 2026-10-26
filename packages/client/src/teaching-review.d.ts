import type {TeachingSiteGuide,TeachingOperatorReview,TeachingRetainedVersion,TeachingEvidenceReference,TeachingValidation} from './teaching-draft.js';
import type {createPlaybookClient} from './playbooks.js';
import type {TeachingGrillingOptions} from './teaching-grilling.js';
export interface TeachingDecisionRequest {
  decisionRef: string; taskId: string; rootGuideId: string; reviewedGuideId: string;
  reviewedSha256: string; contentSha256: string;
}
export interface TeachingEvidenceAvailability {
  id:string;artifact:string;sha256:string;state:'available'|'unavailable'|'unchecked';reason?:string;
}
export interface TeachingSourceState {
  recordingId: string; videoSha256: string; sourceTaskId: string;
  state: 'available' | 'unavailable' | 'unchecked'; reason?: string;
  evidence?:TeachingEvidenceAvailability[];
  gaps?: Array<{tMs: number; scope: string; reason: string}>;
}
export interface TeachingVersionReference {
  guideId: string; sha256: string; rootGuideId: string; revision: number; operationId?: string;
}
export type TeachingSaveResult = (TeachingVersionReference & {status:'saved';ordinaryReuseAllowed:false;recoveredOriginal?:boolean}) |
  {status:'conflict';reason:string;current?:TeachingVersionReference;guideId?:string;sha256?:string;ordinaryReuseAllowed:false} |
  {status:'unknown';reason:'publication_result_unconfirmed';rootGuideId:string;operationId:string;ordinaryReuseAllowed:false};
export interface TeachingReviewInput {
  taskId: string; guideId: string; expectedSha256: string; operationId: string;
}
export interface TeachingFreshRepeatRequest {
  validationRef:string;taskId:string;sourceTaskId:string;rootGuideId:string;
  executedGuideId:string;executedSha256:string;contentSha256:string;
  reviewDecisionId:string;playbookId:string;requestedStartDate:string;requestedEndDate:string;reportId:string;
}
export interface TeachingFreshRepeatInput extends TeachingReviewInput {
  validationRef:string;requestedStartDate:string;requestedEndDate:string;reportId:string;
}
export type TeachingValidationState = ({state:'verified'} & TeachingValidation) |
  {state:'not_validated'|'unverified';reason?:string;validationId?:string};
export interface TeachingAcceptedVersion {
  available:boolean;state:string;guideId?:string;sha256?:string;
  readiness?:'not_requalified'|'qualified_fresh_repeat';validationId?:string;
  previousAcceptedVersion?:TeachingAcceptedVersion;
}
export type TeachingPromotionResult = TeachingSaveResult | (TeachingVersionReference & {
  status:'promoted';ordinaryReuseAllowed:true;recoveredOriginal?:boolean;acceptedVersion:TeachingRetainedVersion;
});
export interface TeachingReviewView extends TeachingVersionReference {
  status:'opened'; guide:TeachingSiteGuide; contentSha256:string;
  steps:Array<{
    stepRef:string;routineId:string;index:number;action:string;target:string;explanation:string;
    expectedResult:{scope:'step'|'routine';items:string[]};
    evidence:Array<TeachingEvidenceReference & {replayable:boolean;availabilityState:'available'|'unavailable'|'unchecked';unavailabilityReason?:string}>;
    sourceGaps:Array<{tMs:number;scope:string;reason:string;timeDomain:'recording_ms'}>;
    uncertainty:string[];
  }>;
  source:TeachingSourceState & {replayable:boolean;evidenceAvailability?:Array<TeachingEvidenceAvailability & {replayable:boolean}>;gaps?:Array<{tMs:number;scope:string;reason:string;timeDomain:'recording_ms'}>};
  review: ({state:'approved'|'rejected'} & TeachingOperatorReview) | {state:'not_reviewed'|'unverified';reason?:string;decisionId?:string};
  validation:TeachingValidationState;
  acceptedVersion:TeachingAcceptedVersion;
  warnings:string[];
  proposals:Array<{id:string;question:string;interpretation:string;evidenceRefs:string[];adopted:false}>;
  blockers:string[];ordinaryReuseAllowed:boolean;
}
/** Existing private candidate directory; receiver-owned authenticated ports stay outside model request data. */
export interface TeachingReviewOptions {
  candidateDirectory:string;
  browser:Pick<ReturnType<typeof createPlaybookClient>,'inspectTeachingDraft'|'validateTeachingCandidate'>;
  acceptedVersion?:TeachingRetainedVersion;
  sourceStatus?:(request:Pick<TeachingSourceState,'recordingId'|'videoSha256'|'sourceTaskId'> & {evidence:TeachingEvidenceReference[]})=>Promise<TeachingSourceState>;
  /** Read an existing explicit operator decision, not create an approval from a role label. */
  readOperatorDecision?:(request:TeachingDecisionRequest)=>Promise<TeachingOperatorReview>;
  /** Existing exact report/content evidence from the receiving application, never caller success labels. */
  readFreshRepeat?:(request:TeachingFreshRepeatRequest)=>Promise<TeachingValidation>;
  /** Reuses the grilling authority reader when optional clarification metadata exists. */
  readOperatorAnswer?:TeachingGrillingOptions['readOperatorAnswer'];
}
export interface TeachingReviewClient {
  open(input:{taskId:string;guideId:string}):Promise<TeachingReviewView>;
  save(input:TeachingReviewInput & {guide:TeachingSiteGuide}):Promise<TeachingSaveResult>;
  recordDecision(input:TeachingReviewInput & {decisionRef:string}):Promise<TeachingSaveResult>;
  recordFreshRepeat(input:TeachingFreshRepeatInput):Promise<TeachingSaveResult>;
  promote(input:TeachingReviewInput):Promise<TeachingPromotionResult>;
  lookupOperation(input:{rootGuideId:string;operationId:string}):Promise<(TeachingVersionReference & {status:'saved';recoveredOriginal:true;ordinaryReuseAllowed:false})|{status:'not_observed';operationId:string;retryAuthority:'not_inferred'}>;
}
export function createTeachingReviewClient(options:TeachingReviewOptions):TeachingReviewClient;
