"use client";
import { useRef, useState } from "react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

type Choice = "prepare" | "reconcile" | "relaunch" | "pause_and_relaunch" | "retry_graceful_close" | "force_close" | "focus" | "cancel";
type Operation = { operationId: string; state?: string; phase?: string; affectedTaskIds?: string[]; failure?: string | null; actions?: { action: Choice; requiresConfirmation: boolean }[] };
const labels: Record<Choice, string> = { prepare: "Review relaunch", reconcile: "Reconcile operation", relaunch: "Relaunch now", pause_and_relaunch: "Pause and relaunch", retry_graceful_close: "Retry graceful close", force_close: "Force close", focus: "Return to browser", cancel: "Cancel operation" };

/** Retain the same operation through lost replies and explicit remediation. */
export function BrowserModeDialog({ open, onOpenChange, taskId, sessionName, mode, onRefresh }: { open: boolean; onOpenChange: (open: boolean) => void; taskId: string; sessionName: string; mode: "cdp" | "cdp_free"; onRefresh: () => Promise<void> }) {
  const [operationId, setOperationId] = useState(() => crypto.randomUUID());

  const [operation, setOperation] = useState<Operation | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const [unknown, setUnknown] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async (choice: Choice) => {
    if (pending.current || (unknown && choice !== "reconcile")) return;
    pending.current = true; setBusy(true); setError(null);
    try {
      const response = await fetch("/api/command", { method: "POST", headers: { "Content-Type": "application/json" }, signal: AbortSignal.timeout(15000),
        body: JSON.stringify({ action: "browser_session_mode_convert", sessionId: taskId, sessionName, operationId, mode, choice, ownerConfirmed: confirmed }) });
      const reply = await response.json();
      if (!response.ok || reply.success !== true || reply.data?.operationId !== operationId || reply.data?.schemaVersion !== "agent-browser.mode-operation.v1") throw new Error("mode_operation_readback_required");
      setOperation(reply.data); setUnknown(false); setConfirmed(false);
      if (reply.data.phase === "completed") await onRefresh().catch(() => setError("The mode changed, but inventory could not refresh. Refresh browsers."));
    } catch { setUnknown(true); setError("The result could not be confirmed. Reconcile this operation before selecting another browser action."); }
    finally { pending.current = false; setBusy(false); }
  };
  const actions = unknown ? [{ action: "reconcile" as const, requiresConfirmation: false }] : operation?.actions ?? [{ action: "prepare" as const, requiresConfirmation: false }];
  return <Dialog open={open} onOpenChange={onOpenChange}><DialogContent style={{ maxWidth: 480 }} aria-describedby="browser-mode-description">
    <DialogHeader><DialogTitle>Change browser mode</DialogTitle><DialogDescription id="browser-mode-description">Review affected tasks before relaunch. The same operation is retained through uncertain results.</DialogDescription></DialogHeader>
    <p>Requested mode: {mode === "cdp" ? "CDP" : "No CDP"}</p>
    <p>Relaunch replaces the whole browser. Unsaved forms, downloads and page state may be lost. Logical tasks, profile and handoff links are retained. Native actions address the browser window.</p>
    {operation && <><p role="status">{operation.state ?? operation.phase}</p><p>Affected tasks</p><ul>{operation.affectedTaskIds?.map(id => <li key={id}>{id}</li>)}</ul><small>Operation: {operation.operationId}</small></>}
    {actions.some(action => action.requiresConfirmation) && <label><input type="checkbox" aria-label="Confirm the selected browser action" checked={confirmed} onChange={event => setConfirmed(event.target.checked)} />Confirm the selected browser action. Force close may lose buffered data. Confirm only the selected scope.</label>}
    {operation?.failure && <p role="alert">{operation.failure}. Use the available recovery action.</p>}
    {error && <p role="alert">{error}</p>}
    {operation && ["completed", "cancelled"].includes((operation.state ?? operation.phase ?? "").toLowerCase()) && <button type="button" onClick={() => { setOperationId(crypto.randomUUID()); setOperation(null); setConfirmed(false); setUnknown(false); setError(null); }}>Prepare another relaunch</button>}
    <div {...(busy ? { "data-mode-operation-busy": true } : {})}>{actions.filter(action => action.action !== "pause_and_relaunch").map(action => labels[action.action] && <button key={action.action} type="button" disabled={busy || (action.requiresConfirmation && !confirmed)} onClick={() => void run(action.action)}>{labels[action.action]}</button>)}</div>
  </DialogContent></Dialog>;
}
