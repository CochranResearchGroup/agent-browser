"use client";
import { useCallback, useEffect, useRef, useState } from "react";

export type BrowserBuildChoice = { buildId: string; expectedSha256: string; executablePath?: string };
export type BrowserBuildEvidence = {
  id: string; name?: string; family?: string; version?: string; available: boolean; nativeBinary?: boolean;
  selection: BrowserBuildChoice; compatibility?: { versionRelation?: string; familyRelation?: string };
};
export type BrowserBuildInspection = {
  schemaVersion: string; profileId: string; profileName?: string; profileDataVersion?: string;
  nextLaunchMode: "cdp" | "cdp_free"; nextLaunchBuild?: BrowserBuildEvidence;
  availableBuilds: BrowserBuildEvidence[]; currentSelectionOperationId?: string;
  activeTasks: { id: string; name: string; browserId?: string }[]; activeOperationId?: string;
};

/** Discovery reads evidence only. A choice never asserts storage or credential portability. */
export function BrowserBuildPicker({ profileId, onChoose, onInspect }: {
  profileId: string; onChoose: (build: BrowserBuildEvidence | null) => void;
  onInspect?: (inspection: BrowserBuildInspection | null) => void;
}) {
  const [inspection, setInspection] = useState<BrowserBuildInspection | null>(null);
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState("");
  const [reviewedPath, setReviewedPath] = useState<string | undefined>();
  const version = useRef(0);
  const callbacks = useRef({ onChoose, onInspect }); callbacks.current = { onChoose, onInspect };
  const inspect = useCallback(async (executablePath?: string) => {
    const current = ++version.current;
    setBusy(true); setError(null); setInspection(null); setSelected("");
    callbacks.current.onChoose(null); callbacks.current.onInspect?.(null);
    try {
      const response = await fetch("/api/command", { method: "POST", headers: { "Content-Type": "application/json" }, signal: AbortSignal.timeout(15000), body: JSON.stringify({ action: "browser_session_build_select", choice: "inspect", profileId, ...(executablePath ? { executablePath } : {}) }) });
      const reply = await response.json();
      if (!response.ok || reply.success !== true || reply.data?.schemaVersion !== "agent-browser.build-selection.v1" || reply.data.profileId !== profileId || !Array.isArray(reply.data.availableBuilds) || !Array.isArray(reply.data.activeTasks)) throw new Error(typeof reply.error === "string" ? reply.error : "Build evidence could not be read.");
      if (current !== version.current) return;
      setInspection(reply.data); setReviewedPath(executablePath); callbacks.current.onInspect?.(reply.data);
    } catch (error) { if (current === version.current) setError(error instanceof Error ? error.message : "Build evidence could not be read."); }
    finally { if (current === version.current) setBusy(false); }
  }, [profileId]);
  useEffect(() => { void inspect(); return () => { version.current += 1; }; }, [inspect]);
  return <div>
    <p>Profile data version: {inspection?.profileDataVersion ?? "Unverified"}. Credential storage, data readability and login portability: Unverified.</p>
    <p>Changing builds may migrate profile data or lose login. Selecting a previous executable does not restore profile bytes. Duplicate and test first, choose another build, or explicitly confirm the original profile. Family warnings are advisory.</p>
    {busy && <p role="status">Reading browser builds…</p>}
    {error && <p role="alert">{error} Refresh discovery before choosing.</p>}
    <div role="region" aria-label="Available browser builds" style={{ maxHeight: "min(16rem, 35vh)", overflowY: "auto" }}>
      {inspection?.availableBuilds.map(build => <label key={build.id} style={{ display: "block" }}>
        <input type="radio" name={`browser-build-${profileId}`} aria-label={`${build.name ?? "Browser"} ${build.version ?? "Unverified version"}`} disabled={busy || !build.available || build.nativeBinary === false || !build.selection?.expectedSha256} checked={selected === build.id} onChange={() => { setSelected(build.id); callbacks.current.onChoose({ ...build, selection: { ...build.selection, ...(reviewedPath ? { executablePath: reviewedPath } : {}) } }); }} />
        {build.name ?? "Browser"} · {build.family ?? "Unverified family"} · {build.version ?? "Unverified version"}
        <small style={{ display: "block" }}>Version relation: {build.compatibility?.versionRelation ?? "unverified"}; family: {build.compatibility?.familyRelation ?? "unverified"}. {build.nativeBinary === false ? "Select the actual browser executable instead of this launcher." : !build.available ? "Unavailable. Choose another build." : ""}</small>
      </label>)}
    </div>
    <label>Other executable<input aria-label="Other browser executable" value={path} onChange={event => setPath(event.target.value)} placeholder="Absolute executable path" /></label>
    <button type="button" disabled={busy || !path.startsWith("/")} onClick={() => void inspect(path)}>Inspect executable</button>
    <button type="button" disabled={busy} onClick={() => void inspect()}>Refresh browser builds</button>
  </div>;
}
