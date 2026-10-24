import type { ActivityEvent, ResultMessage } from '@/types';
import type { CodexWorkspaceObservation, SelectedWorkspaceChatPacket } from './selected-workspace-chat-packet';

export type InspectionVerdict = 'accepted' | 'rejected';
export type InspectionSelection = {
  workspaceId: string; browserId: string; sessionId: string; tabId: string; profileId: string;
};
export type InspectionReviewContext = {
  activitySession: string; selection: InspectionSelection; provider: string;
  packetCreatedAt: string; evidenceIds: string[];
};
export type InspectionReviewSource = Omit<InspectionReviewContext, 'evidenceIds'> & {
  runId: string; contextPacketHash: string; observedAt: string; evidenceRefs: string[];
};
export type InspectionReviewData = {
  version: 'inspection-review.v1'; source: InspectionReviewSource;
  verdict: InspectionVerdict; actor: string;
};
export type InspectionReviewEvent = ResultMessage & { action: 'inspection_review'; data: InspectionReviewData };

const id = (value: unknown): value is string => typeof value === 'string' && value.trim().length > 0
  && value.length <= 256 && !/[\u0000-\u001f]/.test(value) && !/^(?:https?:\/\/|data:)/i.test(value);
const date = (value: unknown): value is string => typeof value === 'string' && value.length <= 64 && Number.isFinite(Date.parse(value));
export const activitySessionName = (session: string) => session || 'default';

/** Freeze only identity/reference metadata before awaiting inspection. Site prose,
 * URLs, prompts and private ledger paths never enter the Activity record. */
export function inspectionSelection(packet: SelectedWorkspaceChatPacket | null | undefined): InspectionSelection | null {
  if (!packet) return null;
  const selection = { workspaceId: packet.selection.workspaceId ?? packet.workspace.id,
    browserId: packet.selection.browserId, sessionId: packet.selection.sessionId,
    tabId: packet.selection.tabId, profileId: packet.selection.profileId };
  return Object.values(selection).every(id) ? selection as InspectionSelection : null;
}
export function captureInspectionReviewContext(packet: SelectedWorkspaceChatPacket, session: string): InspectionReviewContext | null {
  const selection = inspectionSelection(packet);
  if (!selection || !date(packet.createdAt) || !id(activitySessionName(session))) return null;
  return { activitySession: activitySessionName(session), selection, provider: packet.provider,
    packetCreatedAt: packet.createdAt,
    evidenceIds: packet.evidence.filter(item => item.included && item.available && id(item.id)).map(item => item.id).slice(0, 32) };
}
export function inspectionReviewSource(context: InspectionReviewContext | null, observation: CodexWorkspaceObservation | undefined,
  ledger: { runId?: string; contextPacketHash?: string } | undefined): InspectionReviewSource | null {
  if (!context || !observation || !id(observation.runId) || !id(ledger?.contextPacketHash)
    || ledger?.runId !== observation.runId || observation.workspaceId !== context.selection.workspaceId
    || observation.provider !== context.provider || !date(observation.createdAt)) return null;
  const cited = [...observation.blockers, ...observation.risks, ...observation.suggestedNextInspections].flatMap(item => item.evidenceIds);
  if (cited.some(ref => !context.evidenceIds.includes(ref))) return null;
  return { activitySession: context.activitySession, selection: { ...context.selection }, provider: context.provider,
    packetCreatedAt: context.packetCreatedAt, runId: observation.runId,
    contextPacketHash: ledger.contextPacketHash, observedAt: observation.createdAt,
    evidenceRefs: [...new Set(cited)].slice(0, 32) };
}
export function sameInspectionSelection(a: InspectionSelection | null, b: InspectionSelection | null): boolean {
  return !!a && !!b && ['workspaceId', 'browserId', 'sessionId', 'tabId', 'profileId'].every(key => a[key as keyof InspectionSelection] === b[key as keyof InspectionSelection]);
}
export function inspectionReviewKey(source: InspectionReviewSource, actor: string): string {
  return JSON.stringify([source.activitySession, source.selection.workspaceId, source.selection.browserId,
    source.selection.sessionId, source.selection.tabId, source.selection.profileId, source.runId, source.contextPacketHash, actor]);
}
/** Treat restored Activity as display data, never as proof or execution authority. */
export function isInspectionReviewEvent(event: ActivityEvent): event is InspectionReviewEvent {
  if (event?.type !== 'result' || event.action !== 'inspection_review') return false;
  const data = event.data as InspectionReviewData | null;
  const source = data?.source;
  return !!source && data?.version === 'inspection-review.v1' && ['accepted', 'rejected'].includes(data.verdict)
    && id(data.actor) && id(source.activitySession) && id(source.runId) && id(source.contextPacketHash)
    && id(source.provider) && !!source.selection && Object.keys(source.selection).length === 5
    && ['workspaceId', 'browserId', 'sessionId', 'tabId', 'profileId'].every(key => id(source.selection[key as keyof InspectionSelection])) && date(source.packetCreatedAt) && date(source.observedAt)
    && Array.isArray(source.evidenceRefs) && source.evidenceRefs.length <= 32 && source.evidenceRefs.every(id)
    && typeof event.id === 'string' && event.id.startsWith('inspection-review:') && typeof event.timestamp === 'number' && Number.isFinite(new Date(event.timestamp).getTime());
}
export function latestInspectionReview(events: ActivityEvent[], source: InspectionReviewSource, actor: string): InspectionReviewEvent | undefined {
  const key = inspectionReviewKey(source, actor);
  return events.filter(isInspectionReviewEvent).filter(event => inspectionReviewKey(event.data.source, event.data.actor) === key).at(-1);
}
