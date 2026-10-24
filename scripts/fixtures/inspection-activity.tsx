import React, { useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Provider, useAtomValue, useSetAtom } from 'jotai';
import { ChatPanel } from '@/components/chat-panel';
import { ActivityFeed } from '@/components/activity-feed';
import { useActivitySync } from '@/store/activity';
import { activePortAtom, activeSessionNameAtom, useSessionsSync } from '@/store/sessions';
import { useChatStatusSync } from '@/store/chat';
import { useStreamSync } from '@/store/stream';
import { buildSelectedWorkspaceContext } from '@/lib/selected-workspace-context';

/** Owned synthetic workspace metadata. Session metadata enters through the real
 * sessions HTTP hook; no inspection/review/store collaborator is replaced. */
function Fixture() {
  const [scope, setScope] = useState('A');
  const [streamEnabled, setStreamEnabled] = useState(false);
  const port = useAtomValue(activePortAtom);
  useStreamSync(port, streamEnabled);
  const setPort = useSetAtom(activePortAtom);
  const session = useAtomValue(activeSessionNameAtom);
  useSessionsSync();
  useChatStatusSync();
  useActivitySync();
  const context = useMemo(() => buildSelectedWorkspaceContext({
    selection: { workspaceId: `browser:browser-${scope}`, browserId: `browser-${scope}`, sessionId: `session-${scope}`, tabId: `tab-${scope}`, profileId: `profile-${scope}`, jobId: null },
    serviceBrowsers: [{ id: `browser-${scope}`, profileId: `profile-${scope}`, health: 'ready', pid: scope === 'A' ? 12301 : 12302, processStats: { running: true }, activeSessionIds: [`session-${scope}`] }],
    serviceSessions: [{ id: `session-${scope}`, profileId: `profile-${scope}`, browserIds: [`browser-${scope}`], tabIds: [`tab-${scope}`] }],
    serviceTabs: [{ id: `tab-${scope}`, browserId: `browser-${scope}`, sessionId: `session-${scope}`, ownerSessionId: `fixture-session-${scope}`, targetId: `target-${scope}`, lifecycle: 'ready', url: `https://synthetic.example.test/${scope}`, title: `Owned ${scope}` }],
  }), [scope]);
  return <main>
    <nav aria-label="Fixture selection">{['A', 'B'].map(value => <button type="button" key={value} onClick={() => { setScope(value); setPort(value === 'A' ? 9223 : 9224); }}>Select workspace {value}</button>)}</nav>
    <button type="button" onClick={() => setStreamEnabled(true)}>Connect synthetic stream</button>
    <output aria-label="Active session">{session}</output>
    <section aria-label="Inspection Chat"><ChatPanel selectedWorkspaceContext={context} authenticatedUser={{ username: "synthetic-reviewer", displayName: "Synthetic reviewer", role: "operator" }} /></section>
    <section aria-label="Activity feed"><ActivityFeed selectedWorkspaceContext={context} /></section>
  </main>;
}
createRoot(document.getElementById('root')!).render(<Provider><Fixture /></Provider>);
