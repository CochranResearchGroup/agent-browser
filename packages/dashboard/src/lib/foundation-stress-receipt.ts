export type FoundationStressReceiptProjection = {
  recipeId: string;
  operationIdentity: "recorded" | "not_recorded";
  effectState: string;
  replayState: string;
  cleanupState: string;
  verificationState: string;
  entryGate: string;
  promptState?: string;
  promptReasonCode?: string;
  handoffState?: string;
  handoffId?: string;
};

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function stringField(value: Record<string, unknown>, key: string): string | undefined {
  const field = value[key];
  return typeof field === "string" && field.trim() ? field : undefined;
}

/** Project one immutable persisted desktop-interaction result through a strict safe allowlist. */
export function projectFoundationStressReceipt(result: unknown): FoundationStressReceiptProjection | null {
  const resultRecord = record(result);
  const data = resultRecord ? record(resultRecord.data) : null;
  if (!data || data.action !== "desktop_interact") return null;
  const receipt = record(data.interactionReceipt);
  if (!receipt) return null;
  const recipeId = stringField(receipt, "recipeId");
  const effectState = stringField(receipt, "effectState");
  const replayState = stringField(receipt, "replayState");
  const cleanupState = stringField(receipt, "cleanupState");
  const verificationState = stringField(receipt, "verificationState");
  const entryGate = stringField(receipt, "entryGate");
  if (!recipeId || !effectState || !replayState || !cleanupState || !verificationState || !entryGate) return null;

  const promptDisposition = record(receipt.promptDisposition);
  const humanHandoff = record(receipt.humanHandoff);
  return {
    recipeId,
    operationIdentity: stringField(receipt, "operationIdDigest") ? "recorded" : "not_recorded",
    effectState,
    replayState,
    cleanupState,
    verificationState,
    entryGate,
    ...(promptDisposition && stringField(promptDisposition, "state")
      ? { promptState: stringField(promptDisposition, "state") }
      : {}),
    ...(promptDisposition && stringField(promptDisposition, "reasonCode")
      ? { promptReasonCode: stringField(promptDisposition, "reasonCode") }
      : {}),
    ...(humanHandoff && stringField(humanHandoff, "state")
      ? { handoffState: stringField(humanHandoff, "state") }
      : {}),
    ...(humanHandoff && stringField(humanHandoff, "handoffId")
      ? { handoffId: stringField(humanHandoff, "handoffId") }
      : {}),
  };
}
