/** Exact retained owner context. Labels do not establish a speaker's identity. */
export interface TeachingBinding {
  owner: string; desktopId: string; contextRevision: number;
  slot: number; generation: number; application: string; taskId: string; sessionId: string;
}
export interface TeachingSelection {
  recordingId: string; narrationSourceId: string; expected: TeachingBinding;
}
export interface TeachingArtifact {artifact: string; sha256: string}
export interface TeachingGap {tMs: number; scope?: string; reason?: string}
export interface TeachingRecording {
  schemaVersion: number; selection: TeachingSelection; taskId: string;
  binding: TeachingBinding; recordingId: string; state: string; durationMs: number;
  retention: {pinned: boolean; expiresAtUnixMs: number | null};
  video: TeachingArtifact; events: TeachingArtifact;
  narration: TeachingArtifact & {sourceId: string};
  sourceBindings: Array<{tMs: number; detail: unknown}>;
  gaps: TeachingGap[];
  inputEvidence: Array<{tMs: number; source?: string; delivery?: string; applicationAcknowledged: false}>;
  alignment: {state: 'unverified'; recordingOriginMs: number; normalizedOriginMs: number; conversionOffsetMs: null; uncertaintyMs: null; reason: string};
  speakerIdentity: 'unverified'; criticalTokens: 'requires_review';
}
export interface TeachingBundle extends TeachingRecording {
  speechSource: {artifactId: string; revision: string; durationMs: number; originalSha256: string};
}
export interface TeachingRange {startMs: number; endMs: number}
export interface TeachingSegment {
  operationId: string; recordingId: string; narrationSha256: string;
  request: TeachingRange & {requestId: string; artifactId: string; revision: string; language: 'en-US'};
}
export interface TeachingSequenceGap {
  fromRecordingId: string; toRecordingId: string;
  reason: 'protected_input' | 'operator_pause' | 'source_change'; evidenceReference: string;
}
export type RecordingAccess = (request: {recordingId: string; artifact?: string; offset?: number; length?: number}) => Promise<any>;
/** Bound existing authenticated owner APIs; callers cannot supply private source paths. */
export interface TeachingSpeechAdapter {
  capabilities(): Promise<{maximumInputBytes: number; maximumDurationMs: number}>;
  upload(input: {requestId: string; owned: true; audioBase64: string}): Promise<any>;
  saveRecording(input: {requestId: string; name: string; kind: 'recording'; artifactId: string; revision: string}): Promise<any>;
  reconcileSaved(requestId: string): Promise<any>;
  savedRecording(artifactId: string): Promise<any>;
  discardUpload(artifactId: string): Promise<any>;
  transcribe(input: TeachingSegment['request']): Promise<any>;
  recordingStatus(operationId: string): Promise<any>;
  transcript(operationId: string): Promise<any>;
  deleteTranscript(operationId: string): Promise<any>;
}
export interface TeachingEvidenceClient {
  inspect(selection: TeachingSelection): Promise<TeachingRecording>;
  inspectSequence(input: {selections: TeachingSelection[]; gaps: TeachingSequenceGap[]}): Promise<{
    schemaVersion: number; taskId: string; recordings: TeachingRecording[];
    gaps: Array<TeachingSequenceGap & {provenance: 'caller_supplied'; durationMs: null}>;
    captureSuppressionVerified: false;
  }>;
  importNarration(selection: TeachingSelection, input: {requestId: string; owned: true}): Promise<TeachingBundle>;
  segmentRanges(bundle: TeachingBundle): TeachingRange[];
  transcribeSegment(bundle: TeachingBundle, input: TeachingRange & {requestId: string}): Promise<TeachingSegment>;
  readTranscript(bundle: TeachingBundle, segment: TeachingSegment): Promise<{
    status: 'available' | 'source_unavailable' | 'transcript_unavailable';
    operationId: string; recordingId?: string; state?: string; transcript?: unknown;
    alignment?: TeachingRecording['alignment']; gaps?: TeachingGap[]; criticalTokens?: 'requires_review';
  }>;
  deleteTranscript(operationId: string): Promise<unknown>;
  readVideoSlice(bundle: TeachingRecording, range?: {offset?: number; length?: number}): Promise<unknown>;
}
export function createTeachingEvidenceClient(options: {recordings: RecordingAccess; speech: TeachingSpeechAdapter}): TeachingEvidenceClient;
