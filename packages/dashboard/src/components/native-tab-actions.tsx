"use client";

import { useRef, useState, type ReactNode } from "react";
import { MoreHorizontal } from "lucide-react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "@/components/ui/context-menu";
import type { WorkspaceNode, WorkspaceServiceTab } from "@/lib/service-workspaces";
import { postServiceRequest, requireServiceRequestSuccess, type ServiceTabCloseData } from "@agent-browser/client/service-request";

const supplied = (value: unknown): string => typeof value === "string" && value.trim() ? value : "Unknown";

/** Read supplied provenance without treating descriptive names as identity proof.
 * Opening Properties and copying the supplied URL have no browser, selection
 * or handoff effect. Clipboard failure offers manual copying through Properties. */
export function NativeTabActions({ node, tab, children, onClosed }: { node: WorkspaceNode; tab: WorkspaceServiceTab; children: ReactNode; onClosed?: () => Promise<void> }) {
  const [propertiesOpen, setPropertiesOpen] = useState(false);
  const [closeTarget, setCloseTarget] = useState<WorkspaceServiceTab | null>(null);
  const [closeBusy, setCloseBusy] = useState(false);
  const [closeError, setCloseError] = useState<string | null>(null);
  const closePending = useRef(false);
  const [copyFeedback, setCopyFeedback] = useState<"copied" | "failed" | null>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const label = tab.title || tab.taskLabel || tab.url || "Untitled tab";
  const metadata = tab.taskMetadata;
  const canClose = tab.managedTask === true && Boolean(tab.browserId && tab.targetId && (tab.ownerSessionId || tab.sessionId));
  const identity = (value: WorkspaceServiceTab) => JSON.stringify([value.id, value.browserId, value.targetId, value.ownerSessionId || value.sessionId]);
  const closeStillCurrent = closeTarget !== null && identity(closeTarget) === identity(tab);
  const openClose = () => { if (!closePending.current) { setCloseError(null); setCloseTarget({ ...tab }); } };
  const closeTab = async () => {
    if (!closeTarget || !closeStillCurrent || closePending.current || closeError) return;
    const target = closeTarget;
    closePending.current = true; setCloseBusy(true);
    try {
      const response = requireServiceRequestSuccess(await postServiceRequest({
        baseUrl: window.location.origin, signal: AbortSignal.timeout(15000),
        request: { action: "tab_close", browserId: target.browserId!, sessionName: target.ownerSessionId || target.sessionId!,
          serviceName: "agent-browser-dashboard", agentName: "operator", taskName: "Close native browser task", jobTimeoutMs: 10000,
          params: { targetId: target.targetId, operatorClose: true } },
      }));
      const data = response.data as Partial<ServiceTabCloseData> | undefined;
      if (data?.source !== "managed-owner" || data.closedTabId !== target.id || data.targetId !== target.targetId
          || data.physicalTabClosed !== true || data.unrelatedTargetsPreserved !== true) {
        throw new Error("unconfirmed_exact_close_receipt");
      }
      setCloseTarget(null);
      await onClosed?.();
    } catch {
      setCloseError("Close could not be confirmed. Refresh browsers and check close permission before trying again.");
    } finally { closePending.current = false; setCloseBusy(false); }
  };
  const copyUrl = async () => {
    if (!tab.url) return;
    try {
      await navigator.clipboard.writeText(tab.url);
      setCopyFeedback("copied");
    } catch { setCopyFeedback("failed"); }
  };
  const fields: [string, unknown][] = [
    ["Service", metadata?.serviceName], ["Agent", metadata?.agentName],
    ["Human", metadata?.humanName], ["Origin", metadata?.originKind],
    ["Task", metadata?.taskName], ["Purpose", metadata?.purpose],
    ["Attribution", metadata?.attribution], ["Browser", tab.browserId],
    ["Profile", tab.profileId ?? node.profileId], ["Desktop", tab.desktopId],
    ["URL", tab.url], ["Title", tab.title], ["Inventory status", tab.inventoryState ?? tab.lifecycle],
  ];
  return <>
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div className="native-workspace-tab-row" onContextMenu={event => {
          returnFocus.current = event.currentTarget.querySelector<HTMLButtonElement>(".native-workspace-tab");
        }}>
          {children}
          <DropdownMenu onOpenChange={open => { if (open) returnFocus.current = trigger.current; }}>
            <DropdownMenuTrigger asChild>
              <button ref={trigger} type="button" className="native-tab-actions" aria-label={`Tab actions for ${label}`}>
                <MoreHorizontal size={14} aria-hidden="true" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent className="native-tab-menu" align="end" onCloseAutoFocus={event => {
              if (propertiesOpen || closeTarget) event.preventDefault();
            }}>
              <DropdownMenuItem onSelect={() => setPropertiesOpen(true)}>Properties</DropdownMenuItem>
              <DropdownMenuItem disabled={!tab.url} onSelect={() => void copyUrl()}>Copy URL</DropdownMenuItem>
              <DropdownMenuItem disabled={!canClose || closeBusy} onSelect={openClose}>Close Tab</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent className="native-tab-menu" onCloseAutoFocus={event => {
        if (propertiesOpen || closeTarget) event.preventDefault();
      }}>
        <ContextMenuItem onSelect={() => setPropertiesOpen(true)}>Properties</ContextMenuItem>
        <ContextMenuItem disabled={!tab.url} onSelect={() => void copyUrl()}>Copy URL</ContextMenuItem>
        <ContextMenuItem disabled={!canClose || closeBusy} onSelect={openClose}>Close Tab</ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
    {copyFeedback && <p className="native-tab-feedback" role={copyFeedback === "failed" ? "alert" : "status"}>
      {copyFeedback === "copied" ? "URL copied" : "Could not copy this URL. Open Properties to copy it manually."}
    </p>}
    <Dialog open={propertiesOpen} onOpenChange={setPropertiesOpen}>
      <DialogContent className="native-tab-properties" onCloseAutoFocus={event => {
        event.preventDefault(); (returnFocus.current ?? trigger.current)?.focus();
      }}>
        <DialogHeader><DialogTitle>Tab properties</DialogTitle>
          <DialogDescription>Supplied task context and current inventory. Names are descriptive, not verified identity.</DialogDescription>
        </DialogHeader>
        <dl>{fields.map(([name, value]) => <div key={name}><dt>{name}</dt><dd>{supplied(value)}</dd></div>)}</dl>
      </DialogContent>
    </Dialog>
    <Dialog open={closeTarget !== null} onOpenChange={open => { if (!open && !closePending.current) setCloseTarget(null); }}>
      <DialogContent className="native-tab-properties" showCloseButton={!closeBusy} onCloseAutoFocus={event => {
        event.preventDefault(); (returnFocus.current ?? trigger.current)?.focus();
      }}>
        <DialogHeader><DialogTitle>Close tab?</DialogTitle>
          <DialogDescription>Other tabs stay open. An owned browser closes when its last live tab ends.</DialogDescription>
        </DialogHeader>
        <p>{closeTarget?.title || closeTarget?.taskLabel || closeTarget?.url || "Untitled tab"}</p>
        {!closeStillCurrent && closeTarget && <p role="alert">This tab changed. Close this dialog and refresh before trying again.</p>}
        {closeError && <p role="alert">{closeError}</p>}
        {closeBusy && <p role="status">Closing tab…</p>}
        <div className="native-tab-dialog-actions">
          <button type="button" disabled={closeBusy} onClick={() => setCloseTarget(null)}>Cancel</button>
          <button type="button" disabled={closeBusy || !closeStillCurrent || Boolean(closeError)} onClick={() => void closeTab()}>Close tab</button>
        </div>
      </DialogContent>
    </Dialog>
  </>;
}
