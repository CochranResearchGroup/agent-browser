"use client";
import { useBrowserOperation } from "@/components/use-browser-operation";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

type Choice = "prepare" | "reconcile" | "relaunch" | "pause_and_relaunch" | "retry_graceful_close" | "force_close" | "focus" | "restore_previous" | "cancel";
const labels: Record<Choice, string> = { prepare: "Review relaunch", reconcile: "Reconcile operation", relaunch: "Relaunch now", pause_and_relaunch: "Pause and relaunch", retry_graceful_close: "Retry graceful close", force_close: "Force close", focus: "Return to browser", restore_previous: "Restore previous mode", cancel: "Cancel operation" };

/** Retain the same operation through lost replies and explicit remediation. */
export function BrowserModeDialog({ open, onOpenChange, taskId, sessionName, profileId, browserId, mode, onRefresh }: { open: boolean; onOpenChange: (open: boolean) => void; taskId: string; sessionName: string; profileId?: string; browserId?: string; mode: "cdp" | "cdp_free"; onRefresh: () => Promise<void> }) {
  const state = useBrowserOperation(open, { action: "browser_session_mode_convert", kind: "mode_convert", schemaVersion: "agent-browser.mode-operation.v1", sessionId: taskId, sessionName, sourceProfileId: profileId, browserId }, onRefresh);
  const { operation, confirmed, setConfirmed, busy, loading, unknown, error } = state;
  const run = (choice: Choice) => state.run(choice, { mode: operation?.requestedMode ?? mode });
  const actions = unknown ? [{ action: "reconcile" as const, requiresConfirmation: false }] : operation?.actions ?? [{ action: "prepare" as const, requiresConfirmation: false }];
  return <Dialog open={open} onOpenChange={onOpenChange}><DialogContent style={{ maxWidth: 480 }} aria-describedby="browser-mode-description">
    <DialogHeader><DialogTitle>Change browser mode</DialogTitle><DialogDescription id="browser-mode-description">Review affected tasks before relaunch. The same operation is retained through uncertain results.</DialogDescription></DialogHeader>
    <p>Requested mode: {(operation?.requestedMode ?? mode) === "cdp" ? "CDP" : "No CDP"}</p>
    {operation?.effectiveMode && <p>Effective mode: {operation.effectiveMode === "cdp" ? "CDP" : "No CDP"}</p>}
    <p>Relaunch replaces the whole browser. Unsaved forms, downloads and page state may be lost. Logical tasks, profile and handoff links are retained. Native actions address the browser window.</p>
    {operation && <><p role="status">{operation.state ?? operation.phase}</p><p>Affected tasks</p><ul>{operation.affectedTaskIds?.map(id => <li key={id}>{id}</li>)}</ul><small>Operation: {operation.operationId}</small></>}
    {actions.some(action => action.requiresConfirmation) && <label><input type="checkbox" aria-label="Confirm the selected browser action" checked={confirmed} onChange={event => setConfirmed(event.target.checked)} />Confirm the selected browser action. Force close may lose buffered data. Confirm only the selected scope.</label>}
    {operation?.failure && <p role="alert">{operation.failure}. Use the available recovery action.</p>}
    {loading && <p role="status">Reading existing operations…</p>}
    {state.candidates.length > 1 && <div><p>Select an existing operation</p>{state.candidates.map(candidate => <button type="button" key={candidate.operationId} onClick={() => state.select(candidate)}>Resume {candidate.operationId}</button>)}</div>}
    {error && <><p role="alert">{error}</p>{!unknown && <button type="button" disabled={busy || loading} onClick={() => void state.inspect()}>Inspect existing operations</button>}</>}
    {operation && ["completed", "cancelled"].includes((operation.state ?? operation.phase ?? "").toLowerCase()) && <button type="button" onClick={state.startAnother}>Prepare another relaunch</button>}
    <div {...(busy ? { "data-mode-operation-busy": true } : {})}>{actions.filter((action): action is { action: Choice; requiresConfirmation: boolean } => Object.hasOwn(labels, action.action) && action.action !== "pause_and_relaunch").map(action => <button key={action.action} type="button" disabled={busy || loading || !state.readable || !!error && !unknown && !operation || (action.requiresConfirmation && !confirmed)} onClick={() => void run(action.action)}>{labels[action.action]}</button>)}</div>
  </DialogContent></Dialog>;
}
