export interface ChallengeBinding { taskId: string; profileId: string; browserId: string; tabId: string; targetId: string; }
export interface ChallengeVerificationContext { taskId: string; binding: ChallengeBinding; frameSha256: string; classification: Record<string, unknown>; }
export interface ChallengeVerification { state: 'verified' | 'unverified'; taskId: string; frameSha256: string; evidenceRef: string; }
/** Trusted receiving-backend source records. A caller allow label is not policy. */
export interface NativeChallengePolicySource {
  taskId: string; binding: ChallengeBinding; origin: string; sourceRef: string;
  /** Existing ServiceProvider identity, distinct from a registered action/family. */
  provider?: { id: string; kind: string; enabled: boolean; configRef: string | null; capabilities: string[] };
  profile: { id: string; sitePolicyIds: string[]; manualLoginPreferred?: boolean };
  sitePolicy: {
    id: string; originPattern: string; origins: string[];
    challengePolicy: 'avoid_first' | 'manual_only' | 'provider_allowed' | 'provider_preferred' | 'deny';
    allowedChallengeProviders: string[];
    requiresCdpFree?: boolean; manualLoginPreferred?: boolean; interactionMode?: string;
  };
  taskPolicy: {
    taskId: string; hostId: string; revision: number;
    policy: { version: number; deniedActions: string[]; artifactDestinations: Record<string,string> } | null;
  };
}
export interface BoundedNativeChallengeResult {
  taskId: string;
  status: 'awaiting_operator' | 'intervention' | 'verified_access_after_native' | 'verified_access_after_passive_wait';
  retryAllowed: false; taskCompleted?: false; reason?: string; binding?: ChallengeBinding;
  originalOperation?: Record<string,unknown> | null;
  verification?: { clearance: string; access: string; frameSha256: string };
  handoffId?: string; handoffUrl?: string; nextAction?: string;
}
export interface ChallengeAssistanceCapsule {
  schemaVersion: 1; taskId: string; binding: ChallengeBinding; physical: Record<string,unknown>;
  classification: Record<string, unknown>; handoffId: string; handoffUrl: string; initialEpoch: number;
  status: 'awaiting_operator' | 'operator_has_control' | 'intervention' | 'verified_access_after_return';
  humanEpoch?: number; reason?: string; taskCompleted?: false; verification?: Record<string,string>; result?: unknown;
}
export function createChallengeAssistanceClient(options: {
  callTool: (name: string, args: Record<string,unknown>) => Promise<{isError?: boolean; content: Array<{type: string; text?: string}>}>;
  verifyClearance: (context: ChallengeVerificationContext) => Promise<ChallengeVerification>;
  verifyAccess: (context: ChallengeVerificationContext) => Promise<ChallengeVerification>;
  resolvePolicy?: (context: ChallengeVerificationContext) => Promise<NativeChallengePolicySource>;
  /** Clock/wait ports are backend boundaries for deterministic bounded-wait tests. */
  now?: () => number;
  wait?: (milliseconds: number) => Promise<void>;
}): {
  observe(taskId: string): Promise<unknown>;
  resolve(input: {taskId: string; requestId: string}): Promise<BoundedNativeChallengeResult>;
  prepare(input: {taskId: string}): Promise<ChallengeAssistanceCapsule>;
  recordTakeover(capsule: ChallengeAssistanceCapsule): Promise<ChallengeAssistanceCapsule>;
  continueAfterReturn(capsule: ChallengeAssistanceCapsule): Promise<ChallengeAssistanceCapsule>;
};
