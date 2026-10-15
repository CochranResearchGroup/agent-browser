"use client";

import { useRef, useState } from "react";
import type { ServiceManagedHandoff } from "@agent-browser/client/service-observability";
import { postServiceRequest, requireServiceRequestSuccess } from "@agent-browser/client/service-request";

type Props = { record: ServiceManagedHandoff; onUpdated: () => Promise<void> };
type Policy = "link" | "tab" | "unused";
const policies = {
  link: { label: "Link lifetime", operation: "set_link_lifetime", field: "ttlMs", status: "link_lifetime_updated",
    description: "This changes link access only. The tab and unused retention stay unchanged. A finite duration starts when saved." },
  tab: { label: "Tab inactivity", operation: "set_tab_lifetime", field: "tabInactivityMs", status: "tab_lifetime_updated",
    description: "This changes tab inactivity only, measured from the tab’s last activity. Link validity and unused retention stay unchanged. Expiring the final owned tab also closes its browser." },
  unused: { label: "Unused retention", operation: "prepare_human", field: "pendingHumanRetentionMs", status: "pending_human",
    description: "This opens a new wait-for-human retention window. Link validity and tab inactivity stay unchanged. The window starts when saved." },
} as const;

function DurationPolicy({ record, onUpdated, policy }: Props & { policy: Policy }) {
  const spec = policies[policy];
  const hold = record.pendingHuman;
  const initialMs = policy === "unused" ? hold ? hold.expiresAtMs === null ? null : hold.expiresAtMs - hold.requestedAtMs : undefined : record.tabLifetime?.inactivityMs;
  const [mode, setMode] = useState(policy === "link" ? record.expiresAtMs === null ? "perpetual" : "finite"
    : initialMs === null ? "perpetual" : typeof initialMs === "number" ? "finite" : "choose");
  const [seconds, setSeconds] = useState(policy !== "link" && typeof initialMs === "number" && initialMs % 1000 === 0 ? String(initialMs / 1000) : "");
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const [failed, setFailed] = useState(false);
  const [saved, setSaved] = useState(false);
  const duration = mode === "perpetual" ? 0 : Number(seconds) * 1000;
  const valid = mode === "perpetual" || (mode === "finite" && Number.isSafeInteger(Number(seconds)) && Number(seconds) > 0 && Number.isSafeInteger(duration));
  const enabled = record.linkState === "active" && (policy === "link" || record.targetState === "reported");
  const save = async () => {
    if (!valid || pending.current || failed || !enabled) return;
    pending.current = true; setBusy(true); setSaved(false);
    try {
      const reply = requireServiceRequestSuccess(await postServiceRequest({
        baseUrl: window.location.origin, signal: AbortSignal.timeout(15000),
        request: { action: "service_remote_view_handoff_resolve", serviceName: "agent-browser-dashboard", agentName: "operator", taskName: `Change ${spec.label.toLowerCase()}`, jobTimeoutMs: 10000,
          params: { handoffId: record.handoffId, handoffOperation: spec.operation, [spec.field]: duration } },
      }));
      const data = reply.data as Record<string, unknown> | undefined;
      const lifetime = data?.tabLifetime as Record<string, unknown> | null | undefined;
      const pendingHuman = data?.pendingHuman as Record<string, unknown> | null | undefined;
      let receiptMatches: boolean;
      if (policy === "link") {
        receiptMatches = duration === 0 ? data?.expiresAtMs === null : typeof data?.expiresAtMs === "number" && Number.isSafeInteger(data.expiresAtMs) && data.expiresAtMs >= 0;
      } else if (policy === "tab") {
        receiptMatches = lifetime?.inactivityMs === (duration === 0 ? null : duration) && data?.tabId === record.tabId;
      } else {
        const requested = pendingHuman?.requestedAtMs;
        const expires = pendingHuman?.expiresAtMs;
        receiptMatches = data?.tabId === record.tabId && pendingHuman?.handoffId === record.handoffId
          && typeof requested === "number" && Number.isSafeInteger(requested) && requested >= 0
          && (duration === 0 ? expires === null : typeof expires === "number" && Number.isSafeInteger(expires) && expires - requested === duration);
      }
      if (data?.handoffId !== record.handoffId || data.status !== spec.status || !receiptMatches) throw new Error("unconfirmed_policy_receipt");
      setSaved(true);
      await onUpdated();
    } catch { setFailed(true); }
    finally { pending.current = false; setBusy(false); }
  };
  return <form aria-label={spec.label} className="native-handoff-policy" onSubmit={event => { event.preventDefault(); void save(); }}>
    <fieldset disabled={busy || !enabled}>
      <legend>{spec.label}</legend>
      <p>{spec.description}</p>
      <label>{spec.label} mode<select aria-label={`${spec.label} mode`} value={mode} onChange={event => { setMode(event.target.value); setFailed(false); setSaved(false); }}><option value="choose">Choose a policy</option><option value="perpetual">{policy === "unused" ? "Until used" : "Perpetual"}</option><option value="finite">Finite</option></select></label>
      {mode === "finite" && <label>{spec.label} seconds<input aria-label={`${spec.label} seconds`} type="number" min="1" step="1" required value={seconds} onChange={event => { setSeconds(event.target.value); setFailed(false); setSaved(false); }} /></label>}
      <button type="submit" disabled={!valid || failed}>Save {spec.label.toLowerCase()}</button>
    </fieldset>
    {!enabled && <p>{policy === "link" ? "Expired links cannot be renewed here." : `${spec.label} requires an active link and a reported target. Refresh to check its current state.`}</p>}
    {busy && <p role="status">Saving {spec.label.toLowerCase()}…</p>}
    {saved && <p role="status">{spec.label} saved. Refresh shows the latest owner record.</p>}
    {failed && <p role="alert">Could not confirm this change. Refresh handoffs before trying again.</p>}
  </form>;
}

/** Separate forms issue separate owner mutations; changing tab inactivity never
 * sends link expiry, pressure or pending-human policy fields. */
export function NativeHandoffPolicies(props: Props) {
  return <><DurationPolicy {...props} policy="link" /><DurationPolicy {...props} policy="tab" /><DurationPolicy {...props} policy="unused" /></>;
}
