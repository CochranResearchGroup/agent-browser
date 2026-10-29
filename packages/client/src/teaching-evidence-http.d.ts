import type {RecordingAccess, TeachingSpeechAdapter} from './teaching-evidence.js';
/** Backend only. Token must be a Remote View single-record read grant. */
export function createTeachingRecordingReader(options: {url: string; accessToken: string; fetch?: typeof globalThis.fetch}): RecordingAccess;
/** Protected deployment configuration; Origin must be allowed by the Speech owner. */
export function createTeachingSpeechAdapter(options: {
  url: string; token: string; origin: string;
  conversionLimits: {maximumInputBytes: number; maximumDurationMs: number};
  fetch?: typeof globalThis.fetch;
}): TeachingSpeechAdapter;
