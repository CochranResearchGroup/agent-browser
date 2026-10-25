"use client";
import { useCallback, useEffect, useRef, useState } from "react";

export type BrowserOperation = {
  schemaVersion: string; operationId: string; kind?: string; state?: string; phase?: string;
  profileId?: string; browserId?: string; taskId?: string; requestedMode?: "cdp" | "cdp_free"; effectiveMode?: "cdp" | "cdp_free";
  source?: { id: string }; destination?: { id: string; name: string }; launchRequested?: boolean;
  selectedBuild?: { name?: string; family?: string; version?: string }; effectiveBuild?: { name?: string; version?: string }; targetBuild?: { name?: string; version?: string }; migrationWarning?: string;
  affectedTaskIds?: string[]; copyComplete?: boolean; profilePublished?: boolean; failure?: string | null;
  actions?: { action: string; requiresConfirmation: boolean }[];
};
type Scope = { action: string; kind: "mode_convert" | "profile_duplicate"; schemaVersion: string; sessionId?: string; sessionName?: string; sourceProfileId?: string; browserId?: string };

/** Reload reads the owner's journal. No browser or credential data is persisted by the client. */
export function useBrowserOperation(open: boolean, scope: Scope, onRefresh: () => Promise<void>) {
  const [operation, setOperation] = useState<BrowserOperation | null>(null);
  const [candidates, setCandidates] = useState<BrowserOperation[]>([]);
  const [operationId, setOperationId] = useState(() => crypto.randomUUID());
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [unknown, setUnknown] = useState(false);
  const [readable, setReadable] = useState(false);
  const [historyTruncated, setHistoryTruncated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pending = useRef(false);
  const version = useRef(0);
  const key = JSON.stringify(scope);
  const accept = useCallback((next: BrowserOperation) => {
    setOperation(next); setOperationId(next.operationId); setCandidates([]); setConfirmed(false); setUnknown(false);
  }, []);
  const inspect = useCallback(async () => {
    const current = ++version.current; setLoading(true); setReadable(false); setHistoryTruncated(false); setError(null);
    try {
      const response = await fetch("/api/command", { method: "POST", headers: { "Content-Type": "application/json" }, signal: AbortSignal.timeout(15000), body: JSON.stringify({ ...JSON.parse(key), choice: "inspect" }) });
      const reply = await response.json();
      if (!response.ok || reply.success !== true || reply.data?.schemaVersion !== "agent-browser.browser-operations.v1" || reply.data.kind !== scope.kind || !Array.isArray(reply.data.operations)) throw new Error();
      if (scope.sessionId && reply.data.taskId !== scope.sessionId) throw new Error();
      if (scope.sourceProfileId && reply.data.profileId !== scope.sourceProfileId) throw new Error();
      if (scope.browserId && reply.data.browserId !== scope.browserId) throw new Error();
      const found = reply.data.operations as BrowserOperation[];
      if (found.some(next => next.schemaVersion !== scope.schemaVersion || !next.operationId)) throw new Error();
      if (current !== version.current) return;
      setReadable(true); setHistoryTruncated(reply.data.historyTruncated === true);
      if (found.length === 1) accept(found[0]);
      else { setCandidates(found); setOperation(null); setUnknown(false); if (!found.length) setOperationId(crypto.randomUUID()); }
    } catch { if (current === version.current) setError("Existing operations could not be read. Retry inspection before starting another browser operation."); }
    finally { if (current === version.current) setLoading(false); }
  }, [accept, key, scope.kind, scope.schemaVersion, scope.sessionId, scope.sourceProfileId, scope.browserId]);
  useEffect(() => { if (open) void inspect(); return () => { version.current += 1; }; }, [open, inspect]);
  const run = async (choice: string, params: Record<string, unknown>) => {
    if (pending.current || loading || !readable || candidates.length || (unknown && choice !== "reconcile")) return;
    pending.current = true; setBusy(true); setError(null);
    try {
      const response = await fetch("/api/command", { method: "POST", headers: { "Content-Type": "application/json" }, signal: AbortSignal.timeout(15000), body: JSON.stringify({ ...JSON.parse(key), ...params, operationId, choice, ownerConfirmed: confirmed }) });
      const reply = await response.json();
      const refusal = typeof reply.error === "string" ? reply.error.split(":", 1)[0] : "";
      if (reply.success === false && ["mode_operation_affected_tasks_changed", "mode_operation_task_binding_changed", "mode_operation_id_conflict", "mode_operation_profile_busy", "mode_operation_browser_busy", "profile_duplicate_operation_id_conflict", "profile_duplicate_task_profile_conflict", "profile_duplicate_owner_confirmation_required", "profile_duplicate_source_operation_busy", "browser_operation_task_binding_changed", "browser_operation_target_binding_changed"].includes(refusal)) {
        setError(reply.error); return;
      }
      if (!response.ok) throw new Error();
      if (reply.data?.schemaVersion === "agent-browser.browser-operations.v1" && reply.data.state === "busy") {
        const active = reply.data.activeOperation as BrowserOperation | undefined;
        if (active?.kind === scope.kind && active.schemaVersion === scope.schemaVersion) accept(active);
        else { setOperation(null); setCandidates([]); setError(`Continue ${active?.kind ?? "the active operation"} ${active?.operationId ?? ""} from its browser menu.`); }
        return;
      }
      if (reply.success !== true || reply.data?.operationId !== operationId || reply.data?.schemaVersion !== scope.schemaVersion) throw new Error();
      accept(reply.data);
      if (reply.data.phase === "completed" || reply.data.profilePublished === true) await onRefresh().catch(() => setError("The operation is published, but inventory could not refresh. Refresh browsers."));
      return reply.data as BrowserOperation;
    } catch { setUnknown(true); setError("The result could not be confirmed. Reconcile this operation before selecting another browser action."); }
    finally { pending.current = false; setBusy(false); }
  };
  return { operation, candidates, operationId, confirmed, setConfirmed, busy, loading, readable, historyTruncated, unknown, error, inspect, run, select: accept, startAnother: () => { setOperationId(crypto.randomUUID()); setOperation(null); setConfirmed(false); setUnknown(false); setError(null); } };
}
