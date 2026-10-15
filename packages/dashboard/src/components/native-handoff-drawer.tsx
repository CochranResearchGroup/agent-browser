"use client";

import { Fragment, useRef, useState } from "react";
import type { ServiceManagedHandoff } from "@agent-browser/client/service-observability";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

const supplied = (value: unknown) => typeof value === "string" && value.trim() ? value : "Unknown";
const duration = (value: unknown) => value === null ? "Perpetual" : typeof value === "number" && Number.isSafeInteger(value) && value >= 0
  ? `${value % 1000 === 0 ? `${value / 1000} s` : `${value} ms`} inactivity` : "Unknown";
function timestamp(value: unknown) {
  if (typeof value !== "number" || !Number.isFinite(value)) return "Unknown";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Unknown" : <time dateTime={date.toISOString()}>{date.toLocaleString()}</time>;
}
function durableUrl(record: ServiceManagedHandoff) {
  try {
    const url = new URL(record.handoffUrl, "https://agent-browser.invalid");
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash
      || !/^[A-Za-z0-9_-]+$/.test(record.handoffId) || url.pathname !== `/remote-view/${record.handoffId}`) return null;
    return record.handoffUrl;
  } catch { return null; }
}

/** Transient Share output is separate from the owner's retained-record inventory.
 * Opening, copying and inspecting a record never issues a new handoff or selects a task. */
export function NativeHandoffDrawer({ open, onOpenChange, busy, error, sharedLink, onReturnFocus, records, inventoryLoading, inventoryError, onRefresh }: {
  open: boolean; onOpenChange: (open: boolean) => void; busy: boolean; error: string | null;
  sharedLink: { url: string; label: string } | null; onReturnFocus: () => void;
  records?: readonly ServiceManagedHandoff[]; inventoryLoading?: boolean; inventoryError?: string | null;
  onRefresh: () => Promise<void>;
}) {
  const [copyFeedback, setCopyFeedback] = useState<{ id: string; failed: boolean } | null>(null);
  const [inspected, setInspected] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const refreshPending = useRef(false);
  const [refreshError, setRefreshError] = useState(false);
  const refresh = async () => {
    if (refreshPending.current) return;
    refreshPending.current = true; setRefreshing(true); setRefreshError(false);
    try { await onRefresh(); } catch { setRefreshError(true); }
    finally { refreshPending.current = false; setRefreshing(false); }
  };
  const copy = async (id: string, url: string) => {
    try { await navigator.clipboard.writeText(new URL(url, window.location.origin).toString()); setCopyFeedback({ id, failed: false }); }
    catch { setCopyFeedback({ id, failed: true }); }
  };
  return <Dialog open={open} onOpenChange={onOpenChange}>
    <DialogContent className="native-tab-properties native-handoff-drawer" onCloseAutoFocus={event => { event.preventDefault(); onReturnFocus(); }}>
      <DialogHeader><DialogTitle>Handoffs</DialogTitle><DialogDescription>Native Remote View links. Link validity and target lifetime are independent.</DialogDescription></DialogHeader>
      <button type="button" className="native-handoff-refresh" disabled={refreshing || inventoryLoading} onClick={() => void refresh()}>Refresh handoffs</button>
      {(refreshing || inventoryLoading) && <p role="status">Refreshing handoffs…</p>}
      {(inventoryError || refreshError) && <p role="alert">Could not refresh handoffs. Showing the last reported records.</p>}
      {busy && <p role="status">Preparing link…</p>}
      {error && <p role="alert">{error}</p>}
      {sharedLink && <section aria-label="Shared task">
        <p>{sharedLink.label}</p><label>Shared handoff URL<input aria-label="Shared handoff URL" readOnly value={sharedLink.url} /></label>
        <div className="native-tab-dialog-actions"><button type="button" onClick={() => void copy('shared', sharedLink.url)}>Copy link</button><a href={sharedLink.url} target="_blank" rel="noopener noreferrer">Open Remote View</a></div>
      </section>}
      {copyFeedback && <p role={copyFeedback.failed ? "alert" : "status"}>{copyFeedback.failed ? "Could not copy this link. Inspect it to copy the URL manually." : "Link copied"}</p>}
      {records === undefined && !inventoryLoading && <p role="status">Handoff inventory is unavailable. Refresh to check the current owner.</p>}
      {records?.length === 0 && <p role="status">No retained handoffs. Share a task to prepare a link.</p>}
      {records && records.length > 0 && <table aria-label="Retained handoffs" className="native-handoff-table">
        <thead><tr><th scope="col">Task and origin</th><th scope="col">Created and use</th><th scope="col">Lifetimes and target</th><th scope="col">Actions</th></tr></thead>
        <tbody>{records.map(record => {
          const metadata = record.taskMetadata;
          const url = durableUrl(record);
          const detailsOpen = inspected === record.handoffId;
          return <Fragment key={record.handoffId}><tr>
            <td><strong>{supplied(metadata?.taskName)}</strong><span><small>Service: </small><span>{supplied(metadata?.serviceName)}</span></span><span><small>Agent: </small><span>{supplied(metadata?.agentName)}</span></span><span><small>Human: </small><span>{supplied(metadata?.humanName)}</span></span><small>Origin: {supplied(metadata?.originKind)} · {supplied(metadata?.attribution)}</small></td>
            <td><span>Created: {timestamp(record.createdAtMs)}</span><span>{typeof metadata?.firstHumanUseAtMs === "number" ? "Used" : record.pendingHuman ? "Awaiting use" : "Use not recorded"}</span></td>
            <td><span>Link: {record.expiresAtMs === null ? "Perpetual" : timestamp(record.expiresAtMs)}</span><span>Link status: {record.linkState}</span><span>Tab: {record.tabLifetime ? duration(record.tabLifetime.inactivityMs) : "Unknown"}</span><span>Target: {record.targetState === 'reported' ? 'Reported' : record.targetState === 'expired' ? 'Expired' : 'Ended'}</span><span>Pending use: {record.pendingHuman ? record.pendingHuman.expiresAtMs === null ? 'Until used' : timestamp(record.pendingHuman.expiresAtMs) : 'None reported'}</span></td>
            <td className="native-handoff-actions">
              {url && <a href={url} target="_blank" rel="noopener noreferrer">Open</a>}
              <button type="button" disabled={!url} onClick={() => url && void copy(record.handoffId, url)}>Copy</button>
              <button type="button" aria-expanded={detailsOpen} onClick={() => setInspected(detailsOpen ? null : record.handoffId)}>Inspect</button>
              {!url && <span role="alert">Link unavailable</span>}
            </td>
          </tr>
          {detailsOpen && <tr className="native-handoff-detail-row"><td colSpan={4}><dl className="native-handoff-details">
                <div><dt>Link</dt><dd>{url ? new URL(url, window.location.origin).toString() : "Unavailable"}</dd></div><div><dt>Profile</dt><dd>{supplied(record.profileId)}</dd></div>
                <div><dt>Session</dt><dd>{supplied(record.sessionName)}</dd></div><div><dt>Purpose</dt><dd>{supplied(metadata?.purpose)}</dd></div>
                <div><dt>First human use</dt><dd>{timestamp(metadata?.firstHumanUseAtMs)}</dd></div>
              </dl></td></tr>}
          </Fragment>;
        })}</tbody>
      </table>}
    </DialogContent>
  </Dialog>;
}
