export interface ChallengeBinding { taskId: string; profileId: string; browserId: string; tabId: string; targetId: string; }
export interface ChallengeVerificationContext { taskId: string; binding: ChallengeBinding; frameSha256: string; classification: Record<string, unknown>; }
export interface ChallengeVerification { state: 'verified' | 'unverified'; taskId: string; frameSha256: string; evidenceRef: string; }
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
}): {
  observe(taskId: string): Promise<unknown>;
  prepare(input: {taskId: string}): Promise<ChallengeAssistanceCapsule>;
  recordTakeover(capsule: ChallengeAssistanceCapsule): Promise<ChallengeAssistanceCapsule>;
  continueAfterReturn(capsule: ChallengeAssistanceCapsule): Promise<ChallengeAssistanceCapsule>;
};
