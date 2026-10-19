"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";
import { MoreHorizontal } from "lucide-react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "@/components/ui/context-menu";
import { tabDescription } from "@/lib/tab-description";
import type { WorkspaceNode, WorkspaceServiceTab } from "@/lib/service-workspaces";
import { postServiceRequest, requireServiceRequestSuccess, ServiceOperationError, ServiceRequestHttpError, type ServiceTabCloseData } from "@agent-browser/client/service-request";

// Shared Dialog animation utilities must not animate viewport resizing or
// override the native Properties panel's border-box dimensions.
const propertiesStyle = { boxSizing: "border-box", width: "min(480px, calc(100vw - 32px))", animation: "none", transition: "none", transform: "none" } as const;
const supplied = (value: unknown): string => typeof value === "string" && value.trim() ? value : "Unknown";
const createdTime = (value: unknown): string => {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) return "Unknown";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Unknown" : date.toLocaleString();
};

/** Read supplied provenance without treating descriptive names as identity proof.
 * Opening Properties and copying the supplied URL have no browser, selection
 * or handoff effect. Clipboard failure offers manual copying through Properties.
 * Close captures an exact identity and requires an owner-confirmed close receipt
 * before releasing the row or its selected native frame. */
export function NativeTabActions({ node, tab, children, onClosed, onProfileChanged, onShare, shareBusy }: { node: WorkspaceNode; tab: WorkspaceServiceTab; children: ReactNode; onClosed?: () => Promise<void>; onProfileChanged?: () => Promise<void>; onShare?: (opener: HTMLElement | null) => void; shareBusy?: boolean }) {
  const [propertiesOpen, setPropertiesOpen] = useState(false);
  const [closeTarget, setCloseTarget] = useState<WorkspaceServiceTab | null>(null);
  const [closeBusy, setCloseBusy] = useState(false);
  const [closeError, setCloseError] = useState<string | null>(null);
  const closePending = useRef(false);
  const [copyFeedback, setCopyFeedback] = useState<"copied" | "failed" | null>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const label = tabDescription(tab);
  const metadata = tab.taskMetadata;
  const [profile, setProfile] = useState(tab.profileProperties ?? null);
  const [profileName, setProfileName] = useState(tab.profileProperties?.name ?? "");
  const [profileBusy, setProfileBusy] = useState(false);
  const [profileUnconfirmed, setProfileUnconfirmed] = useState(false);
  const [profileMessage, setProfileMessage] = useState<string | null>(null);
  const profilePending = useRef(false);
  const lastProfileSource = useRef(JSON.stringify(tab.profileProperties ?? null));
  useEffect(() => {
    if (!propertiesOpen && !profilePending.current) {
      const source = JSON.stringify(tab.profileProperties ?? null);
      if (source !== lastProfileSource.current) {
        setProfile(tab.profileProperties ?? null);
        setProfileName(tab.profileProperties?.name ?? "");
        lastProfileSource.current = source;
      } else {
        setProfileName(profile?.name ?? "");
      }
      setProfileMessage(null); setProfileUnconfirmed(false);
    }
  }, [propertiesOpen, tab.profileProperties]);
  const saveProfile = async (promote: boolean) => {
    if (!profile || profilePending.current || profileUnconfirmed || !profileName.trim()) return;
    profilePending.current = true; setProfileBusy(true); setProfileMessage(null);
    const expected = profile;
    try {
      const response = await fetch("/api/dashboard/profile-properties", {
        method: "PATCH", credentials: "same-origin", redirect: "error", signal: AbortSignal.timeout(15000),
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ profileId: expected.id, expectedName: expected.name, expectedKind: expected.kind, name: profileName.trim(), promote }),
      });
      const body = await response.json();
      const saved = body?.data?.profile;
      if (!response.ok || body?.success !== true || saved?.id !== expected.id || saved?.name !== profileName.trim()
        || saved?.kind !== (promote ? "named" : expected.kind)) throw new Error("profile_save_unconfirmed");
      setProfile(saved); setProfileName(saved.name);
      void onProfileChanged?.().catch(() => undefined);
      setProfileMessage(promote ? "Profile is now durable. Browser data and open tabs are preserved." : "Profile name saved.");
    } catch {
      setProfileUnconfirmed(true);
      setProfileMessage("Profile change could not be confirmed. Refresh browsers to read its current name and retention before retrying.");
    } finally { profilePending.current = false; setProfileBusy(false); }
  };
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
    } catch (error) {
      const failure = error instanceof ServiceOperationError || error instanceof ServiceRequestHttpError ? error.failure : null;
      setCloseError(failure?.code === "operator_tab_close_profile_policy_denied" && failure.effectState === "no_effect"
        ? "This profile does not allow you to close this tab. No tab was closed. Ask the profile owner to review close permission."
        : "Close could not be confirmed. Refresh browsers and check close permission before trying again.");
    } finally { closePending.current = false; setCloseBusy(false); }
  };
  const copyUrl = async () => {
    if (!tab.url) return;
    try {
      await navigator.clipboard.writeText(tab.url);
      setCopyFeedback("copied");
    } catch { setCopyFeedback("failed"); }
  };
  const provenanceFields: [string, unknown][] = [
    ["Service", metadata?.serviceName], ["Agent", metadata?.agentName],
    ["Human", metadata?.humanName], ["Origin", metadata?.originKind],
    ["Task", metadata?.taskName], ["Purpose", metadata?.purpose],
    ["Job", metadata?.jobId], ["Request", metadata?.requestId], ["Attribution", metadata?.attribution],
  ];
  const recordedProvenance = provenanceFields.filter(([, value]) => typeof value === "string" && value.trim() && value !== "unknown");
  const fields: [string, unknown][] = [
    ...recordedProvenance,
    ["Created", createdTime(metadata?.createdAtMs ?? tab.createdAtMs)], ["Browser", tab.browserId],
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
              <button ref={trigger} type="button" className="native-tab-actions" title="Tab actions" aria-label={`Tab actions for ${label}`}>
                <MoreHorizontal size={14} aria-hidden="true" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent className="native-tab-menu" align="end" onCloseAutoFocus={event => {
              if (propertiesOpen || closeTarget) event.preventDefault();
            }}>
              <DropdownMenuItem disabled={!onShare || shareBusy || !tab.desktopId} onSelect={() => onShare?.(returnFocus.current ?? trigger.current)}>Share</DropdownMenuItem>
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
        <ContextMenuItem disabled={!onShare || shareBusy || !tab.desktopId} onSelect={() => onShare?.(returnFocus.current ?? trigger.current)}>Share</ContextMenuItem>
        <ContextMenuItem onSelect={() => setPropertiesOpen(true)}>Properties</ContextMenuItem>
        <ContextMenuItem disabled={!tab.url} onSelect={() => void copyUrl()}>Copy URL</ContextMenuItem>
        <ContextMenuItem disabled={!canClose || closeBusy} onSelect={openClose}>Close Tab</ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
    {copyFeedback && <p className="native-tab-feedback" role={copyFeedback === "failed" ? "alert" : "status"}>
      {copyFeedback === "copied" ? "URL copied" : "Could not copy this URL. Open Properties to copy it manually."}
    </p>}
    <Dialog open={propertiesOpen} onOpenChange={open => { if (!profilePending.current) setPropertiesOpen(open); }}>
      <DialogContent className="native-tab-properties" style={propertiesStyle} showCloseButton={!profileBusy} onCloseAutoFocus={event => {
        event.preventDefault(); (returnFocus.current ?? trigger.current)?.focus();
      }}>
        <DialogHeader><DialogTitle>Tab properties</DialogTitle>
          <DialogDescription>Supplied task context and current inventory. Names are descriptive, not verified identity.</DialogDescription>
        </DialogHeader>
        {recordedProvenance.length === 0 && <p role="status">Original task attribution was not recorded for this tab. Known inventory facts are shown below.</p>}
        <section className="native-profile-properties" aria-label="Profile properties">
          <p><strong>Profile retention:</strong> {profile ? profile.kind === "disposable" ? "Throwaway" : "Durable" : "Not reported"}</p>
          {profile && <>
            <p>{profile.kind === "disposable" ? "Eligible for automatic cleanup after its work ends. Make durable to retain browser data across tasks." : "Browser data is retained across tasks."}</p>
            <label htmlFor={`profile-name-${tab.id}`}>Profile name</label>
            <input id={`profile-name-${tab.id}`} value={profileName} maxLength={120} disabled={profileBusy}
              onChange={event => { setProfileName(event.target.value); if (!profileUnconfirmed) setProfileMessage(null); }} />
            <div className="native-tab-dialog-actions">
              <button type="button" disabled={profileBusy || profileUnconfirmed || !profileName.trim() || profileName.trim() === profile.name} onClick={() => void saveProfile(false)}>Save name</button>
              {profile.kind === "disposable" && <button type="button" disabled={profileBusy || profileUnconfirmed || !profileName.trim()} onClick={() => void saveProfile(true)}>Make durable</button>}
            </div>
            <p>Renaming changes the display name. The profile ID and browser data stay the same.</p>
          </>}
          {profileBusy && <p role="status">Saving profile…</p>}
          {profileMessage && <p role="status">{profileMessage}</p>}
        </section>
        <dl>{fields.map(([name, value]) => <div key={name}><dt>{name}</dt><dd>{supplied(value)}</dd></div>)}</dl>
      </DialogContent>
    </Dialog>
    <Dialog open={closeTarget !== null} onOpenChange={open => { if (!open && !closePending.current) setCloseTarget(null); }}>
      <DialogContent className="native-tab-properties" style={propertiesStyle} showCloseButton={!closeBusy} onCloseAutoFocus={event => {
        event.preventDefault(); (returnFocus.current ?? trigger.current)?.focus();
      }}>
        <DialogHeader><DialogTitle>Close tab?</DialogTitle>
          <DialogDescription>Other tabs stay open. An owned browser closes when its last live tab ends.</DialogDescription>
        </DialogHeader>
        <p>{closeTarget ? tabDescription(closeTarget) : "Untitled tab"}</p>
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
