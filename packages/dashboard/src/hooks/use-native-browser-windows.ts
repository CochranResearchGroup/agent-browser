"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { fetchCoordinatedDashboardRead, startCompletionDrivenDashboardPoll } from "@/lib/dashboard-read-coordinator";
import { DASHBOARD_WORKSPACE_SELECTION_EVENT } from "@/lib/workspace-url-selection";
import { nativeBrowserWindows, type NativeBrowserWindow } from "@/lib/native-window-inventory";

/** Native discovery proceeds independently of managed-task and diagnostic reads.
 * Preserve authentication and access failures as operator guidance; dashboard
 * authentication does not establish a Remote View session. */
export function useNativeBrowserWindows() {
  const [windows, setWindows] = useState<NativeBrowserWindow[]>([]);
  const [loading, setLoading] = useState(true);
  const [issue, setIssue] = useState<string | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const flight = useRef<Promise<void> | null>(null);
  const expiry = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mounted = useRef(true);
  const observation = useRef({ token: -1, firstSeen: 0 });
  const refresh = useCallback((revalidate = false): Promise<void> => {
    if (flight.current) return flight.current;
    const request = (async () => {
      try {
        const response = await fetchCoordinatedDashboardRead("/api/v1/desktops/browser-windows?include_hidden=true", { freshForMs: 1000, revalidate });
        if (response.status === 401) throw new Error("native_inventory_sign_in");
        if (response.status === 403) throw new Error("native_inventory_forbidden");
        if (!response.ok) throw new Error("native_inventory_unavailable");
        if (response.headers.get("content-type")?.toLowerCase().includes("text/html")) {
          throw new Error("native_inventory_access_page");
        }
        const result = nativeBrowserWindows(await response.json());
        if (!mounted.current) return;
        const firstSeen = performance.now() - result.ageMs;
        observation.current = { token: result.observedAtMs, firstSeen: observation.current.token === result.observedAtMs ? Math.min(observation.current.firstSeen, firstSeen) : firstSeen };
        const ageMs = performance.now() - observation.current.firstSeen;
        if (ageMs >= 30_000) throw new Error("native_inventory_stale");
        setWindows(result.windows);
        setIssue(result.unverified ? "Some Remote View desktops could not be checked." : null);
        if (expiry.current) clearTimeout(expiry.current);
        expiry.current = setTimeout(() => { setWindows([]); setIssue("Remote View browser inventory is stale. Refresh browsers."); }, 30_000 - ageMs);
      } catch (error) {
        if (!mounted.current) return;
        // An old observation's expiry must not overwrite a newer access failure.
        if (expiry.current) { clearTimeout(expiry.current); expiry.current = null; }
        setWindows([]);
        const code = error instanceof Error ? error.message : "";
        setIssue(code === "native_inventory_sign_in"
          ? "Sign in to Remote View to load browser windows. Your dashboard sign-in does not sign you in to Remote View. After signing in, refresh browsers."
          : code === "native_inventory_forbidden"
            ? "Remote View denied access to browser windows. Check your account permissions before refreshing browsers."
            : code === "native_inventory_access_page"
              ? "Remote View returned a web page instead of browser windows. Check Remote View sign-in and access, then refresh browsers."
              : code === "native_inventory_stale"
                ? "Remote View browser inventory is stale. Refresh browsers."
                : code === "native_inventory_unavailable"
                  ? "Remote View browser inventory is unavailable. Managed tasks remain listed separately."
                  : "Could not load Remote View browser windows. Check Remote View sign-in and access, or the service connection, then refresh browsers.");
      } finally { if (mounted.current) setLoading(false); flight.current = null; }
    })();
    flight.current = request;
    return request;
  }, []);
  useEffect(() => {
    mounted.current = true;
    const select = () => setSelectedKey(new URL(window.location.href).searchParams.get("native-window"));
    select();
    window.addEventListener("popstate", select);
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, select);
    const stop = startCompletionDrivenDashboardPoll(() => refresh(), 3000);
    return () => { mounted.current = false; stop(); if (expiry.current) clearTimeout(expiry.current); window.removeEventListener("popstate", select); window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, select); };
  }, [refresh]);
  const select = useCallback((target: NativeBrowserWindow) => {
    const url = new URL(window.location.href);
    url.searchParams.set("native-window", target.key);
    history.replaceState(history.state, "", `${url.pathname}${url.search}${url.hash}`);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, []);
  return { windows, loading, issue, selectedKey, select, refresh };
}
