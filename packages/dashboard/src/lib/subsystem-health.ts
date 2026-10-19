/** Health evidence is bounded to its source; reachability never proves a viewer. */
export type HealthState = "healthy" | "attention" | "failed" | "unknown";
export type SubsystemHealth = { id: string; label: string; state: HealthState; detail: string; checkedAt?: number };
export const HEALTH_FRESH_MS = 30_000;
export function currentHealth(row: SubsystemHealth, now: number): SubsystemHealth {
  if (row.checkedAt !== undefined && now - row.checkedAt >= HEALTH_FRESH_MS) {
    return { ...row, state: "unknown", detail: "Previous check is stale. Refresh health to check again." };
  }
  return row;
}
export function overallHealth(rows: readonly SubsystemHealth[]): HealthState {
  if (rows.some(row => row.state === "failed")) return "failed";
  if (rows.some(row => row.state === "attention")) return "attention";
  if (!rows.length || rows.some(row => row.state === "unknown")) return "unknown";
  return "healthy";
}
export function runtimeHealthRow(value: unknown, checkedAt: number): SubsystemHealth {
  const row = { id: "runtime", label: "Browser runtime", checkedAt };
  if (!value || typeof value !== "object") return { ...row, state: "unknown", detail: "Runtime health response was not recognized." };
  const health = value as { ready?: boolean; state?: string; observedAtEpochMs?: number; issues?: Array<{ code?: string }> };
  if (!Number.isFinite(health.observedAtEpochMs) || health.observedAtEpochMs! > checkedAt + 1000 || checkedAt - health.observedAtEpochMs! >= HEALTH_FRESH_MS) {
    return { ...row, state: "unknown", detail: "Runtime diagnostics have no current observation timestamp. Refresh or review Service diagnostics." };
  }
  const codes = [...new Set((Array.isArray(health.issues) ? health.issues : []).map(issue => issue?.code)
    .filter((code): code is string => typeof code === "string" && /^[a-z0-9_.:-]{1,96}$/i.test(code)))].slice(0, 5);
  const findings = codes.length ? ` Findings: ${codes.join(", ")}.` : "";
  row.checkedAt = Math.min(checkedAt, health.observedAtEpochMs!);
  return { ...row, state: health.ready === true && health.state === "ready" ? "healthy" : health.ready === false || health.state === "degraded" ? "attention" : "unknown",
    detail: health.ready === true && health.state === "ready" ? "Runtime diagnostics report ready. This does not verify Native desktop delivery." : `Runtime readiness is not confirmed.${findings} Review Service diagnostics for owner-reported findings.` };
}
export function nativeHealthRow(value: unknown, checkedAt: number): SubsystemHealth {
  const row = { id: "native", label: "Remote View desktops", checkedAt };
  if (!Array.isArray(value) || !value.length || value.some(slot => !slot || typeof slot.slot !== "number" || typeof slot.state !== "string")) {
    return { ...row, state: "unknown", detail: "Native desktop health response was empty or unrecognized." };
  }
  const ready = value.filter(slot => slot.state === "ready").length;
  const unavailable = value.filter(slot => slot.state !== "ready").map(slot => slot.slot).sort((a, b) => a - b);
  return { ...row, state: ready === value.length ? "healthy" : ready ? "attention" : "failed",
    detail: `${ready} of ${value.length} desktops pass Native readiness.${unavailable.length ? ` Unavailable: ${unavailable.join(", ")}. They may be stopped or failing verification.` : ""} This check does not prove an authenticated display connection.` };
}
