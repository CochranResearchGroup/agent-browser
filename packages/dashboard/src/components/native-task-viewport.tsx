"use client";

import { useEffect, useRef, useState } from "react";
import {
  requestServiceRemoteViewHandoff,
  ServiceOperationError, ServiceRequestHttpError,
} from "@agent-browser/client/service-request";
import { queueWorkspaceFocusRequest } from "@/lib/workspace-focus-request";

export type NativeTaskViewportTarget = {
  browserId: string;
  sessionName: string;
  targetId: string;
  displayAllocationId: string;
};

type ViewFailure = "handoff" | "operator-access" | "viewer-page" | "viewer-access";
type ViewState = { identity: string; status: "opening" | "opened" | "failed"; url?: string; failure?: ViewFailure };

/** Native Remote View supplies the viewer and its authentication. This component
 * selects the exact owned task and embeds only its ready durable handoff.
 * Census expiry preserves only an already requested exact binding; stale metadata
 * cannot authorize a new selection. Missing or absent bindings clear the viewer.
 * Failure guidance distinguishes preparation, explicit operator policy and the
 * observable iframe boundary. An unreadable page is not proof of an expired
 * login; Remote View alone owns authentication and connection diagnostics. */
export function NativeTaskViewport({ target, unverifiedTarget = null, hasSelection, desktopUnavailable = false, presenceUnavailable = null, loading = false, onRefresh }: {
  target: NativeTaskViewportTarget | null;
  unverifiedTarget?: NativeTaskViewportTarget | null;
  hasSelection?: boolean;
  desktopUnavailable?: boolean;
  presenceUnavailable?: "absent" | "unknown" | null;
  loading?: boolean;
  onRefresh?: () => Promise<void>;
}) {
  const binding = target ?? unverifiedTarget;
  const canRequest = target !== null;
  const browserId = binding?.browserId;
  const sessionName = binding?.sessionName;
  const targetId = binding?.targetId;
  const displayAllocationId = binding?.displayAllocationId;
  const identity = JSON.stringify([browserId, sessionName, targetId, displayAllocationId]);
  const [attempt, setAttempt] = useState(0);
  const [view, setView] = useState<ViewState | null>(null);
  const generation = useRef(0);
  const requested = useRef<{ identity: string; attempt: number } | null>(null);
  useEffect(() => () => {
    generation.current += 1;
    requested.current = null;
  }, []);

  useEffect(() => {
    if (!canRequest || !browserId || !sessionName || !targetId || !displayAllocationId) {
      // The rail becoming unverified does not revoke Native authority or cancel
      // a handoff already being verified. Its exact binding must still match.
      if (unverifiedTarget && requested.current?.identity === identity) return;
      generation.current += 1;
      requested.current = null;
      setView(null);
      return;
    }
    if (requested.current?.identity === identity && requested.current.attempt === attempt) return;
    const current = ++generation.current;
    requested.current = { identity, attempt };
    setView({ identity, status: "opening" });
    void queueWorkspaceFocusRequest(JSON.stringify(["native-desktop", displayAllocationId]), async () => {
      if (generation.current !== current) return;
      try {
        const attribution = {
          browserId, sessionName, serviceName: "agent-browser-dashboard",
          agentName: "operator", taskName: "Select native browser task", jobTimeoutMs: 10000,
        };
        const handoff = await requestServiceRemoteViewHandoff({
          ...attribution, baseUrl: window.location.origin,
          // Preparing an existing-task handoff already performs signed focus.
          // Keep it in the display queue so a later selection wins after this
          // effect completes, with the same native maximize behavior.
          displayAllocationId, params: { targetId, maximize: true }, signal: AbortSignal.timeout(15000),
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
        if (generation.current === current) setView({ identity, status: "failed", failure: accessCheckRequired ? "operator-access" : "handoff" });
      }
    });
  }, [canRequest, browserId, sessionName, targetId, displayAllocationId, identity, attempt, unverifiedTarget]);

  const current = view?.identity === identity ? view : null;
  const hasBinding = Boolean(target || (unverifiedTarget && current));
  const selectionMissing = !hasBinding && hasSelection === true;
  return <section className="native-task-viewport" aria-label="Native desktop" data-native-task-failure={current?.status === "failed" ? current.failure : undefined} data-native-task-state={selectionMissing ? loading ? "refreshing" : desktopUnavailable || presenceUnavailable ? "unavailable" : "missing" : current?.status ?? "unselected"}>
    {!hasBinding ? selectionMissing ? <div role={loading ? "status" : "alert"} className="native-task-message">
      <p>{loading ? "Refreshing selected tab…" : presenceUnavailable === "absent"
        ? "This task is not currently available. Its retained record does not represent a live browser tab."
        : presenceUnavailable === "unknown"
          ? "This task could not be verified as live. Refresh browsers to check again."
          : desktopUnavailable
        ? "This browser has no native desktop to display. Refresh browsers to check availability, or continue through the workflow that owns this task."
        : "The selected tab cannot be shown. Refresh browsers or select another tab."}</p>
      {onRefresh && <button type="button" disabled={loading} onClick={() => void onRefresh()}>{loading ? "Refreshing browsers" : "Refresh browsers"}</button>}
    </div> : <p className="native-task-message">Select a tab to open its native desktop.</p>
      : current?.status === "failed" ? <div role="alert" className="native-task-message">
        <p>{current.failure === "operator-access"
          ? "Check dashboard and profile access before retrying this selection, or choose another tab."
          : current.failure === "viewer-access"
            ? "Check Remote View sign-in or access. The embedded page could not be inspected."
            : current.failure === "viewer-page"
              ? "The Remote View desktop page is unavailable. Its handoff was prepared, but a Native viewer page did not load."
              : "Could not prepare this task's Remote View handoff. Refresh the task, then retry this selection."}</p>
        {current.url && <>
          <p>{current.failure === "viewer-access"
            ? "Open Remote View to check access, then retry this selection."
            : "Check Subsystem health or open Remote View, then retry this selection."}</p>
          <a href={current.url} target="_blank" rel="noopener noreferrer">Open Remote View</a>
        </>}
        {onRefresh && <button type="button" disabled={loading} onClick={() => void onRefresh()}>{loading ? "Refreshing browsers" : "Refresh browsers"}</button>}
        <button type="button" disabled={!canRequest} aria-label="Retry native desktop" onClick={() => setAttempt(value => value + 1)}>Retry</button>
      </div>
        : current?.status === "opened" && current.url ? <iframe
          title="Native Remote View desktop" src={current.url} allow="clipboard-read; clipboard-write; fullscreen"
          allowFullScreen referrerPolicy="same-origin"
          onLoad={event => {
            const frame = event.currentTarget;
            if (!frame.isConnected) return;
            let failure: ViewFailure = "viewer-page";
            try {
              if (frame.contentWindow?.location.href === "about:blank") return;
              // Native workspace embedding is same-origin. Check only its page
              // boundary; Remote View owns connection status and frame delivery.
              const body = frame.contentDocument?.body;
              if (!body) failure = "viewer-access";
              if (body?.id === "viewer" && body.dataset.workspace === "true") return;
            } catch {
              // Cross-origin redirects and blocked frames are unverified access,
              // not evidence that authentication expired or the desktop ended.
              failure = "viewer-access";
            }
            setView(previous => previous?.identity === identity && previous.url === frame.src
              ? { ...previous, status: "failed", failure } : previous);
          }}
        /> : <p role="status" className="native-task-message">Opening native desktop…</p>}
  </section>;
}
