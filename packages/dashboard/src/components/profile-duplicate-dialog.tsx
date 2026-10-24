"use client";
import { useRef, useState } from "react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

type Choice = "prepare" | "reconcile" | "open_source" | "focus_source" | "close_and_duplicate" | "cancel";
type Operation = { operationId: string; state?: string; phase?: string; affectedTaskIds?: string[]; copyComplete?: boolean; profilePublished?: boolean; actions?: { action: Choice; requiresConfirmation: boolean }[] };
const labels: Record<Choice, string> = { prepare: "Review duplication", reconcile: "Reconcile operation", open_source: "Open source for review", focus_source: "Return to source browser", close_and_duplicate: "Close source and duplicate", cancel: "Cancel operation" };

/** Retain the same operation through lost replies and explicit remediation. */
export function ProfileDuplicateDialog({ open, onOpenChange, profileId, onRefresh }: { open: boolean; onOpenChange: (open: boolean) => void; profileId: string; onRefresh: () => Promise<void> }) {
  const [operationId, setOperationId] = useState(() => crypto.randomUUID());
  const [sourceProfile, setSourceProfile] = useState(profileId);
  const [name, setName] = useState("Independent copy");
  const [launch, setLaunch] = useState(true);
  const [operation, setOperation] = useState<Operation | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const [unknown, setUnknown] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async (choice: Choice) => {
    if (pending.current || (!name.trim()) || (unknown && choice !== "reconcile")) return;
    pending.current = true; setBusy(true); setError(null);
    try {
      const response = await fetch("/api/command", { method: "POST", headers: { "Content-Type": "application/json" }, signal: AbortSignal.timeout(15000),
        body: JSON.stringify({ action: "browser_session_profile_duplicate", operationId, sourceProfileId: sourceProfile, destinationName: name.trim(), launch, choice, ownerConfirmed: confirmed }) });
      const reply = await response.json();
      if (!response.ok || reply.success !== true || reply.data?.operationId !== operationId || reply.data?.schemaVersion !== "agent-browser.profile-duplication.v1") throw new Error("profile_operation_readback_required");
      setOperation(reply.data); setUnknown(false); setConfirmed(false);
      if (reply.data.profilePublished === true) await onRefresh().catch(() => setError("The profile is published, but inventory could not refresh. Refresh browsers."));
    } catch { setUnknown(true); setError("The result could not be confirmed. Reconcile this operation before selecting another browser action."); }
    finally { pending.current = false; setBusy(false); }
  };
  const actions = unknown ? [{ action: "reconcile" as const, requiresConfirmation: false }] : operation?.actions ?? [{ action: "prepare" as const, requiresConfirmation: false }];
  return <Dialog open={open} onOpenChange={onOpenChange}><DialogContent style={{ maxWidth: 480 }} aria-describedby="profile-duplicate-description">
    <DialogHeader><DialogTitle>Duplicate browser profile</DialogTitle><DialogDescription id="profile-duplicate-description">Create an independent profile. Copies evolve separately; signing in is verified separately from copying.</DialogDescription></DialogHeader>
    <p>Source profile: {sourceProfile}</p>
    <label>Copy name<input aria-label="Copy name" value={name} disabled={busy || operation !== null || unknown} onChange={event => setName(event.target.value)} /></label>
    <label><input type="checkbox" checked={launch} disabled={busy || operation !== null || unknown} onChange={event => setLaunch(event.target.checked)} />Open the new copy</label>
    {operation && <><p role="status">{operation.state ?? operation.phase}</p><p>Affected tasks</p><ul>{operation.affectedTaskIds?.map(id => <li key={id}>{id}</li>)}</ul><small>Operation: {operation.operationId}</small></>}
    {actions.some(action => action.requiresConfirmation) && <label><input type="checkbox" aria-label="Confirm the selected browser action" checked={confirmed} onChange={event => setConfirmed(event.target.checked)} />Confirm the selected browser action. Finish unsaved work before closing the source.</label>}
    {error && <p role="alert">{error}</p>}
    {operation && ["completed", "cancelled"].includes((operation.state ?? operation.phase ?? "").toLowerCase()) && <button type="button" onClick={() => { setOperationId(crypto.randomUUID()); setSourceProfile(profileId); setOperation(null); setConfirmed(false); setUnknown(false); setError(null); }}>Prepare another copy</button>}
    <div {...(busy ? { "data-profile-operation-busy": true } : {})}>{actions.map(action => labels[action.action] && <button key={action.action} type="button" disabled={busy || (action.requiresConfirmation && !confirmed)} onClick={() => void run(action.action)}>{labels[action.action]}</button>)}</div>
  </DialogContent></Dialog>;
}
