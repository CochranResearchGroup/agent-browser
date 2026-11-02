"use client";
import { useState } from "react";
import type { BrowserBuildEvidence } from "@/components/browser-build-picker";
import { useBrowserOperation } from "@/components/use-browser-operation";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

type SourceObservation = { mode?: string; observedAtMs?: number; build?: { name?: string; version?: string; sha256?: string }; buildEvidence?: string };
type Choice = "prepare" | "reconcile" | "open_source" | "focus_source" | "close_and_duplicate" | "retry_launch" | "finish_without_opening" | "cancel";
const labels: Record<Choice, string> = { prepare: "Review duplication", reconcile: "Reconcile operation", open_source: "Open source for review", focus_source: "Return to source browser", close_and_duplicate: "Close source and duplicate", retry_launch: "Retry opening copy", finish_without_opening: "Finish without opening", cancel: "Cancel operation" };

/** Retain the same operation through lost replies and explicit remediation. */
export function ProfileDuplicateDialog({ open, onOpenChange, profileId, taskId, sessionName, browserId, build, mode, sourceBuild, sourceMode, onChooseCopy, onRefresh }: { open: boolean; onOpenChange: (open: boolean) => void; profileId: string; taskId?: string | null; sessionName?: string | null; browserId?: string; build?: BrowserBuildEvidence; mode?: "cdp" | "cdp_free"; sourceBuild?: BrowserBuildEvidence; sourceMode?: "cdp" | "cdp_free"; onChooseCopy?: (profileId: string) => void; onRefresh: () => Promise<void> }) {
  const [name, setName] = useState("Independent copy");
  const [launch, setLaunch] = useState(true);
  const state = useBrowserOperation(open, { action: "browser_session_profile_duplicate", kind: "profile_duplicate", schemaVersion: "agent-browser.profile-duplication.v1", sourceProfileId: profileId, sessionId: taskId ?? undefined, sessionName: sessionName ?? undefined, browserId }, onRefresh);
  const { operation, confirmed, setConfirmed, busy, loading, unknown, error } = state;
  const sourceProfile = profileId;
  const sourceObservation = (operation as { sourceObservation?: SourceObservation } | null)?.sourceObservation;
  const retainedSourceBuild = (operation?.source as { id: string; browserBuild?: string } | undefined)?.browserBuild;
  const run = (choice: Choice) => { if (name.trim()) void state.run(choice, { destinationName: operation?.destination?.name ?? name.trim(), launch: operation?.launchRequested ?? launch, ...(!operation && choice === "prepare" ? { ...build?.selection, ...(mode ? { mode } : {}) } : {}) }); };
  const actions = unknown ? [{ action: "reconcile" as const, requiresConfirmation: false }] : operation?.actions ?? [{ action: "prepare" as const, requiresConfirmation: false }];
  return <Dialog open={open} onOpenChange={onOpenChange}><DialogContent style={{ maxWidth: 540, maxHeight: "90vh", overflowY: "auto" }} aria-describedby="profile-duplicate-description">
    <DialogHeader><DialogTitle>Duplicate browser profile</DialogTitle><DialogDescription id="profile-duplicate-description">Create an independent profile. Copies evolve separately; signing in is verified separately from copying.</DialogDescription></DialogHeader>
    {(operation?.selectedBuild ?? build) && <p>Copy build: {(operation?.selectedBuild ?? build)?.name} {(operation?.selectedBuild ?? build)?.version}. Mode: {(operation?.requestedMode ?? mode) === "cdp_free" ? "No CDP" : (operation?.requestedMode ?? mode) === "cdp" ? "CDP" : "Unverified"}. Source preparation uses its configured build. Review source changes before authorizing close.</p>}
    {build && <p>Copying preserves files independently. Storage readability, credentials and login portability remain unverified. The target may migrate the copy; selecting an earlier executable does not restore profile bytes.</p>}
    <p>Source profile: {sourceProfile}</p>
    {operation && <p>Original source build: {retainedSourceBuild ?? "Unverified"}. Original source mode: {sourceObservation?.mode === "cdp_free" ? "No CDP" : sourceObservation?.mode === "cdp" ? "CDP" : "Unverified"}. Historical owner observation: {sourceObservation?.observedAtMs ?? "Not retained"}.</p>}
    {sourceObservation?.build && <p>Observed source executable: {sourceObservation.build.name ?? "Unverified browser"} {sourceObservation.build.version ?? "Unverified version"}. Build evidence: {sourceObservation.buildEvidence ?? "Unverified"}. This frozen observation is not current login proof.</p>}
    {sourceBuild && <p>Source configured build: {sourceBuild.name ?? "Unverified browser"} {sourceBuild.version ?? "Unverified version"}. Configured mode: {sourceMode === "cdp_free" ? "No CDP" : sourceMode === "cdp" ? "CDP" : "Unverified"}. This is configuration, not current process proof.</p>}
    <p>A consistent copy requires the owner to establish clean source closure. Review affected work and explicitly authorize the offered source action. Never close unrelated work.</p>
    <details open={operation?.copyComplete === true || !!operation?.failure}><summary>Copy experiment checkpoints</summary>
      <p>Copy completion: {operation?.copyComplete === true ? "Complete" : "Unverified"}</p>
      <p>Independent profile identity: {operation?.profilePublished && operation.destination?.id ? operation.destination.id : "Unverified"}</p>
      <p>Account identity: Unverified</p>
      <p>Authentication usability: Unverified</p>
      <p>Independent storage writes: Unverified</p>
      <small>Copy completion and publication do not verify site identity, authentication or storage divergence. Real-account qualification is a separate workflow.</small>
    </details>
    <label>Copy name<input aria-label="Copy name" value={name} disabled={busy || operation !== null || unknown} onChange={event => setName(event.target.value)} /></label>
    <label><input type="checkbox" checked={launch} disabled={busy || operation !== null || unknown} onChange={event => setLaunch(event.target.checked)} />Open the new copy</label>
    {operation && <><p role="status">{operation.state ?? operation.phase}</p><p>Affected tasks</p><ul>{operation.affectedTaskIds?.map(id => <li key={id}>{id}</li>)}</ul><small>Operation: {operation.operationId}</small></>}
    {actions.some(action => action.requiresConfirmation) && <label><input type="checkbox" aria-label="Confirm the selected browser action" checked={confirmed} onChange={event => setConfirmed(event.target.checked)} />Confirm the selected browser action. Finish unsaved work before closing the source.</label>}
    {loading && <p role="status">Reading existing operations…</p>}
    {state.historyTruncated && !loading && <p role="status">Showing bounded operation history. Current and pending operations appear first.</p>}
    {state.candidates.length > 1 && <div><p>Select an existing operation</p><div role="region" aria-label="Existing browser operations" style={{ maxHeight: "min(8rem, 18vh)", overflowY: "auto" }}>{state.candidates.map(candidate => <button type="button" key={candidate.operationId} onClick={() => state.select(candidate)}>Resume {candidate.destination?.name ?? candidate.operationId} ({candidate.operationId})</button>)}</div></div>}
    {operation?.failure && <p role="alert">Owner failure: {operation.failure}. Inspect or continue this original operation; copying is not verified by this failure.</p>}
    {error && <><p role="alert">{error}</p>{!unknown && <button type="button" disabled={busy || loading} onClick={() => void state.inspect()}>Inspect existing operations</button>}</>}
    {operation && ["completed", "cancelled"].includes((operation.state ?? operation.phase ?? "").toLowerCase()) && <button type="button" onClick={state.startAnother}>Prepare another copy</button>}
    {onChooseCopy && operation?.destination?.id && operation.profilePublished && ["completed", "cancelled"].includes((operation.state ?? operation.phase ?? "").toLowerCase()) && <button type="button" disabled={busy || unknown} onClick={() => onChooseCopy(operation.destination!.id)}>Choose another build for this copy</button>}
    <div {...(busy ? { "data-profile-operation-busy": true } : {})}>{actions.filter((action): action is { action: Choice; requiresConfirmation: boolean } => Object.hasOwn(labels, action.action)).map(action => <button key={action.action} type="button" disabled={busy || loading || !state.readable || !!error && !unknown && !operation || (action.requiresConfirmation && !confirmed)} onClick={() => void run(action.action)}>{labels[action.action]}</button>)}</div>
  </DialogContent></Dialog>;
}
