"use client";

import { ExternalLink, Copy, Info, Clock3, ArrowDown, ArrowUp, ArrowUpDown, Search, X } from "lucide-react";
import { NativeHandoffDelete } from "@/components/native-handoff-delete";
import { NativeHandoffPolicies } from "@/components/native-handoff-policies";
import { Fragment, useRef, useState, type CSSProperties } from "react";
import type { ServiceManagedHandoff } from "@agent-browser/client/service-observability";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

const recorded = (value: unknown): string | null => typeof value === "string" && value.trim() ? value.trim() : null;
const supplied = (value: unknown) => recorded(value) ?? "Not recorded";
function handoffTitle(record: ServiceManagedHandoff) {
  return recorded(record.taskMetadata?.taskName) ?? recorded(record.taskMetadata?.purpose)
    ?? recorded(record.profileId) ?? recorded(record.sessionName) ?? "Browser handoff";
}
const duration = (value: unknown) => value === null ? "Perpetual" : typeof value === "number" && Number.isSafeInteger(value) && value >= 0
  ? `${value % 1000 === 0 ? `${value / 1000} s` : `${value} ms`} inactivity` : "Not recorded";
function timestamp(value: unknown, compact = false) {
  if (typeof value !== "number" || !Number.isFinite(value)) return "Not recorded";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Not recorded" : <time dateTime={date.toISOString()} title={date.toLocaleString()}>{compact ? date.toLocaleDateString(undefined, { month: "short", day: "numeric" }) : date.toLocaleString()}</time>;
}
function durableUrl(record: ServiceManagedHandoff) {
  try {
    const url = new URL(record.handoffUrl, "https://agent-browser.invalid");
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash
      || !/^[A-Za-z0-9_-]+$/.test(record.handoffId) || url.pathname !== `/remote-view/${record.handoffId}`) return null;
    return record.handoffUrl;
  } catch { return null; }
}

type SortColumn = "task" | "created" | "lifetime";
const sortLabels: Record<SortColumn, string> = { task: "Task", created: "Created", lifetime: "Link status" };
function sortValue(record: ServiceManagedHandoff, column: SortColumn): string | number {
  if (column === "task") return handoffTitle(record).toLocaleLowerCase();
  if (column === "created") return record.createdAtMs ?? -1;
  return record.expiresAtMs ?? Number.POSITIVE_INFINITY;
}

/** Transient Share output is separate from the owner's retained-record inventory.
 * Missing creator fields retain useful profile/session context without inventing identity.
 * Opening, copying and inspecting a record never issues a new handoff or selects a task. */
export function NativeHandoffDrawer({ open, onOpenChange, busy, error, sharedLink, onReturnFocus, records, inventoryLoading, inventoryError, onRefresh, railWidth = 224 }: {
  open: boolean; onOpenChange: (open: boolean) => void; busy: boolean; error: string | null;
  sharedLink: { url: string; label: string } | null; onReturnFocus: () => void;
  records?: readonly ServiceManagedHandoff[]; inventoryLoading?: boolean; inventoryError?: string | null;
  onRefresh: () => Promise<void>; railWidth?: number;
}) {
  const [deleted, setDeleted] = useState<Set<string>>(new Set());
  const [deletionNotice, setDeletionNotice] = useState(false);
  const refreshButton = useRef<HTMLButtonElement>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const [showInactive, setShowInactive] = useState(false);
  const [query, setQuery] = useState("");
  const searchTerms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const [sort, setSort] = useState<{ column: SortColumn; descending: boolean }>({ column: "created", descending: true });
  // Link validity is independent of whether an expired task can be recreated.
  const retainedRecords = records?.filter(record => !deleted.has(record.handoffId));
  const inactiveCount = retainedRecords?.filter(record => record.linkState !== "active").length ?? 0;
  const visibleRecords = retainedRecords?.filter(record => showInactive || record.linkState === "active").filter(record => {
    // Include collapsed context so people can find a link without opening every row.
    const searchable = [handoffTitle(record), record.profileId, record.sessionName, record.handoffId,
      record.handoffUrl, record.linkState, ...Object.values(record.taskMetadata ?? {})]
      .filter(value => typeof value === "string").join(" ").toLocaleLowerCase();
    return searchTerms.every(term => searchable.includes(term));
  }).sort((left, right) => {
    const a = sortValue(left, sort.column), b = sortValue(right, sort.column);
    const compared = typeof a === "string" && typeof b === "string" ? a.localeCompare(b) : a < b ? -1 : a > b ? 1 : 0;
    return (sort.descending ? -compared : compared) || left.handoffId.localeCompare(right.handoffId);
  });
  const visibleSharedLink = sharedLink && ![...deleted].some(id => sharedLink.url.endsWith(`/remote-view/${id}`)) ? sharedLink : null;
  const [copyFeedback, setCopyFeedback] = useState<{ id: string; failed: boolean } | null>(null);
  const [inspected, setInspected] = useState<string | null>(null);
  const [policies, setPolicies] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const refreshPending = useRef(false);
  const [refreshError, setRefreshError] = useState(false);
  const refresh = async () => {
    if (refreshPending.current) return;
    refreshPending.current = true; setRefreshing(true); setRefreshError(false);
    try { await onRefresh(); } catch { setRefreshError(true); }
    finally { refreshPending.current = false; setRefreshing(false); }
  };
  const linkDeleted = (id: string) => {
    // Only an exact owner acknowledgement suppresses a stale revoked row.
    // This is transient display state, never a persistent link registry.
    setDeleted(current => new Set(current).add(id)); setDeletionNotice(true); setCopyFeedback(null);
    if (inspected === id) setInspected(null);
    if (policies === id) setPolicies(null);
    void refresh();
    requestAnimationFrame(() => refreshButton.current?.focus());
  };
  const copy = async (id: string, url: string) => {
    try { await navigator.clipboard.writeText(new URL(url, window.location.origin).toString()); setCopyFeedback({ id, failed: false }); }
    catch { setCopyFeedback({ id, failed: true }); }
  };
  return <Dialog open={open} onOpenChange={onOpenChange}>
    <DialogContent style={{ "--native-handoff-rail-width": `${railWidth}px` } as CSSProperties} className="native-tab-properties native-handoff-drawer" onCloseAutoFocus={event => { event.preventDefault(); onReturnFocus(); }}>
      <DialogHeader><DialogTitle>Handoffs</DialogTitle><DialogDescription>Find a browser task, then open or copy its link.</DialogDescription></DialogHeader>
      <div className="native-handoff-toolbar">
        <div className="native-handoff-search"><Search size={15} aria-hidden="true" />
          <input ref={searchInput} type="search" aria-label="Search handoffs" placeholder="Search handoffs…" value={query} onChange={event => setQuery(event.target.value)} />
          {query && <button type="button" aria-label="Clear search" onClick={() => { setQuery(""); searchInput.current?.focus(); }}><X size={14} aria-hidden="true" /></button>}
        </div>
        <button ref={refreshButton} type="button" className="native-handoff-refresh" aria-label="Refresh handoffs" disabled={refreshing || inventoryLoading} onClick={() => void refresh()}>Refresh</button>
      </div>
      <div className="native-handoff-list-options">
        {visibleRecords && <span role="status">{visibleRecords.length} {visibleRecords.length === 1 ? "handoff" : "handoffs"}</span>}
        <label className="native-handoff-filter"><input type="checkbox" checked={showInactive} onChange={event => setShowInactive(event.target.checked)} />Show inactive ({inactiveCount})</label>
      </div>
      {(refreshing || inventoryLoading) && <p role="status">Refreshing handoffs…</p>}
      {(inventoryError || refreshError) && <p role="alert">Could not refresh handoffs. Showing the last reported records.</p>}
      {busy && <p role="status">Preparing link…</p>}
      {error && <p role="alert">{error}</p>}
      {visibleSharedLink && <section aria-label="Shared task">
        <p>{visibleSharedLink.label}</p><label>Shared handoff URL<input aria-label="Shared handoff URL" readOnly value={visibleSharedLink.url} /></label>
        <div className="native-tab-dialog-actions"><button type="button" onClick={() => void copy('shared', visibleSharedLink.url)}>Copy link</button><a href={visibleSharedLink.url} target="_blank" rel="noopener noreferrer">Open Remote View</a></div>
      </section>}
      {deletionNotice && <p role="status">Link deleted.</p>}
      {copyFeedback && <p role={copyFeedback.failed ? "alert" : "status"}>{copyFeedback.failed ? "Could not copy this link. Inspect it to copy the URL manually." : "Link copied"}</p>}
      {records === undefined && !inventoryLoading && <p role="status">Handoff inventory is unavailable. Refresh to check the current owner.</p>}
      {visibleRecords?.length === 0 && <p role="status">{searchTerms.length ? "No matching handoffs. Try another search or show inactive links." : retainedRecords?.length ? "No active handoffs. Show inactive to inspect retained links." : "No retained handoffs. Share a task to prepare a link."}</p>}
      {visibleRecords && visibleRecords.length > 0 && <div className="native-handoff-results"><table aria-label="Retained handoffs" className="native-handoff-table">
        <thead><tr>{(Object.keys(sortLabels) as SortColumn[]).map(column => {
          const selected = sort.column === column;
          const Icon = selected ? sort.descending ? ArrowDown : ArrowUp : ArrowUpDown;
          const help = `Sort ${column === "task" ? "by task name" : column === "created" ? "by creation time" : "by link expiry"}, ${selected && !sort.descending ? "descending" : "ascending"}`;
          return <th key={column} scope="col" aria-sort={selected ? sort.descending ? "descending" : "ascending" : "none"}><button type="button" className="native-handoff-sort" title={help} aria-label={help} onClick={() => setSort({ column, descending: selected ? !sort.descending : false })}>{sortLabels[column]}<Icon size={12} aria-hidden="true" /></button></th>;
        })}<th scope="col">Actions</th></tr></thead>
        <tbody>{visibleRecords.map(record => {
          const metadata = record.taskMetadata;
          const inactive = record.linkState !== "active";
          const subtitle = [recorded(record.profileId) === handoffTitle(record) ? null : recorded(record.profileId),
            recorded(metadata?.humanName) ?? recorded(metadata?.agentName)].filter(Boolean).join(" · ");
          const url = durableUrl(record);
          const detailsOpen = inspected === record.handoffId;
          return <Fragment key={record.handoffId}><tr data-link-state={record.linkState}>
            <td><strong>{handoffTitle(record)}</strong>
              {subtitle && <span className="native-handoff-context">{subtitle}</span>}
            </td>
            <td>{timestamp(record.createdAtMs, true)}</td>
            <td><span className={`native-handoff-status${inactive ? " native-handoff-inactive" : ""}`}>{record.linkState === "active" ? "Active" : record.linkState === "expired" ? "Expired" : record.linkState}</span>
              <small>{record.expiresAtMs === null ? "Never expires" : <>Expires {timestamp(record.expiresAtMs, true)}</>}</small>
              {record.pendingHuman ? <small>Awaiting use</small> : typeof metadata?.firstHumanUseAtMs === "number" && <small>Opened</small>}
              {record.targetState !== "reported" && <small>Task {record.targetState}</small>}
            </td>
            <td><div className="native-handoff-actions">
              {url && !inactive && <a href={url} target="_blank" rel="noopener noreferrer" aria-label="Open" title="Open"><ExternalLink size={14} aria-hidden="true" /></a>}
              <button type="button" aria-label="Copy" title="Copy" disabled={!url} onClick={() => url && void copy(record.handoffId, url)}><Copy size={14} aria-hidden="true" /></button>
              <button type="button" aria-label="Inspect" title="Inspect" aria-expanded={detailsOpen} onClick={() => setInspected(detailsOpen ? null : record.handoffId)}><Info size={14} aria-hidden="true" /></button>
              <button type="button" aria-label="Policies" title="Policies" disabled={!url} aria-expanded={policies === record.handoffId} onClick={() => setPolicies(policies === record.handoffId ? null : record.handoffId)}><Clock3 size={14} aria-hidden="true" /></button>
              <NativeHandoffDelete record={record} onDeleted={linkDeleted} />
              {!url && <span role="alert">Link unavailable</span>}
            </div></td>
          </tr>
          {detailsOpen && <tr className="native-handoff-detail-row"><td colSpan={4}><dl className="native-handoff-details">
                <div><dt>Link</dt><dd>{url ? new URL(url, window.location.origin).toString() : "Unavailable"}</dd></div><div><dt>Profile</dt><dd>{supplied(record.profileId)}</dd></div>
                <div><dt>Session</dt><dd>{supplied(record.sessionName)}</dd></div><div><dt>Purpose</dt><dd>{supplied(metadata?.purpose)}</dd></div>
                <div><dt>Service</dt><dd>{supplied(metadata?.serviceName)}</dd></div>
                <div><dt>Agent</dt><dd>{supplied(metadata?.agentName)}</dd></div><div><dt>Human</dt><dd>{supplied(metadata?.humanName)}</dd></div>
                <div><dt>Origin</dt><dd>{supplied(metadata?.originKind)}</dd></div><div><dt>Link ID</dt><dd>{record.handoffId}</dd></div>
                <div><dt>Created</dt><dd>{timestamp(record.createdAtMs)}</dd></div><div><dt>Link expiry</dt><dd>{record.expiresAtMs === null ? "Perpetual" : timestamp(record.expiresAtMs)}</dd></div>
                <div><dt>Tab retention</dt><dd>{record.tabLifetime ? duration(record.tabLifetime.inactivityMs) : "Default policy"}</dd></div>
                <div><dt>Task</dt><dd>{record.targetState === "reported" ? "Retained" : record.targetState}</dd></div>
                <div><dt>Pending use</dt><dd>{record.pendingHuman ? record.pendingHuman.expiresAtMs === null ? "Kept open until confirmed use" : timestamp(record.pendingHuman.expiresAtMs) : "None reported"}</dd></div>
                <div><dt>Context source</dt><dd>{supplied(metadata?.attribution)}</dd></div>
                <div><dt>First confirmed use</dt><dd>{timestamp(metadata?.firstHumanUseAtMs)}</dd></div>
              </dl></td></tr>}
          {policies === record.handoffId && <tr className="native-handoff-detail-row"><td colSpan={4}><NativeHandoffPolicies record={record} onUpdated={refresh} /></td></tr>}
          </Fragment>;
        })}</tbody>
      </table></div>}
    </DialogContent>
  </Dialog>;
}
