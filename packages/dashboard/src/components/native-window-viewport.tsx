"use client";

import { useEffect, useRef, useState } from "react";
import type { NativeBrowserWindow } from "@/lib/native-window-inventory";
import { queueWorkspaceFocusRequest } from "@/lib/workspace-focus-request";

type NativeWorkspaceWindow = Window & { RemoteViewWorkspace?: { isConnected: () => boolean; activateWindow: (generation: number, windowId: number) => Promise<void> } };

/** Remote View owns connection, input gating and the generation-bound native
 * window operation. Observing a window creates no managed task or tab custody. */
export function NativeWindowViewport({ target, loading }: { target: NativeBrowserWindow | null; loading: boolean }) {
  const frame = useRef<HTMLIFrameElement>(null);
  const [view, setView] = useState<{ key: string; state: "opening" | "opened" | "failed" } | null>(null);
  const [attempt, setAttempt] = useState(0);
  const key = target?.key;
  const desktopId = target?.desktopId;
  const windowId = target?.windowId;
  const generation = target?.generation;
  useEffect(() => {
    let cancelled = false;
    if (!key || !desktopId || !windowId || !generation) { setView(null); return; }
    setView({ key, state: "opening" });
    void queueWorkspaceFocusRequest(JSON.stringify(["native-desktop", desktopId]), async () => {
      if (cancelled) return;
      try {
        const deadline = performance.now() + 15000;
        let api: NativeWorkspaceWindow["RemoteViewWorkspace"];
        while (!cancelled && performance.now() < deadline) {
          api = (frame.current?.contentWindow as NativeWorkspaceWindow | null)?.RemoteViewWorkspace;
          if (api?.isConnected()) break;
          await new Promise(resolve => setTimeout(resolve, 100));
        }
        if (cancelled) return;
        if (!api?.isConnected()) throw new Error("native_viewer_unavailable");
        await api.activateWindow(generation, windowId);
        if (!cancelled) setView({ key, state: "opened" });
      } catch { if (!cancelled) setView({ key, state: "failed" }); }
    });
    return () => { cancelled = true; };
  }, [key, desktopId, windowId, generation, attempt]);
  if (!target) return <p role="status" className="native-task-message">{loading ? "Checking native browser window…" : "This browser window could not be verified. Refresh browsers to check again."}</p>;
  const current = view?.key === target.key ? view : null;
  return <section className="native-observed-window" aria-label="Native browser window" data-native-window-state={current?.state ?? "opening"}>
    {current?.state === "failed" && <div role="alert" className="native-task-message"><p>The native window could not be focused. Check the desktop connection and input lock.</p><button type="button" onClick={() => setAttempt(value => value + 1)}>Retry native window</button></div>}
    <iframe ref={frame} key={`${target.desktopId}:${target.generation}`} title="Native Remote View desktop" src={`/${target.slot}?workspace=true`} allow="clipboard-read; clipboard-write; fullscreen" allowFullScreen referrerPolicy="same-origin" />
  </section>;
}
