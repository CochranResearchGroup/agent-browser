"use client";

import { Trash2 } from "lucide-react";
import { useRef, useState } from "react";
import type { ServiceManagedHandoff } from "@agent-browser/client/service-observability";
import { postServiceRequest, requireServiceRequestSuccess } from "@agent-browser/client/service-request";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

/** Revoke one logical link through its existing owner. No tab, session, browser
 * or provider-close operation is issued. Inventory owns post-acknowledgement refresh. */
export function NativeHandoffDelete({ record, onDeleted }: { record: ServiceManagedHandoff; onDeleted: (id: string) => void }) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const pending = useRef(false);
  const confirmed = useRef(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const label = typeof record.taskMetadata?.taskName === "string" ? record.taskMetadata.taskName : "Unknown task";
  const remove = async () => {
    if (pending.current || failed) return;
    pending.current = true; setBusy(true);
    try {
      const response = requireServiceRequestSuccess(await postServiceRequest({
        baseUrl: window.location.origin, signal: AbortSignal.timeout(15000),
        request: { action: "service_remote_view_handoff_resolve", serviceName: "agent-browser-dashboard", agentName: "operator", taskName: "Delete handoff link", jobTimeoutMs: 10000,
          params: { handoffId: record.handoffId, handoffOperation: "delete" } },
      }));
      const data = response.data as Record<string, unknown> | undefined;
      if (data?.handoffId !== record.handoffId || data.status !== "deleted") throw new Error("unconfirmed_link_delete_receipt");
      confirmed.current = true; setOpen(false); onDeleted(record.handoffId);
    } catch { setFailed(true); }
    finally { pending.current = false; setBusy(false); }
  };
  return <>
    <button ref={trigger} type="button" aria-label="Delete link" title="Delete link" onClick={() => { confirmed.current = false; setFailed(false); setOpen(true); }}><Trash2 size={14} aria-hidden="true" /></button>
    <Dialog open={open} onOpenChange={next => { if (!pending.current) setOpen(next); }}>
      <DialogContent className="native-tab-properties" showCloseButton={!busy} onCloseAutoFocus={event => { event.preventDefault(); if (!confirmed.current) trigger.current?.focus(); }}>
        <DialogHeader><DialogTitle>Delete link?</DialogTitle><DialogDescription>This revokes this link only. Its tab and browser stay open.</DialogDescription></DialogHeader>
        <p>{label}</p><p>Link: {record.handoffId}</p>
        {busy && <p role="status">Deleting link…</p>}
        {failed && <p role="alert">Could not confirm deletion. Refresh handoffs before trying again.</p>}
        <div className="native-tab-dialog-actions"><button type="button" disabled={busy} onClick={() => setOpen(false)}>Cancel</button><button type="button" disabled={busy || failed} onClick={() => void remove()}>Delete link</button></div>
      </DialogContent>
    </Dialog>
  </>;
}
