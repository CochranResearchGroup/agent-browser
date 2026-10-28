import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createTeachingEvidenceClient} from '../packages/client/src/teaching-evidence.js';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const recordingId = 'a'.repeat(32);
const audio = Buffer.from('synthetic isolated narration');
const video = Buffer.from('synthetic desktop video');
const events = Buffer.from([
  {type:'start',tMs:0},
  {type:'mode',scope:'audio',reason:'source_binding',tMs:0,detail:{sourceId:'operator-mic',epoch:4,sessionId:'session-1',sampleRate:48000}},
  {type:'input',tMs:1100,source:'human_viewer',delivery:'sent_to_transport',input:{keysym:65,pressed:true}},
  {type:'gap',scope:'audio',tMs:1800,reason:'lease_expired'},
].map(JSON.stringify).join('\n')+'\n');
const manifest = {recordingId,owner:'operator-1',desktopId:'desktop-1',contextRevision:2,state:'complete',durationMs:400000,pinned:true,expiresAtUnixMs:null,start:{slot:2,generation:7,context:{application:'agent-browser',taskId:'task-1',sessionId:'session-1'},sourceTracks:[{sourceId:'operator-mic'}]},videoSha256:hash(video),eventsSha256:hash(events),sourceSha256:[hash(audio)]};
const selection={recordingId,narrationSourceId:'operator-mic',expected:{owner:'operator-1',desktopId:'desktop-1',contextRevision:2,slot:2,generation:7,application:'agent-browser',taskId:'task-1',sessionId:'session-1'}};
let revoked=false, reads=0, submits=0;
const artifacts={'source-0.webm':audio,'video.webm':video,'events.jsonl':events};
const recordings=async input=>{
  reads++;
  if(revoked) throw Error('private provider error with token');
  if(!input.artifact)return {recordings:[structuredClone(manifest)]};
  const bytes=artifacts[input.artifact], part=bytes.subarray(input.offset,input.offset+input.length);
  return {recordingId,artifact:input.artifact,offset:input.offset,size:bytes.length,base64:part.toString('base64'),sha256:hash(part),nextOffset:input.offset+part.length,eof:input.offset+part.length===bytes.length};
};
const speech={
  reconcileSaved:async()=>({status:'not_found'}),
  capabilities:async()=>({maximumInputBytes:1000000,maximumDurationMs:500000}),
  upload:async()=>({artifactId:'upload-1',revision:hash(audio),durationMs:400001}),
  saveRecording:async()=>({artifactId:'saved-1'}),
  savedRecording:async()=>({id:'saved-1',kind:'recording',mediaType:'audio/wav',sha256:'b'.repeat(64),provenance:{durationMs:400001,sourceRevision:hash(audio)}}),
  discardUpload:async()=>({disposed:true}),
  transcribe:async input=>{submits++;return {id:'op-1',state:'completed',request:input};},
  recordingStatus:async()=>({id:'op-1',state:'completed'}),
  transcript:async()=>({operationId:'op-1',source:{artifactId:'saved-1',revision:'b'.repeat(64),startMs:300000,endMs:400001},status:'speech',text:'June 12: 15 kilograms',words:[{text:'June',startMs:300010,endMs:300020}],segments:[{text:'June 12: 15 kilograms',startMs:300010,endMs:300100}]}),
  deleteTranscript:async()=>({deleted:true}),
};
const client=createTeachingEvidenceClient({recordings,speech});
const inspected=await client.inspect(selection);
assert.equal(inspected.taskId,'task-1');
assert.equal(inspected.narration.artifact,'source-0.webm');
assert.equal(inspected.inputEvidence[0].delivery,'sent_to_transport');
assert.equal(JSON.stringify(inspected).includes('keysym'),false);
assert.equal(inspected.gaps[0].reason,'lease_expired');
assert.equal(inspected.sourceBindings.length,1);
const bundle=await client.importNarration(selection,{requestId:'lesson-1',owned:true});
assert.deepEqual(client.segmentRanges(bundle),[{startMs:0,endMs:300000},{startMs:300000,endMs:400001}]);
const segment=await client.transcribeSegment(bundle,{requestId:'lesson-part-2',startMs:300000,endMs:400001});
const reopened=createTeachingEvidenceClient({recordings,speech});
const result=await reopened.readTranscript(bundle,segment);
assert.equal(result.transcript.text,'June 12: 15 kilograms');
assert.equal(result.transcript.segments[0].startMs,300010);
assert.equal(result.alignment.state,'unverified');
assert.equal(submits,1);
revoked=true;
const unavailable=await reopened.readTranscript(bundle,segment);
assert.equal(unavailable.status,'source_unavailable');
assert.equal('transcript' in unavailable,false);
assert.equal(submits,1);
assert.equal(JSON.stringify(unavailable).includes('token'),false);
assert.ok(reads>0);
console.log('teaching evidence: scoped selection, explicit segmentation and reopen pass');

// Cross-owner access failures must not create copies or substitute peer sources.
revoked=false;
for(const patch of [
  {expected:{...selection.expected,taskId:'peer-task'}},
  {expected:{...selection.expected,contextRevision:1}},
  {narrationSourceId:'desktop-output'},
]) await assert.rejects(client.inspect({...selection,...patch}),/teaching_/);
assert.equal(submits,1);
const oldEvents=artifacts['events.jsonl'];
artifacts['events.jsonl']=Buffer.from('tampered');
await assert.rejects(client.inspect(selection),/teaching_artifact_changed/);
artifacts['events.jsonl']=oldEvents;
await assert.rejects(client.transcribeSegment(bundle,{requestId:'long',startMs:0,endMs:400001}),/teaching_range_invalid/);
assert.equal(submits,1);

// The concrete adapters use owner HTTP contracts, preserve bearer separation,
// reject redirects and keep provider error bodies out of caller errors.
const {createTeachingRecordingReader,createTeachingSpeechAdapter}=await import('../packages/client/src/teaching-evidence-http.js');
const calls=[];
const httpFetch=async(url,options)=>{
  calls.push({url,options});
  return Response.json(url.endsWith('/transcript')?{operationId:'original-op'}:{recordings:[manifest]});
};
const reader=createTeachingRecordingReader({url:'http://127.0.0.1:9999/provider',accessToken:'scoped-record',fetch:httpFetch});
await reader({recordingId});
assert.equal(calls[0].url,'http://127.0.0.1:9999/provider/v1/recording-access');
assert.equal(calls[0].options.headers.Authorization,'Bearer scoped-record');
assert.equal(calls[0].options.redirect,'error');
const httpSpeech=createTeachingSpeechAdapter({url:'https://speech.test/prefix',token:'speech-owner',origin:'https://workspace.test',conversionLimits:{maximumInputBytes:1000000,maximumDurationMs:500000},fetch:httpFetch});
await httpSpeech.transcript('original-op');
assert.equal(calls[1].url,'https://speech.test/prefix/v1/recordings/original-op/transcript');
assert.equal(calls[1].options.headers.Authorization,'Bearer speech-owner');
const badReader=createTeachingRecordingReader({url:'http://127.0.0.1:9999',accessToken:'scoped-record',fetch:async()=>Response.json({private:'secret'}, {status:403})});
await assert.rejects(badReader({recordingId}),/^Error: teaching_owner_rejected_403$/);
console.log('teaching evidence: concrete owner HTTP routes and credential separation pass');

const second={...selection,recordingId:'b'.repeat(32)};
const sequenceClient=createTeachingEvidenceClient({speech,recordings:async input=>{
  const result=await recordings({...input,recordingId});
  if(!input.artifact)result.recordings[0].recordingId=input.recordingId;
  else result.recordingId=input.recordingId;
  return result;
}});
const sequence=await sequenceClient.inspectSequence({selections:[selection,second],gaps:[{fromRecordingId:recordingId,toRecordingId:second.recordingId,reason:'protected_input',evidenceReference:'owner-confirmed-stop-1'}]});
assert.equal(sequence.recordings.length,2);
assert.equal(sequence.gaps[0].provenance,'caller_supplied');
assert.equal(sequence.gaps[0].durationMs,null);
assert.equal(sequence.captureSuppressionVerified,false);
manifest.state='open';
await assert.rejects(client.inspect(selection),/teaching_recording_not_finalized_av/);
await assert.rejects(sequenceClient.inspectSequence({selections:[selection,second],gaps:[{fromRecordingId:recordingId,toRecordingId:second.recordingId,reason:'protected_input',evidenceReference:'owner-stop-1'}]}),/teaching_recording_not_finalized_av/);
manifest.state='complete';
manifest.pinned=false;manifest.expiresAtUnixMs=1;
await assert.rejects(client.inspect(selection),/teaching_source_expired/);
manifest.pinned=true;manifest.expiresAtUnixMs=null;
console.log('teaching evidence: active and expired recording refusal pass');

// A persisted capsule is untrusted: selected Remote View hashes cannot grant
// transcription of a substituted Speech Saved source.
const substituted=structuredClone(bundle);
substituted.speechSource={artifactId:'peer-saved',revision:'c'.repeat(64),durationMs:400001,originalSha256:hash(audio)};
let substitutedSubmits=0;
const substitutedSpeech={...speech,
  savedRecording:async artifactId=>artifactId==='peer-saved'?{id:artifactId,name:'Unrelated peer',kind:'recording',mediaType:'audio/wav',sha256:'c'.repeat(64),provenance:{durationMs:400001,sourceRevision:hash(Buffer.from('unrelated peer narration'))}}:speech.savedRecording(artifactId),
  transcribe:async request=>{substitutedSubmits++;return {id:'peer-operation',state:'completed',request};},
};
const substitutedClient=createTeachingEvidenceClient({recordings,speech:substitutedSpeech});
await assert.rejects(substitutedClient.transcribeSegment(substituted,{requestId:'substituted-source',startMs:0,endMs:1000}),/teaching_saved_source_invalid/);
assert.equal(substitutedSubmits,0,'Unrelated Saved audio must never be transcribed');

let substitutedTranscriptReads=0;
const peerSegment={operationId:'peer-operation',recordingId,narrationSha256:hash(audio),request:{requestId:'peer-request',artifactId:'peer-saved',revision:'c'.repeat(64),startMs:0,endMs:1000,language:'en-US'}};
const substitutedReader=createTeachingEvidenceClient({recordings,speech:{...substitutedSpeech,
  recordingStatus:async operationId=>({id:operationId,state:'completed'}),
  transcript:async operationId=>{substitutedTranscriptReads++;return {operationId,source:{artifactId:'peer-saved',revision:'c'.repeat(64),startMs:0,endMs:1000},text:'unrelated peer narration'};},
}});
const refusedPeerTranscript=await substitutedReader.readTranscript(substituted,peerSegment);
assert.equal(refusedPeerTranscript.status,'source_unavailable');
assert.equal('transcript' in refusedPeerTranscript,false);
assert.equal(substitutedTranscriptReads,0,'Reopen must recheck Saved provenance before reading text');

// Hash equality does not allow relabeling a selected recording as its peer.
const relabeled=structuredClone(bundle);relabeled.recordingId='d'.repeat(32);
const priorSubmits=submits;
await assert.rejects(client.transcribeSegment(relabeled,{requestId:'relabeled-source',startMs:0,endMs:1000}),/teaching_source_changed/);
assert.equal(submits,priorSubmits);

// Ordinary exact Agent Browser task IDs may be longer than a display label.
const longTaskId='session:workflow-'+ 'e'.repeat(140)+':borrowed-profile:1';
const longSelection=structuredClone(selection);longSelection.expected.taskId=longTaskId;
const longClient=createTeachingEvidenceClient({speech,recordings:async input=>{
  const response=await recordings(input);
  if(!input.artifact)response.recordings[0].start.context.taskId=longTaskId;
  return response;
}});
const longRecording=await longClient.inspect(longSelection);
assert.equal(longRecording.taskId,longTaskId);
await assert.rejects(longClient.inspect({...longSelection,expected:{...longSelection.expected,taskId:longTaskId+'peer'}}),/teaching_binding_changed/);
for(const taskId of ['x'.repeat(4001),'task\u0085break','   '])
  await assert.rejects(longClient.inspect({...longSelection,expected:{...longSelection.expected,taskId}}),/teaching_binding_required/);
console.log('teaching evidence: persisted source substitution refuses and long exact task identity passes');
