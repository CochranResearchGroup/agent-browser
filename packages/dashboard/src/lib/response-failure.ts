/** A projection of owner failure metadata, never a new authorization decision. */
export function responseFailureMetadata(payload: unknown): { code: string; observationId?: string } | null {
  if (!payload || typeof payload !== "object") return null;
  const envelope = payload as Record<string, any>;
  if (envelope.success !== false && envelope.ok !== false && envelope.data?.ok !== false && envelope.data?.success !== false) return null;
  const failure = envelope.failure ?? envelope.data?.failure ?? envelope.error?.data?.failure;
  const candidate = failure?.code ?? envelope.data?.code ?? envelope.code;
  const occurrence = failure?.occurrenceId ?? envelope.data?.occurrenceId ?? envelope.occurrenceId;
  return {
    code: typeof candidate === "string" && /^[a-z][a-z0-9_]{0,127}$/.test(candidate) ? candidate : "application_failure",
    ...(typeof occurrence === "string" && /^[a-zA-Z0-9_-]{1,160}$/.test(occurrence) ? { observationId: occurrence } : {}),
  };
}
