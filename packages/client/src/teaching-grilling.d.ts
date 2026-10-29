import type {TeachingSiteGuide,TeachingClarificationQuestion,TeachingClarificationAnswer} from './teaching-draft.js';
import type {createTeachingReviewClient,TeachingReviewView,TeachingReviewInput,TeachingSaveResult} from './teaching-review.js';
export interface TeachingGrillingOptions {
 review:ReturnType<typeof createTeachingReviewClient>;
 /** Trusted application configuration; reads an existing authenticated answer event. */
 readOperatorAnswer?:(request:Omit<TeachingClarificationAnswer,'answer'|'actorRef'|'sourceRef'|'provenance'>)=>Promise<TeachingClarificationAnswer>;
}
export interface TeachingAnswerVerificationOptions {
 view:Pick<TeachingReviewView,'guide'|'rootGuideId'>;
 readOperatorAnswer?:TeachingGrillingOptions['readOperatorAnswer'];
 readCandidate:(request:{taskId:string;guideId:string})=>Promise<Pick<TeachingReviewView,'guide'|'rootGuideId'|'sha256'|'contentSha256'>>;
}
export function verifyTeachingClarificationAnswers(options:TeachingAnswerVerificationOptions):Promise<Array<TeachingClarificationAnswer & {verification:'verified'|'unverified'}>>;
export interface TeachingGrillingView extends TeachingReviewView {
 round:Array<TeachingClarificationQuestion & {recommendation:string;recommendationAdopted:false;questionSha256:string;sources:TeachingSiteGuide['teaching']['evidence']}>;
 answers:Array<TeachingClarificationAnswer & {verification:'verified'|'unverified'}>;
 answerAuthority:'verified'|'unverified';
}
export function createTeachingGrillingClient(options:TeachingGrillingOptions):{
 prepare(input:TeachingReviewInput & {questions:Array<Pick<TeachingClarificationQuestion,'id'|'category'|'stepRefs'|'dependsOn'>>}):Promise<TeachingSaveResult>;
 open(input:{taskId:string;guideId:string}):Promise<TeachingGrillingView>;
 answer(input:TeachingReviewInput & {questionId:string;answerRef:string;revisedGuide:TeachingSiteGuide}):Promise<TeachingSaveResult>;
 lookupOperation:ReturnType<typeof createTeachingReviewClient>['lookupOperation'];
};
