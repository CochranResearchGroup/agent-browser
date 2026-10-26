export type TeachingAttribution = 'observed' | 'inferred' | 'unknown';
export interface TeachingEvidenceReference {
  id: string; kind: 'video' | 'narration' | 'input' | 'result'; artifact: string;
  startMs: number; endMs: number; sha256: string; summary: string;
}
export interface TeachingClaim {id: string; attribution: TeachingAttribution; statement: string; evidenceRefs: string[]}
export interface TeachingParameter {id: string; kind: 'date' | 'boolean' | 'text'; example: string | boolean; meaning: string; evidenceRefs: string[]}
export interface TeachingStep {
  action: 'observe' | 'select' | 'activate' | 'fill' | 'set_checked' | 'verify' | 'request_assistance';
  target: string; parameter?: string; expectedResult?: string; description: string; evidenceRefs: string[]; attribution: TeachingAttribution;
}
export interface TeachingBranch {condition: string; next: string; evidenceRefs: string[]; attribution: TeachingAttribution}
export interface TeachingRoutine {
  id: string; goal: string; inputs: string[]; preconditions: string[]; steps: TeachingStep[];
  branches: TeachingBranch[]; outcomes: Array<{description: string; verification: string; evidenceRefs: string[]}>;
  recovery: TeachingBranch[];
}
export interface TeachingRetainedVersion {guideId: string; sha256: string}
export interface TeachingVersionLineage {
  rootGuideId: string; revision: number; parentGuideId: string; parentSha256: string;
  operationId: string; operationKind: 'edit' | 'review'; intentSha256: string;
  acceptedVersion?: TeachingRetainedVersion;
}
export interface TeachingOperatorReview {
  decisionId: string; decision: 'approved' | 'rejected'; actorRef: string;
  sourceRef: string; provenance: 'authenticated_operator'; taskId: string;
  rootGuideId: string; reviewedGuideId: string; reviewedSha256: string;
  contentSha256: string; sourceUnavailableAcknowledged: boolean;
}
/** Inert knowledge inside the existing SiteGuide candidate, never execution approval. */
export interface TeachingDraft {
  schemaVersion: 'agent-browser.teaching-draft.v1'; state: 'draft'; preparedBy: string;
  sourceTaskId: string; recordingId: string; videoSha256: string;
  evidence: TeachingEvidenceReference[]; claims: TeachingClaim[]; parameters: TeachingParameter[];
  states: Array<{id: string; description: string; evidenceRefs: string[]}>;
  controls: Array<{id: string; role: string; name: string; attribution: TeachingAttribution; evidenceRefs: string[]}>;
  routines: TeachingRoutine[];
  playbook: {id: string; goal: string; routineIds: string[]; parameters: string[]; completionVerification: string; evidenceRefs: string[]};
  unresolved: Array<{id: string; question: string; interpretation: string; evidenceRefs: string[]}>;
  limitations: string[]; lineage?: TeachingVersionLineage; review?: TeachingOperatorReview;
}
export interface TeachingSiteGuide {
  schemaVersion: 'agent-browser.siteguide.v1'; guideId: string; name: string;
  allowedOrigins: string[]; states: Array<{state: string; selector: string}>;
  teaching: TeachingDraft;
}
export function writeTeachingDraftCandidate(options: {
  candidateDirectory: string; candidateOnly: true; guide: TeachingSiteGuide;
}): Promise<{guideId: string; path: string; sha256: string; state: 'draft'; executionAllowed: false}>;
