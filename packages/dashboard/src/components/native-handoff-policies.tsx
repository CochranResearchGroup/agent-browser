"use client";

import { useRef, useState } from "react";
import type { ServiceManagedHandoff } from "@agent-browser/client/service-observability";
import { postServiceRequest, requireServiceRequestSuccess } from "@agent-browser/client/service-request";

/** Each form changes one owner policy through one explicit public operation.
 * Durations start at owner acceptance; no target selection or creation is requested. */
export function NativeHandoffPolicies({ record, onUpdated }: { record: ServiceManagedHandoff; onUpdated: () => Promise<void> }) {
  const [mode, setMode] = useState(record.expiresAtMs === null ? "perpetual" : "finite");
  const [seconds, setSeconds] = useState("");
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const [failed, setFailed] = useState(false);
  const [saved, setSaved] = useState(false);
  const duration = mode === "perpetual" ? 0 : Number(seconds) * 1000;
  const valid = mode === "perpetual" || (Number.isSafeInteger(Number(seconds)) && Number(seconds) > 0 && Number.isSafeInteger(duration));
  const save = async () => {
    if (!valid || pending.current || failed || record.linkState !== "active") return;
    pending.current = true; setBusy(true); setSaved(false);
    try {
      const reply = requireServiceRequestSuccess(await postServiceRequest({
        baseUrl: window.location.origin, signal: AbortSignal.timeout(15000),
        request: { action: "service_remote_view_handoff_resolve", serviceName: "agent-browser-dashboard", agentName: "operator", taskName: "Change link lifetime", jobTimeoutMs: 10000,
          params: { handoffId: record.handoffId, handoffOperation: "set_link_lifetime", ttlMs: duration } },
      }));
      const data = reply.data as Record<string, unknown> | undefined;
      if (data?.handoffId !== record.handoffId || data.status !== "link_lifetime_updated"
        || (duration === 0 ? data.expiresAtMs !== null : typeof data.expiresAtMs !== "number" || !Number.isSafeInteger(data.expiresAtMs))) {
        throw new Error("unconfirmed_policy_receipt");
      }
      setSaved(true);
      await onUpdated();
    } catch { setFailed(true); }
    finally { pending.current = false; setBusy(false); }
  };
  return <form aria-label="Link lifetime" className="native-handoff-policy" onSubmit={event => { event.preventDefault(); void save(); }}>
    <fieldset disabled={busy || record.linkState !== "active"}>
      <legend>Link lifetime</legend>
      <p>This changes link access only. The tab and unused retention stay unchanged. A finite duration starts when saved.</p>
      <label>Link lifetime mode<select aria-label="Link lifetime mode" value={mode} onChange={event => { setMode(event.target.value); setFailed(false); setSaved(false); }}><option value="perpetual">Perpetual</option><option value="finite">Finite</option></select></label>
      {mode === "finite" && <label>Link lifetime seconds<input aria-label="Link lifetime seconds" type="number" min="1" step="1" required value={seconds} onChange={event => { setSeconds(event.target.value); setFailed(false); setSaved(false); }} /></label>}
      <button type="submit" disabled={!valid || failed}>Save link lifetime</button>
    </fieldset>
    {record.linkState !== "active" && <p>Expired links cannot be renewed here.</p>}
    {busy && <p role="status">Saving link lifetime…</p>}
    {saved && <p role="status">Link lifetime saved. Refresh shows the latest owner record.</p>}
    {failed && <p role="alert">Could not confirm this change. Refresh handoffs before trying again.</p>}
  </form>;
}
