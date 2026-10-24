"use client";

import { useEffect, useState } from "react";
import { CircleHelp, Code2, MousePointer2 } from "lucide-react";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { ProfileDuplicateDialog } from "@/components/profile-duplicate-dialog";
import type { ServiceBrowserModeInspection } from "@agent-browser/client/service-observability";

/** Inspect owner evidence without focusing a browser, opening a viewer or changing mode. */
export function BrowserModeButton({ label, inspection, profileId, observed = false, onRefresh }: {
  label: string; profileId?: string | null; observed?: boolean; inspection?: ServiceBrowserModeInspection | null; onRefresh: () => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [duplicateOpen, setDuplicateOpen] = useState(false);
  const sourceProfileId = profileId ?? inspection?.profileId;
  const [, expireEvidence] = useState(0);
  useEffect(() => {
    const age = Date.now() - (inspection?.observedAtMs ?? Number.NaN);
    if (!Number.isFinite(age) || age < 0 || age >= 30_000) return;
    const timer = window.setTimeout(() => expireEvidence(tick => tick + 1), 30_000 - age);
    return () => window.clearTimeout(timer);
  }, [inspection?.observedAtMs]);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState(false);
  const age = Date.now() - (inspection?.observedAtMs ?? Number.NaN);
  const current = Number.isFinite(age) && age >= 0 && age < 30_000;
  const mode = current ? inspection?.mode : "unverified";
  const name = mode === "cdp" ? "CDP" : mode === "cdp_free" ? "No CDP" : "Unverified";
  const Icon = mode === "cdp" ? Code2 : mode === "cdp_free" ? MousePointer2 : CircleHelp;
  const refresh = async () => {
    setRefreshing(true); setError(false);
    try { await onRefresh(); } catch { setError(true); } finally { setRefreshing(false); }
  };
  return <><DropdownMenu open={open} onOpenChange={setOpen} modal={false}>
    <DropdownMenuTrigger asChild>
      <button type="button" className="native-browser-mode" aria-label={`Browser mode for ${label}: ${name}`}
        title={`Browser mode: ${name}`} onPointerEnter={event => { if (event.pointerType !== "touch") setOpen(true); }}>
        <Icon size={14} aria-hidden="true" />
      </button>
    </DropdownMenuTrigger>
    <DropdownMenuContent className="native-tab-menu" aria-label="Browser mode details" aria-labelledby={undefined} align="start">
      <DropdownMenuLabel>Mode: {name}</DropdownMenuLabel>
      <DropdownMenuLabel>Ownership: {observed ? "Observed" : current && inspection?.ownership === "managed" ? "Managed" : current && inspection?.ownership === "app_provided" ? "App-provided" : "Unverified"}</DropdownMenuLabel>
      <DropdownMenuLabel>Executable: {current ? inspection?.executable?.name ?? "Unverified" : "Unverified"}</DropdownMenuLabel>
      <DropdownMenuLabel>Family: {current ? inspection?.executable?.family ?? "Unverified" : "Unverified"}</DropdownMenuLabel>
      <DropdownMenuLabel>Version: {current ? inspection?.executable?.version ?? "Unverified" : "Unverified"}</DropdownMenuLabel>
      <DropdownMenuLabel>Native input: Unverified</DropdownMenuLabel>
      {name === "Unverified" && <DropdownMenuLabel>Refresh inventory to observe this browser again.</DropdownMenuLabel>}
      <DropdownMenuSeparator />
      {sourceProfileId && <DropdownMenuItem onSelect={() => { setOpen(false); setDuplicateOpen(true); }}>Duplicate profile</DropdownMenuItem>}
      <DropdownMenuItem disabled={refreshing} onSelect={event => { event.preventDefault(); void refresh(); }}>{refreshing ? "Refreshing inventory…" : "Refresh inventory"}</DropdownMenuItem>
      {error && <DropdownMenuLabel role="alert">Inventory refresh failed. Try again.</DropdownMenuLabel>}
    </DropdownMenuContent>
  </DropdownMenu>{sourceProfileId && <ProfileDuplicateDialog open={duplicateOpen} onOpenChange={setDuplicateOpen} profileId={sourceProfileId} onRefresh={onRefresh} />}</>;
}
