const FOUNDATION_STRESS_RECIPE_ID = "p110-foundation-stress-v1" as const;
const EFFECT_STATES = ["no_effect", "verified_success", "effect_uncertain", "cancelled_after_effect"] as const;
const REPLAY_STATES = ["first_execution", "replayed_terminal"] as const;
const CLEANUP_STATES = ["not_needed", "released", "release_failed"] as const;
const VERIFICATION_STATES = ["not_verified", "passed", "unchanged"] as const;
const ENTRY_GATES = ["closed_live_evidence_required"] as const;
const PROMPT_STATES = ["actionable_observation", "operator_intervention_required"] as const;
const PROMPT_REASON_CODES = [
  "synthetic_prompt_actionable",
  "synthetic_prompt_requires_operator_review",
] as const;

type EffectState = typeof EFFECT_STATES[number];
type ReplayState = typeof REPLAY_STATES[number];
type CleanupState = typeof CLEANUP_STATES[number];
type VerificationState = typeof VERIFICATION_STATES[number];
type EntryGate = typeof ENTRY_GATES[number];
type PromptState = typeof PROMPT_STATES[number];
type PromptReasonCode = typeof PROMPT_REASON_CODES[number];

export type FoundationStressReceiptProjection = {
  recipeId: typeof FOUNDATION_STRESS_RECIPE_ID;
  operationIdentity: "recorded";
  effectState: EffectState;
  replayState: ReplayState;
  cleanupState: CleanupState;
  verificationState: VerificationState;
  entryGate: EntryGate;
  promptState?: PromptState;
  promptReasonCode?: PromptReasonCode;
  handoffState?: "ready";
  handoffId?: string;
};

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function stringField(value: Record<string, unknown>, key: string): string | null {
  const field = value[key];
  return typeof field === "string" && field.trim() ? field : null;
}

function enumField<const T extends readonly string[]>(
  value: Record<string, unknown>,
  key: string,
  allowed: T,
): T[number] | null {
  const field = stringField(value, key);
  return field && allowed.includes(field) ? field as T[number] : null;
}

function isSha256(value: string | null): boolean {
  return value !== null && /^[a-f0-9]{64}$/.test(value);
}

/** Project one immutable persisted stress receipt through a strict safe allowlist. */
export function projectFoundationStressReceipt(result: unknown): FoundationStressReceiptProjection | null {
  const resultRecord = record(result);
  const data = resultRecord ? record(resultRecord.data) : null;
  if (!data || data.action !== "desktop_interact") return null;
  const receipt = record(data.interactionReceipt);
  if (!receipt || receipt.recipeId !== FOUNDATION_STRESS_RECIPE_ID) return null;

  const effectState = enumField(receipt, "effectState", EFFECT_STATES);
  const replayState = enumField(receipt, "replayState", REPLAY_STATES);
  const cleanupState = enumField(receipt, "cleanupState", CLEANUP_STATES);
  const verificationState = enumField(receipt, "verificationState", VERIFICATION_STATES);
  const entryGate = enumField(receipt, "entryGate", ENTRY_GATES);
  if (
    !effectState || !replayState || !cleanupState || !verificationState || !entryGate
    || !isSha256(stringField(receipt, "operationIdDigest"))
  ) return null;

  const promptDisposition = receipt.promptDisposition === null
    ? null
    : record(receipt.promptDisposition);
  if (receipt.promptDisposition !== null && !promptDisposition) return null;
  const promptState = promptDisposition
    ? enumField(promptDisposition, "state", PROMPT_STATES)
    : null;
  const promptReasonCode = promptDisposition
    ? enumField(promptDisposition, "reasonCode", PROMPT_REASON_CODES)
    : null;
  if (promptDisposition && (!promptState || !promptReasonCode)) return null;

  const humanHandoff = receipt.humanHandoff === null ? null : record(receipt.humanHandoff);
  if (receipt.humanHandoff !== null && !humanHandoff) return null;
  if (humanHandoff) {
    if (
      humanHandoff.state !== "ready"
      || humanHandoff.reason !== "effect_uncertain"
      || !stringField(humanHandoff, "handoffId")
      || "handoffUrl" in humanHandoff
    ) return null;
  }

  return {
    recipeId: FOUNDATION_STRESS_RECIPE_ID,
    operationIdentity: "recorded",
    effectState,
    replayState,
    cleanupState,
    verificationState,
    entryGate,
    ...(promptState ? { promptState } : {}),
    ...(promptReasonCode ? { promptReasonCode } : {}),
    ...(humanHandoff ? { handoffState: "ready" as const, handoffId: stringField(humanHandoff, "handoffId")! } : {}),
  };
}
