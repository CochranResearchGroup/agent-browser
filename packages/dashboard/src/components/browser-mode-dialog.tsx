"use client";
import { useState } from "react";
import { BrowserBuildPicker, type BrowserBuildEvidence } from "@/components/browser-build-picker";
import { useBrowserOperation } from "@/components/use-browser-operation";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

type Choice = "prepare" | "reconcile" | "relaunch" | "pause_and_relaunch" | "retry_graceful_close" | "force_close" | "focus" | "restore_previous" | "cancel";
const labels: Record<Choice, string> = { prepare: "Review relaunch", reconcile: "Reconcile operation", relaunch: "Relaunch now", pause_and_relaunch: "Pause before relaunch", retry_graceful_close: "Retry graceful close", force_close: "Force close", focus: "Return to browser", restore_previous: "Restore previous build and mode", cancel: "Cancel operation" };

/** Retain the same operation through lost replies and explicit remediation. */
export function BrowserModeDialog({ open, onOpenChange, taskId, sessionName, profileId, browserId, mode, build, onRefresh }: { open: boolean; onOpenChange: (open: boolean) => void; taskId: string; sessionName: string; profileId?: string; browserId?: string; mode: "cdp" | "cdp_free"; build?: BrowserBuildEvidence; onRefresh: () => Promise<void> }) {
  const [alternative, setAlternative] = useState<BrowserBuildEvidence | null>(null);
  const state = useBrowserOperation(open, { action: "browser_session_mode_convert", kind: "mode_convert", schemaVersion: "agent-browser.mode-operation.v1", sessionId: taskId, sessionName, sourceProfileId: profileId, browserId }, onRefresh);
  const { operation, confirmed, setConfirmed, busy, loading, unknown, error } = state;
  const run = (choice: Choice) => state.run(choice, { mode: operation?.requestedMode ?? mode, ...(!operation && choice === "prepare" ? build?.selection : {}) });
  const actions = unknown ? [{ action: "reconcile" as const, requiresConfirmation: false }] : operation?.actions ?? [{ action: "prepare" as const, requiresConfirmation: false }];
  return <Dialog open={open} onOpenChange={onOpenChange}><DialogContent style={{ maxWidth: 540, maxHeight: "90vh", overflowY: "auto" }} aria-describedby="browser-mode-description">
    <DialogHeader><DialogTitle>Change browser mode</DialogTitle><DialogDescription id="browser-mode-description">Review affected tasks before relaunch. The same operation is retained through uncertain results.</DialogDescription></DialogHeader>
    <p>Requested mode: {(operation?.requestedMode ?? mode) === "cdp" ? "CDP" : "No CDP"}</p>
    {operation?.effectiveMode && <p>Effective mode: {operation.effectiveMode === "cdp" ? "CDP" : "No CDP"}</p>}
    {(operation?.selectedBuild ?? build) && <p>Selected build: {(operation?.selectedBuild ?? build)?.name} {(operation?.selectedBuild ?? build)?.version}. Profile readability, credential storage and login portability: Unverified.</p>}
    {operation?.effectiveBuild && <p>Effective executable: {operation.effectiveBuild.name} {operation.effectiveBuild.version}</p>}
    {(build || operation?.selectedBuild) && <p>Build changes may migrate profile data or lose login. Restoring the previous executable does not restore profile bytes. Duplicate and test first, or cancel and choose another build.</p>}
    <p>Relaunch replaces the whole browser. Unsaved forms, downloads and page state may be lost. Logical tasks, profile and handoff links are retained. Native actions address the browser window.</p>
    {operation && <><p role="status">{operation.state ?? operation.phase}</p><p>Affected tasks</p><ul>{operation.affectedTaskIds?.map(id => <li key={id}>{id}</li>)}</ul><small>Operation: {operation.operationId}</small></>}
    {actions.some(action => action.requiresConfirmation) && <label><input type="checkbox" aria-label="Confirm the selected browser action" checked={confirmed} onChange={event => setConfirmed(event.target.checked)} />Confirm the selected browser action. Force close may lose buffered data. Confirm only the selected scope.</label>}
    {profileId && actions.some(action => action.action === "select_build") && <div><p>Choose another executable for this same operation. Selection does not launch it.</p><BrowserBuildPicker profileId={profileId} onChoose={setAlternative} /><button type="button" disabled={!alternative || !confirmed || busy || loading || unknown} onClick={() => void state.run("select_build", { mode: operation?.requestedMode ?? mode, ...alternative?.selection })}>Use another build in this operation</button></div>}
    {operation?.failure && <p role="alert">{operation.failure}. Use the available recovery action.</p>}
    {loading && <p role="status">Reading existing operations…</p>}
    {state.historyTruncated && !loading && <p role="status">Showing bounded operation history. Current and pending operations appear first.</p>}
    {state.candidates.length > 1 && <div><p>Select an existing operation</p><div role="region" aria-label="Existing browser operations" style={{ maxHeight: "min(16rem, 40vh)", overflowY: "auto" }}>{state.candidates.map(candidate => <button type="button" key={candidate.operationId} onClick={() => state.select(candidate)}>Resume {candidate.operationId}</button>)}</div></div>}
    {error && <><p role="alert">{error}</p>{!unknown && <button type="button" disabled={busy || loading} onClick={() => void state.inspect()}>Inspect existing operations</button>}</>}
    {operation && ["completed", "cancelled"].includes((operation.state ?? operation.phase ?? "").toLowerCase()) && <button type="button" onClick={state.startAnother}>Prepare another relaunch</button>}
    <div {...(busy ? { "data-mode-operation-busy": true } : {})}>{actions.filter((action): action is { action: Choice; requiresConfirmation: boolean } => Object.hasOwn(labels, action.action)).map(action => <button key={action.action} type="button" disabled={busy || loading || !state.readable || !!error && !unknown && !operation || (action.requiresConfirmation && !confirmed)} onClick={() => void run(action.action)}>{labels[action.action]}</button>)}</div>
  </DialogContent></Dialog>;
}
