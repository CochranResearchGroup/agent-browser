"use client";

import { useEffect, useRef, useState } from "react";
import {
  postServiceRequest, requestServiceRemoteViewHandoff, requireServiceRequestSuccess,
  ServiceOperationError, ServiceRequestHttpError,
} from "@agent-browser/client/service-request";
import { queueWorkspaceFocusRequest } from "@/lib/workspace-focus-request";

export type NativeTaskViewportTarget = {
  browserId: string;
  sessionName: string;
  targetId: string;
  displayAllocationId: string;
};

type ViewState = { identity: string; status: "opening" | "opened" | "failed"; url?: string; accessCheckRequired?: boolean };

/** Native Remote View supplies the viewer and its authentication. This component
 * selects the exact owned task and embeds only its ready durable handoff.
 * A missing selected target preserves the requested identity and offers inventory refresh.
 * Known access recourse selects bounded guidance without exposing producer diagnostics. */
export function NativeTaskViewport({ target, hasSelection, loading = false, onRefresh }: {
  target: NativeTaskViewportTarget | null;
  hasSelection?: boolean;
  loading?: boolean;
  onRefresh?: () => Promise<void>;
}) {
  const selectionMissing = !target && hasSelection === true;
  const browserId = target?.browserId;
  const sessionName = target?.sessionName;
  const targetId = target?.targetId;
  const displayAllocationId = target?.displayAllocationId;
  const identity = JSON.stringify([browserId, sessionName, targetId, displayAllocationId]);
  const [attempt, setAttempt] = useState(0);
  const [view, setView] = useState<ViewState | null>(null);
  const generation = useRef(0);

  useEffect(() => {
    const current = ++generation.current;
    if (!browserId || !sessionName || !targetId || !displayAllocationId) {
      setView(null);
      return;
    }
    setView({ identity, status: "opening" });
    void queueWorkspaceFocusRequest(JSON.stringify(["native-desktop", displayAllocationId]), async () => {
      if (generation.current !== current) return;
      try {
        const attribution = {
          browserId, sessionName, serviceName: "agent-browser-dashboard",
          agentName: "operator", taskName: "Select native browser task", jobTimeoutMs: 10000,
        };
        const selected = await postServiceRequest({
          baseUrl: window.location.origin,
          signal: AbortSignal.timeout(15000),
          request: { ...attribution, action: "view_focus", params: { targetId, sessionName, operatorFocus: true, maximize: true } },
        });
        if (generation.current !== current) return;
        requireServiceRequestSuccess(selected);
        const handoff = await requestServiceRemoteViewHandoff({
          ...attribution, baseUrl: window.location.origin,
          displayAllocationId, params: { targetId }, signal: AbortSignal.timeout(15000),
        });
        if (generation.current !== current) return;
        const url = new URL(handoff.handoffUrl, window.location.href);
        if (!["http:", "https:"].includes(url.protocol) || url.username || url.password) {
          throw new Error("native_handoff_url_invalid");
        }
        setView({ identity, status: "opened", url: url.toString() });
      } catch (error) {
        const failure = error instanceof ServiceOperationError || error instanceof ServiceRequestHttpError
          ? error.failure : null;
        const accessCheckRequired = failure?.code === "operator_focus_profile_policy_denied"
          || failure?.code === "operator_focus_authority_required";
        if (generation.current === current) setView({ identity, status: "failed", accessCheckRequired });
      }
    });
    return () => { generation.current += 1; };
  }, [browserId, sessionName, targetId, displayAllocationId, identity, attempt]);

  const current = view?.identity === identity ? view : null;
  return <section className="native-task-viewport" aria-label="Native desktop" data-native-task-state={selectionMissing ? loading ? "refreshing" : "missing" : current?.status ?? "unselected"}>
    {!target ? selectionMissing ? <div role={loading ? "status" : "alert"} className="native-task-message">
      <p>{loading ? "Refreshing selected tab…" : "The selected tab cannot be shown. Refresh browsers or select another tab."}</p>
      {onRefresh && <button type="button" disabled={loading} onClick={() => void onRefresh()}>{loading ? "Refreshing browsers" : "Refresh browsers"}</button>}
    </div> : <p className="native-task-message">Select a tab to open its native desktop.</p>
      : current?.status === "failed" ? <div role="alert" className="native-task-message">
        <p>{current.accessCheckRequired
          ? "Check dashboard and profile access before retrying this selection, or choose another tab."
          : "The native desktop could not be opened. Refresh the task, then retry this selection."}</p>
        {current.url && <>
          <p>Open Remote View to sign in or check the desktop, then retry.</p>
          <a href={current.url} target="_blank" rel="noopener noreferrer">Open Remote View</a>
        </>}
        <button type="button" aria-label="Retry native desktop" onClick={() => setAttempt(value => value + 1)}>Retry</button>
      </div>
        : current?.status === "opened" && current.url ? <iframe
          title="Native Remote View desktop" src={current.url} allow="clipboard-read; clipboard-write; fullscreen"
          allowFullScreen referrerPolicy="same-origin"
          onLoad={event => {
            const frame = event.currentTarget;
            if (!frame.isConnected) return;
            try {
              if (frame.contentWindow?.location.href === "about:blank") return;
              // Native workspace embedding is same-origin. Check only its page
              // boundary; Remote View owns connection status and frame delivery.
              const body = frame.contentDocument?.body;
              if (body?.id === "viewer" && body.dataset.workspace === "true") return;
            } catch {
              // Authentication redirects and blocked frames cannot be inspected.
            }
            setView(previous => previous?.identity === identity && previous.url === frame.src
              ? { ...previous, status: "failed" } : previous);
          }}
        /> : <p role="status" className="native-task-message">Opening native desktop…</p>}
  </section>;
}
