"use client";

import { useRef, useState, type ReactNode } from "react";
import { MoreHorizontal } from "lucide-react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from "@/components/ui/context-menu";
import type { WorkspaceNode, WorkspaceServiceTab } from "@/lib/service-workspaces";

const supplied = (value: unknown): string => typeof value === "string" && value.trim() ? value : "Unknown";

/** Read supplied provenance without treating descriptive names as identity proof.
 * Opening Properties and copying the supplied URL have no browser, selection
 * or handoff effect. Clipboard failure offers manual copying through Properties. */
export function NativeTabActions({ node, tab, children }: { node: WorkspaceNode; tab: WorkspaceServiceTab; children: ReactNode }) {
  const [propertiesOpen, setPropertiesOpen] = useState(false);
  const [copyFeedback, setCopyFeedback] = useState<"copied" | "failed" | null>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const label = tab.title || tab.taskLabel || tab.url || "Untitled tab";
  const metadata = tab.taskMetadata;
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
              if (propertiesOpen) event.preventDefault();
            }}>
              <DropdownMenuItem onSelect={() => setPropertiesOpen(true)}>Properties</DropdownMenuItem>
              <DropdownMenuItem disabled={!tab.url} onSelect={() => void copyUrl()}>Copy URL</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent className="native-tab-menu" onCloseAutoFocus={event => {
        if (propertiesOpen) event.preventDefault();
      }}>
        <ContextMenuItem onSelect={() => setPropertiesOpen(true)}>Properties</ContextMenuItem>
        <ContextMenuItem disabled={!tab.url} onSelect={() => void copyUrl()}>Copy URL</ContextMenuItem>
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
  </>;
}
