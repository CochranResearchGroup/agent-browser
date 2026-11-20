import {withClientErrorRecovery} from '../../../client/src/client-recovery.js';
/** A projection of owner failure metadata, never a new authorization decision. */
export function responseFailureMetadata(payload: unknown, fallback = "application_failure"): { code: string; observationId?: string } | null {
  if (!payload || typeof payload !== "object") return null;
  const envelope = payload as Record<string, any>;
  if (envelope.success !== false && envelope.ok !== false && envelope.data?.ok !== false && envelope.data?.success !== false) return null;
  const failure = envelope.failure ?? envelope.data?.failure ?? envelope.error?.data?.failure;
  const candidate = failure?.code ?? envelope.data?.code ?? envelope.error?.code ?? envelope.code;
  const occurrence = failure?.occurrenceId ?? envelope.data?.occurrenceId ?? envelope.occurrenceId;
  return {
    code: typeof candidate === "string" && /^[a-z][a-z0-9_]{0,127}$/.test(candidate) ? candidate : fallback,
    ...(typeof occurrence === "string" && /^[a-zA-Z0-9_-]{1,160}$/.test(occurrence) ? { observationId: occurrence } : {}),
  };
}

/** Carry the owner object for callers; logging uses only the projection above. */
export function dashboardResponseFailureError(payload: unknown, fallback: string): Error {
  const envelope = payload as Record<string, any> | null;
  const metadata = responseFailureMetadata(payload, fallback);
  const publicMessage = typeof envelope?.error === "string" && envelope.error.length <= 512
    ? envelope.error : null;
  return withClientErrorRecovery(Object.assign(new Error(publicMessage ?? metadata?.code ?? fallback), {
    code: metadata?.code ?? fallback,
    failure: envelope?.failure ?? envelope?.data?.failure ?? envelope?.error?.data?.failure ?? null,
    occurrenceId: metadata?.observationId ?? null,
    ...Object.fromEntries(["recoveryPlan","executableNextAction","recoveryRegistry","uncertainty","requestId"].map(key=>[key,envelope?.[key]??envelope?.data?.[key]??envelope?.error?.data?.[key]]).filter(([,value])=>value!==undefined)),
  }),{source:"packages/dashboard/src/lib/response-failure.ts",owningFunction:"dashboardResponseFailureError"},"client_dashboard_owner_failure");
}
