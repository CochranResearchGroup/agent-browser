"use client";

import { useState } from 'react';
import { useAtomValue, useSetAtom } from 'jotai/react';
import { Check, X } from 'lucide-react';
import { combinedEventsAtom, recordInspectionReviewAtom } from '@/store/activity';
import { activeSessionNameAtom } from '@/store/sessions';
import { activitySessionName, latestInspectionReview, sameInspectionSelection,
  type InspectionReviewSource, type InspectionSelection, type InspectionVerdict } from '@/lib/inspection-review';

/** Recording a judgment never runs a suggested action or publishes page prose. */
export function InspectionReviewControls({ source, selection, actor }: {
  source: InspectionReviewSource | null; selection: InspectionSelection | null; actor: string | undefined;
}) {
  const session = activitySessionName(useAtomValue(activeSessionNameAtom));
  const events = useAtomValue(combinedEventsAtom);
  const record = useSetAtom(recordInspectionReviewAtom);
  const [feedback, setFeedback] = useState<{ key: string; text: string } | null>(null);
  const current = !!source && source.activitySession === session && sameInspectionSelection(source.selection, selection) && !!actor;
  const key = JSON.stringify([source?.runId, source?.contextPacketHash, selection, session, actor]);
  const latest = source && actor ? latestInspectionReview(events, source, actor) : undefined;
  const review = (verdict: InspectionVerdict) => {
    if (!current || !source || !actor) return;
    const result = record({ source, currentSelection: selection, verdict, actor });
    if (!result.recorded) { setFeedback({ key, text: 'The inspection binding changed. Select its original workspace before recording.' }); return; }
    setFeedback({ key, text: result.persistence === 'saved' ? 'Recorded in Activity and saved for reload.'
      : result.persistence === 'off' ? 'Recorded in Activity for this session. Persistence is off.'
      : 'Recorded in Activity, but browser storage is unavailable. It may be lost on reload.' });
  };
  return <div className="space-y-1 rounded border border-border/50 p-2 text-[11px]">
    <p>Record your judgment in Activity. Only identity and evidence references are included; observation text stays here.</p>
    <div className="flex flex-wrap gap-2">{(['accepted', 'rejected'] as const).map(verdict =>
      <button key={verdict} type="button" onClick={() => review(verdict)} disabled={!current || latest?.data.verdict === verdict}
        title={!current ? 'Select the original workspace with a complete inspection run and signed-in reviewer.' : `Record as ${verdict}; this does not execute any action.`}
        className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 disabled:opacity-40">
        {verdict === 'accepted' ? <Check className="size-3" aria-hidden="true" /> : <X className="size-3" aria-hidden="true" />}
        Record as {verdict}
      </button>)}</div>
    {!current && <p>Select the original workspace and run a fully attributed inspection before recording a review.</p>}
    {feedback?.key === key && <p role="status">{feedback.text}</p>}
  </div>;
}
