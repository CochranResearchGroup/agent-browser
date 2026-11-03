import type {TeachingBundle, TeachingEvidenceClient} from './teaching-evidence.js';
export interface TeachingMediaTranscript {operationId:string;artifactId:string;revision:string;startMs:number;endMs:number;text:string}
export interface TeachingMediaPacket {
 schemaVersion:'agent-browser.teaching-agent-media.v1';operationId:string;taskId:string;profileId:string;sitePolicyId:string;allowedOrigins:string[];
 intendedIdentity:{account:string;organization:string};recordingId:string;videoSha256:string;narrationSha256:string;
 gaps:TeachingBundle['gaps'];alignment:TeachingBundle['alignment'];
 frames:Array<{evidenceId:string;startMs:number;endMs:number;sha256:string;mimeType:'image/jpeg'|'image/png';dataBase64:string}>;
 transcripts:TeachingMediaTranscript[];
}
/** Consumes finalized owner media; returns no private paths or authorization. */
export function loadTeachingMediaPacket(options:{
 evidence:Pick<TeachingEvidenceClient,'inspect'|'readVideoSlice'>;bundle:TeachingBundle;
 transcripts:Array<TeachingMediaTranscript|Awaited<ReturnType<TeachingEvidenceClient['readTranscript']>>>;
 context:{taskId:string;profileId:string;sitePolicyId:string;allowedOrigins:string[];account?:string;organization?:string};operationId:string;
 intervals?:Array<{startMs:number;endMs:number}>;
 /** Dependency test port only; never accept executable/decoder configuration from HTTP. */
 decodeFrame?:(input:{videoBytes:Uint8Array;timeMs:number;startMs:number;endMs:number})=>Promise<{bytes:Uint8Array;mimeType:'image/jpeg'|'image/png'}>;
}):Promise<TeachingMediaPacket>;
