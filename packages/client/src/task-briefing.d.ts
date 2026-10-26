/** Retained evidence is not current browser/page or task-completion proof. */
export interface TaskBriefing {
  schemaVersion: 'agent-browser.task-briefing.v1';
  taskId: string;
  profileId: string;
  browserId: string;
  tabId: string | null;
  targetId: string | null;
  contextEvidence: 'retained_not_live';
  currentTargetState: 'retained_not_live' | 'missing_or_conflicting';
  lastActivityAtMs: number;
  goal: { status: 'missing' | 'caller_supplied'; taskName?: string | null; purpose?: string | null; requestId?: string | null; jobId?: string | null; [key: string]: unknown };
  lastVerifiedOutcome: { status: 'missing' | 'historical_verified_navigation'; observedAtMs?: number; taskCompletion?: 'unverified'; [key: string]: unknown };
  artifacts: { status: 'not_recorded' | 'recorded_references_not_revalidated'; references: Array<{artifactId: string; kind: string; handle: string; observedAtMs: number}> | null };
  assistance: { status: string; [key: string]: unknown };
  control: { epoch: number; owner: string; pauseReads: boolean; cooperativeOnly: true };
  uncertainOperations: Array<{source: string; operationId: string; retryAllowed: false; scope?: string; state?: string}>;
  uncertaintyTruncated: boolean;
  legacyRequestAttribution: 'exact_retained_lane' | 'ambiguous_excluded';
  browserEffects: 'none';
  liveObservation: 'not_performed';
}
export function getTaskBriefing(options: { baseUrl: string; taskId: string; headers?: Record<string,string>; signal?: AbortSignal; fetch?: typeof globalThis.fetch }): Promise<TaskBriefing>;
