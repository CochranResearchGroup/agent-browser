"use client";

import { atom } from "jotai";
import { useEffect } from "react";
import { useAtomValue, useSetAtom } from "jotai/react";
import type { ActivityEvent } from "@/types";
import { streamEventsAtom } from "@/store/stream";
import { activeSessionNameAtom } from "@/store/sessions";
import { activitySessionName, isInspectionReviewEvent, latestInspectionReview, sameInspectionSelection,
  type InspectionReviewEvent, type InspectionReviewSource, type InspectionSelection, type InspectionVerdict } from '@/lib/inspection-review';

const PERSIST_KEY = "ab-persist-activity";
const MAX_PERSISTED = 500;
function activityStorageKey(session: string) { return `ab-activity-${session}`; }
function loadPersistedEvents(session: string): ActivityEvent[] {
  try { const parsed = JSON.parse(localStorage.getItem(activityStorageKey(session)) ?? '[]'); return Array.isArray(parsed) ? parsed.slice(-MAX_PERSISTED) : []; }
  catch { return []; }
}
function savePersistedEvents(session: string, events: ActivityEvent[]): boolean {
  try { localStorage.setItem(activityStorageKey(session), JSON.stringify(events.slice(-MAX_PERSISTED))); return true; }
  catch { return false; }
}
function clearPersistedEvents(session: string) { try { localStorage.removeItem(activityStorageKey(session)); } catch { /* Browser storage may be unavailable. */ } }

export const persistActivityAtom = atom(typeof window !== "undefined" ? localStorage.getItem(PERSIST_KEY) === "true" : false);
export const restoredEventsAtom = atom<ActivityEvent[]>([]);
const restoredSessionAtom = atom<string | null>(null);
// Local operator reviews belong to Activity, not a daemon's replaceable stream
// snapshot. They share the same cap, persistence key and Clear lifecycle.
const localReviewsAtom = atom<InspectionReviewEvent[]>([]);
const currentSessionAtom = atom(get => activitySessionName(get(activeSessionNameAtom)));
export const combinedEventsAtom = atom(get => {
  const session = get(currentSessionAtom);
  const restored = get(persistActivityAtom) && get(restoredSessionAtom) === session ? get(restoredEventsAtom) : [];
  const events = [...restored, ...get(streamEventsAtom), ...get(localReviewsAtom)].filter(event =>
    event.type !== 'result' || event.action !== 'inspection_review' || (isInspectionReviewEvent(event) && event.data.source.activitySession === session));
  const seen = new Set<string>();
  return events.filter(event => {
    if (!isInspectionReviewEvent(event)) return true;
    if (seen.has(event.id)) return false;
    seen.add(event.id); return true;
  }).sort((a, b) => a.timestamp - b.timestamp).slice(-MAX_PERSISTED);
});

export const recordInspectionReviewAtom = atom(null, (get, set, input: {
  source: InspectionReviewSource; currentSelection: InspectionSelection | null; verdict: InspectionVerdict; actor: string;
}) => {
  const session = get(currentSessionAtom);
  if (input.source.activitySession !== session || !sameInspectionSelection(input.source.selection, input.currentSelection)
    || !input.actor.trim() || !['accepted', 'rejected'].includes(input.verdict)) return { recorded: false, persistence: 'off' as const };
  const previous = latestInspectionReview(get(combinedEventsAtom), input.source, input.actor);
  if (previous?.data.verdict !== input.verdict) {
    const event: InspectionReviewEvent = { type: 'result', action: 'inspection_review', id: `inspection-review:${crypto.randomUUID()}`,
      success: true, duration_ms: 0, timestamp: Date.now(),
      data: { version: 'inspection-review.v1', source: structuredClone(input.source), verdict: input.verdict, actor: input.actor } };
    if (!isInspectionReviewEvent(event)) return { recorded: false, persistence: 'off' as const };
    set(localReviewsAtom, [...get(localReviewsAtom), event].slice(-MAX_PERSISTED));
  }
  const persistence = !get(persistActivityAtom) ? 'off' : savePersistedEvents(session, get(combinedEventsAtom)) ? 'saved' : 'unavailable';
  return { recorded: true, persistence };
});
export const togglePersistAtom = atom(null, (get, set) => {
  const next = !get(persistActivityAtom); set(persistActivityAtom, next);
  try { localStorage.setItem(PERSIST_KEY, String(next)); } catch { /* Review feedback reports unavailable durable storage. */ }
  if (!next) { clearPersistedEvents(get(currentSessionAtom)); set(restoredEventsAtom, []); }
});
export const clearActivityAtom = atom(null, (get, set) => {
  const session = get(currentSessionAtom);
  set(streamEventsAtom, []); set(restoredEventsAtom, []);
  set(localReviewsAtom, get(localReviewsAtom).filter(event => event.data.source.activitySession !== session));
  clearPersistedEvents(session);
});
export function useActivitySync() {
  const persist = useAtomValue(persistActivityAtom);
  const session = useAtomValue(currentSessionAtom);
  const combinedEvents = useAtomValue(combinedEventsAtom);
  const restoredSession = useAtomValue(restoredSessionAtom);
  const setRestored = useSetAtom(restoredEventsAtom);
  const setRestoredSession = useSetAtom(restoredSessionAtom);
  useEffect(() => {
    setRestored(persist ? loadPersistedEvents(session) : []); setRestoredSession(session);
  }, [persist, session, setRestored, setRestoredSession]);
  useEffect(() => {
    if (persist && restoredSession === session && combinedEvents.length > 0) savePersistedEvents(session, combinedEvents);
  }, [persist, session, restoredSession, combinedEvents]);
}
