"use client";
import { useState } from "react";
import { BrowserBuildPicker, type BrowserBuildEvidence, type BrowserBuildInspection } from "@/components/browser-build-picker";
import { ProfileDuplicateDialog } from "@/components/profile-duplicate-dialog";

export type ProfileLabSource = { id: string; name: string; taskId?: string; sessionName?: string; browserId?: string };

/** Plan independent copies through the existing owner. Discovery never reads profile files or verifies a website login. */
export function ProfileLab({ profiles, onRefresh }: { profiles: ProfileLabSource[]; onRefresh: () => Promise<void> }) {
  const [sourceId, setSourceId] = useState(profiles[0]?.id ?? "");
  const [build, setBuild] = useState<BrowserBuildEvidence | null>(null);
  const [inspection, setInspection] = useState<BrowserBuildInspection | null>(null);
  const [mode, setMode] = useState<"cdp" | "cdp_free">("cdp_free");
  const [open, setOpen] = useState(false);
  const source = profiles.find(profile => profile.id === sourceId);
  return <section aria-label="Profile Lab">
    <h3>Profile Lab</h3>
    <p>Test an independently named copy with a deliberately selected browser build and mode. Real account identity, login usability and storage divergence require separate observation.</p>
    <label>Source profile<select aria-label="Source profile" value={source?.id ?? ""} onChange={event => { setSourceId(event.target.value); setBuild(null); setInspection(null); }}><option value="" disabled>Choose source</option>{profiles.map(profile => <option key={profile.id} value={profile.id}>{profile.name}</option>)}</select></label>
    {source && <>
      <p>Source configured build: {inspection?.nextLaunchBuild?.name ?? "Unverified browser"} {inspection?.nextLaunchBuild?.version ?? "Unverified version"}. Configured mode: {inspection ? inspection.nextLaunchMode === "cdp_free" ? "No CDP" : "CDP" : "Unverified"}. Current process posture must be checked separately.</p>
      <p>Source work</p><ul>{inspection?.activeTasks.map(task => <li key={task.id}>{task.name} ({task.id})</li>)}</ul>
      <BrowserBuildPicker key={source.id} profileId={source.id} onChoose={setBuild} onInspect={setInspection} />
      <label>Target mode<select aria-label="Target mode" value={mode} onChange={event => setMode(event.target.value as "cdp" | "cdp_free")}><option value="cdp_free">No CDP</option><option value="cdp">Automation connected (CDP)</option></select></label>
      <button type="button" disabled={!build || !inspection} onClick={() => setOpen(true)}>Review independent copy</button>
      <ProfileDuplicateDialog key={source.id} open={open} onOpenChange={setOpen} profileId={source.id} taskId={source.taskId} sessionName={source.sessionName} browserId={source.browserId} build={build ?? undefined} mode={mode} sourceBuild={inspection?.nextLaunchBuild} sourceMode={inspection?.nextLaunchMode} onRefresh={onRefresh} />
    </>}
  </section>;
}
