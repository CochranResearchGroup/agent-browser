"use client";

import { useState } from "react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

/** Transient Share output is separate from the owner's retained-record inventory. */
export function NativeHandoffDrawer({ open, onOpenChange, busy, error, sharedLink, onReturnFocus }: {
  open: boolean; onOpenChange: (open: boolean) => void; busy: boolean; error: string | null;
  sharedLink: { url: string; label: string } | null; onReturnFocus: () => void;
}) {
  const [copyError, setCopyError] = useState(false);
  const copy = async () => {
    if (!sharedLink) return;
    try { await navigator.clipboard.writeText(sharedLink.url); setCopyError(false); }
    catch { setCopyError(true); }
  };
  return <Dialog open={open} onOpenChange={onOpenChange}>
    <DialogContent className="native-tab-properties native-handoff-drawer" onCloseAutoFocus={event => { event.preventDefault(); onReturnFocus(); }}>
      <DialogHeader><DialogTitle>Handoffs</DialogTitle><DialogDescription>Share a task through its native Remote View link.</DialogDescription></DialogHeader>
      {busy && <p role="status">Preparing link…</p>}
      {error && <p role="alert">{error}</p>}
      {sharedLink && <section aria-label="Shared task">
        <p>{sharedLink.label}</p><label>Shared handoff URL<input aria-label="Shared handoff URL" readOnly value={sharedLink.url} /></label>
        <div className="native-tab-dialog-actions"><button type="button" onClick={() => void copy()}>Copy link</button><a href={sharedLink.url} target="_blank" rel="noopener noreferrer">Open Remote View</a></div>
        {copyError && <p role="alert">Could not copy this link. Select the URL to copy it manually.</p>}
      </section>}
    </DialogContent>
  </Dialog>;
}
