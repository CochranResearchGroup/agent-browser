"use client";
import { useRef, useState } from "react";
import { BrowserBuildPicker, type BrowserBuildEvidence, type BrowserBuildInspection } from "@/components/browser-build-picker";
import { BrowserModeDialog } from "@/components/browser-mode-dialog";
import { ProfileDuplicateDialog } from "@/components/profile-duplicate-dialog";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

/** Build, profile and mode remain separate choices; existing owner operations perform effects. */
export function BrowserBuildDialog({ open, onOpenChange, profileId, taskId, onRefresh, onChooseCopy }: {
  open: boolean; onOpenChange: (open: boolean) => void; profileId: string; taskId?: string | null;
  onRefresh: () => Promise<void>; onChooseCopy: (profileId: string) => void;
}) {
  const [build, setBuild] = useState<BrowserBuildEvidence | null>(null);
  const [inspection, setInspection] = useState<BrowserBuildInspection | null>(null);
  const [mode, setMode] = useState<"cdp" | "cdp_free">("cdp");
  const [modeChosen, setModeChosen] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const [child, setChild] = useState<"mode" | "copy" | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [receiptId, setReceiptId] = useState<string | null>(null);
  const [unknown, setUnknown] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const operationId = useRef(crypto.randomUUID());
  const pending = useRef(false);
  const task = inspection?.activeTasks.find(task => task.id === taskId) ?? inspection?.activeTasks[0];
  const receive = (next: BrowserBuildInspection | null) => {
    setInspection(next); setConfirmed(false);
    if (next) { if (!modeChosen) setMode(next.nextLaunchMode); setReceiptId(next.currentSelectionOperationId ?? null); setUnknown(false); }
  };
  const select = async (choice: "select_for_next_launch" | "open_selected") => {
    if (pending.current || !inspection || (choice === "select_for_next_launch" && (!build || !confirmed)) || (choice === "open_selected" && !receiptId)) return;
    pending.current = true; setBusy(true); setError(null); setStatus(null);
    try {
      const requestId = choice === "open_selected" ? receiptId : operationId.current;
      const response = await fetch("/api/command", { method: "POST", headers: { "Content-Type": "application/json" }, signal: AbortSignal.timeout(15000), body: JSON.stringify({ action: "browser_session_build_select", choice, profileId, operationId: requestId, ...(choice === "select_for_next_launch" ? { ...build!.selection, mode, ownerConfirmed: confirmed } : {}) }) });
      const reply = await response.json();
      if (!response.ok || reply.success !== true || reply.data?.operationId !== requestId || reply.data?.schemaVersion !== "agent-browser.build-selection.v1") throw new Error(typeof reply.error === "string" ? reply.error : "Outcome could not be confirmed.");
      setStatus(reply.data.phase); setUnknown(false);
      if (choice === "select_for_next_launch") { setReceiptId(requestId); setInspection({ ...inspection, nextLaunchBuild: build!, nextLaunchMode: mode, currentSelectionOperationId: requestId! }); setConfirmed(false); operationId.current = crypto.randomUUID(); }
      await onRefresh().catch(() => setError("Inventory could not refresh. Read browser builds again."));
    } catch (error) { setUnknown(true); setError(`${error instanceof Error ? error.message : "Unknown result"} Refresh browser builds to read retained selection and active tasks before another action.`); }
    finally { pending.current = false; setBusy(false); }
  };
  return <><Dialog open={open && child === null} onOpenChange={onOpenChange}><DialogContent style={{ maxWidth: 560, maxHeight: "90vh", overflowY: "auto" }}>
    <DialogHeader><DialogTitle>Choose browser build</DialogTitle><DialogDescription>Review the build and profile evidence before changing a running browser or its next launch.</DialogDescription></DialogHeader>
    {open && <BrowserBuildPicker key={profileId} profileId={profileId} onChoose={next => { setBuild(next); setConfirmed(false); }} onInspect={receive} />}
    <fieldset disabled={busy}><legend>Launch mode</legend>{(["cdp", "cdp_free"] as const).map(value => <label key={value}><input type="radio" name={`build-mode-${profileId}`} checked={mode === value} onChange={() => { setMode(value); setModeChosen(true); setConfirmed(false); }} />{value === "cdp" ? "CDP" : "No CDP"}</label>)}</fieldset>
    {inspection?.activeOperationId && <p role="status">Continue active operation {inspection.activeOperationId} from the browser menu before starting another operation.</p>}
    <button type="button" disabled={!build || busy || unknown} onClick={() => setChild("copy")}>Duplicate and test selected build</button>
    {task && <button type="button" disabled={!build || busy || unknown} onClick={() => setChild("mode")}>Review original profile relaunch</button>}
    {!task && <><label><input type="checkbox" aria-label="Confirm build migration and login risk" checked={confirmed} disabled={busy || unknown} onChange={event => setConfirmed(event.target.checked)} />Confirm migration and login risk for this profile.</label><button type="button" disabled={!build || !confirmed || busy || unknown || !!inspection?.activeOperationId} onClick={() => void select("select_for_next_launch")}>Select for next launch</button></>}
    {receiptId && <p>Saved build: {inspection?.nextLaunchBuild?.name ?? "Unverified"} {inspection?.nextLaunchBuild?.version ?? ""}; saved mode: {inspection?.nextLaunchMode === "cdp_free" ? "No CDP" : "CDP"}. Operation: {receiptId}. Chooser changes take effect only after selection is confirmed.</p>}
    {receiptId && <button type="button" disabled={busy || unknown || !!inspection?.activeOperationId} onClick={() => void select("open_selected")}>Open saved build selection</button>}
    {inspection?.activeTasks.map(task => <p key={task.id}>Active task: {task.name} ({task.id})</p>)}
    {status && <p role="status">{status}. Login and profile readability require separate verification in the browser.</p>}
    {error && <p role="alert">{error}</p>}
  </DialogContent></Dialog>
  {task && child === "mode" && <BrowserModeDialog open onOpenChange={value => { if (!value) setChild(null); }} taskId={task.id} sessionName={task.name} profileId={profileId} browserId={task.browserId} mode={mode} build={build ?? undefined} onRefresh={onRefresh} />}
  {child === "copy" && <ProfileDuplicateDialog open onOpenChange={value => { if (!value) setChild(null); }} profileId={profileId} taskId={task?.id} sessionName={task?.name} browserId={task?.browserId} build={build ?? undefined} mode={mode} onRefresh={onRefresh} onChooseCopy={id => { setChild(null); onOpenChange(false); onChooseCopy(id); }} />}
  </>;
}
