"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Activity, CircleCheck, CircleHelp, CircleX, RefreshCw, TriangleAlert } from "lucide-react";
import { Dialog, DialogContent, DialogDescription, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { currentHealth, nativeHealthRow, overallHealth, runtimeHealthRow, type HealthState, type SubsystemHealth } from "@/lib/subsystem-health";

const labels: Record<HealthState, string> = { healthy: "Healthy", attention: "Needs attention", failed: "Unavailable", unknown: "Not verified" };
const icons = { healthy: CircleCheck, attention: TriangleAlert, failed: CircleX, unknown: CircleHelp };
const unknownRows: SubsystemHealth[] = [
  { id: "runtime", label: "Browser runtime", state: "unknown", detail: "Open this drawer to check runtime diagnostics." },
  { id: "native", label: "Remote View desktops", state: "unknown", detail: "Open this drawer to check Native desktop readiness." },
];

/** No periodic diagnostic requests: live inventory errors and selected-view state supplement explicit checks. */
export function SubsystemHealthIndicator({ inventoryIssue, nativeIssue, loading }: {
  inventoryIssue?: string | null; nativeIssue?: string | null; loading?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [checking, setChecking] = useState(false);
  const [rows, setRows] = useState(unknownRows);
  const [now, setNow] = useState(Date.now);
  const [viewerState, setViewerState] = useState<string | null>(null);
  const flight = useRef<AbortController | null>(null);
  const cancelled = useRef(false);
  const [connectionState, setConnectionState] = useState<string | null>(null);
  useEffect(() => {
    const read = () => {
      const viewport = document.querySelector(".native-task-viewport, .native-observed-window");
      setViewerState(viewport?.getAttribute("data-native-task-state") ?? viewport?.getAttribute("data-native-window-state") ?? null);
      try { setConnectionState(viewport?.querySelector("iframe")?.contentDocument?.getElementById("connection-status")?.dataset.state ?? null); }
      catch { setConnectionState(null); }
    };
    const observer = new MutationObserver(read);
    observer.observe(document.querySelector(".native-workspace-main") ?? document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-native-task-state", "data-native-window-state"] });
    read();
    const timer = setInterval(read, 1000);
    return () => { observer.disconnect(); clearInterval(timer); };
  }, []);
  // Expire evidence without launching another network request.
  useEffect(() => { const timer = setInterval(() => setNow(Date.now()), 1000); return () => clearInterval(timer); }, []);
  useEffect(() => () => flight.current?.abort(), []);
  const refresh = useCallback(async () => {
    if (flight.current) return;
    const controller = new AbortController(); flight.current = controller; setChecking(true);
    setRows(unknownRows.map(row => ({ ...row, detail: "Checking…" })));
    const timeout = setTimeout(() => controller.abort(), 8000);
    const check = async (url: string, parse: (value: unknown, at: number) => SubsystemHealth, fallback: SubsystemHealth) => {
      try {
        const response = await fetch(url, { credentials: "same-origin", cache: "no-store", signal: controller.signal, redirect: "error" });
        if (!response.ok) return { ...fallback, checkedAt: Date.now(), state: response.status === 401 || response.status === 403 ? "attention" as const : "failed" as const,
          detail: `Health endpoint returned HTTP ${response.status}. ${response.status === 401 || response.status === 403 ? "Check access in Service diagnostics." : "The subsystem health endpoint is unavailable. Review Service diagnostics."}` };
        return parse(await response.json(), Date.now());
      } catch {
        return { ...fallback, state: "unknown" as const, checkedAt: Date.now(), detail: "Health check could not complete. The service may be unreachable, signed out, or slow. Retry or review Service diagnostics." };
      }
    };
    try {
      const results = await Promise.all([
        check("/api/runtime/health", runtimeHealthRow, unknownRows[0]),
        check("/api/v1/slots", nativeHealthRow, unknownRows[1]),
      ]);
      if (flight.current === controller && !cancelled.current) { setRows(results); setNow(Date.now()); }
    } finally {
      clearTimeout(timeout);
      if (flight.current === controller) { flight.current = null; if (!cancelled.current) setChecking(false); }
    }
  }, []);
  useEffect(() => { cancelled.current = false; return () => { cancelled.current = true; }; }, []);
  const evidence: SubsystemHealth[] = [
    { id: "inventory", label: "Browser inventory", state: inventoryIssue ? "failed" : loading ? "unknown" : "healthy", detail: inventoryIssue ? "Browser inventory could not be refreshed. Refresh browsers to retry." : loading ? "Waiting for the current browser inventory." : "The workspace inventory is available; browser and viewer readiness are checked separately." },
    ...(nativeIssue ? [{ id: "native-inventory", label: "Native window census", state: "attention" as const, detail: nativeIssue }] : []),
    ...rows.map(row => currentHealth(row, now)),
    { id: "viewer", label: "Selected Native viewer", state: viewerState === "failed" || connectionState === "error" ? "failed" : connectionState === "connected" ? "healthy" : "unknown", detail: viewerState === "failed" ? "The selected Native desktop failed to open. A successful handoff is not proof that the viewer loaded." : connectionState === "connected" ? "The selected Native viewer reports a connected display transport." : connectionState === "error" ? "The selected Native viewer reports a connection failure. Inspect its message for recovery." : viewerState === "opened" ? "The Native page has opened. Its connection status is shown inside the viewer; this indicator does not infer display readiness from an iframe." : "No current Native display connection has been verified by this indicator." },
  ];
  const state = overallHealth(evidence.filter(row => row.id !== "viewer" || viewerState !== null));
  const Icon = icons[state];
  return <Dialog open={open} onOpenChange={value => { setOpen(value); if (value) void refresh(); }}>
    <DialogTrigger asChild><button type="button" className={`subsystem-health-trigger health-${state}`} aria-label={`Subsystem health: ${labels[state]}`} title={`Subsystem health: ${labels[state]}. Open details.`}><Activity size={15} aria-hidden="true" /><Icon size={9} aria-hidden="true" /></button></DialogTrigger>
    <DialogContent className="subsystem-health-drawer" style={{ left: "auto", top: 12, right: 12, bottom: 12, translate: "none", transform: "none", animation: "none" }}>
      <DialogTitle>Subsystem health</DialogTitle>
      <DialogDescription>Current inventory signals and read-only checks. Stale or missing evidence is never shown as healthy.</DialogDescription>
      <div className="flex items-center justify-between gap-3"><span role="status">{checking ? "Checking subsystems…" : labels[state]}</span><button type="button" className="flex items-center gap-2 rounded border px-3 py-2" disabled={checking} onClick={() => void refresh()} title="Refresh subsystem health"><RefreshCw size={13} aria-hidden="true" />Refresh health</button></div>
      <div className="min-h-0 overflow-auto">{evidence.map(row => { const RowIcon = icons[row.state]; return <section key={row.id} className="subsystem-health-row"><div className={`flex items-center gap-2 health-${row.state}`}><RowIcon size={15} aria-hidden="true" /><h3 className="font-semibold">{row.label}</h3><span className="ml-auto text-xs">{labels[row.state]}</span></div><p>{row.detail}</p>{row.checkedAt && <p className="text-xs">Checked {new Date(row.checkedAt).toLocaleTimeString()}</p>}</section>; })}</div>
      <a className="underline" href="/service">Open Service diagnostics</a>
    </DialogContent>
  </Dialog>;
}
