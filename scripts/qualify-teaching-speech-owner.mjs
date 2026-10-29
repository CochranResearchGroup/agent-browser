// Explicit synthetic cross-package acceptance; no installed/private service is contacted.
import assert from 'node:assert/strict';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {createTeachingEvidenceClient} from '../packages/client/src/teaching-evidence.js';
import {createTeachingSpeechAdapter} from '../packages/client/src/teaching-evidence-http.js';
const modulePath=process.argv[2];
if(!modulePath || !modulePath.startsWith('/')) throw Error('Pass an absolute reviewed Speech package dist/index.mjs path');
const speech=await import(pathToFileURL(resolve(modulePath)).href);
const directory=await mkdtemp(join(tmpdir(),'agent-browser-teaching-owner-'));
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const grant={app:'operator',tenant:'owner',audience:'speech-receipts',expiresAt:Date.now()+600000,scopes:['speech:readiness','speech:transcribe','speech:observe','speech:result','speech:cancel','speech:close']};
const baseProvenance={runtime:'synthetic/1',revision:'fixture',rights:'fixture',terms:'fixture',weightsSha256:'a'.repeat(64)};
const policy=backend=>({approvedAt:'2026-10-26T00:00:00Z',approvedBy:'fixture',apps:['operator'],audience:'operator',backend,rights:'fixture',tenant:'owner',terms:'fixture',use:'personal'});
let uploads,saved,core,server;
try {
  // Actual FFmpeg emits isolated Opus/WebM; the Speech owner does actual decode.
  const source=join(directory,'narration.webm');
  execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-f','lavfi','-i','sine=frequency=440:duration=2','-ar','48000','-c:a','libopus',source],{timeout:10000,stdio:'pipe'});
  const audio=await readFile(source);
  uploads=speech.openRecordingUploads({directory:join(directory,'uploads'),conversion:{maximumDurationMs:300000}});
  saved=speech.openSavedArtifacts({directory:join(directory,'saved'),operators:[grant],principals:[grant]});
  let recognitions=0;
  core=speech.openSpeech({directory:join(directory,'core'),principals:[grant],policy:()=>policy('kokoro'),provenance:{...baseProvenance,device:'cpu',model:'fixture'},worker:{synthesize:async()=>{throw Error('not used');}},recording:{
    capacity:()=>({available:true,reservedBytes:0,observedAt:Date.now()}),policy:()=>policy('parakeet-v3'),
    provenance:{...baseProvenance,device:'cuda',language:'en-US',model:'nvidia/parakeet-tdt-0.6b-v3'},
    source:speech.savedRecordingSource({savedArtifacts:saved,temporarySource:uploads.source}),
    worker:{transcribe:async bytes=>{assert.equal(bytes.length,32000);recognitions++;return {status:'speech',text:'June twelve fifteen kilograms',words:[{text:'June twelve fifteen kilograms',startMs:100,endMs:500}],segments:[{text:'June twelve fifteen kilograms',startMs:100,endMs:500}]};}},
  }});
  server=await speech.startHttp(0,{core,recordingUploads:uploads,savedArtifacts:saved,dashboardOperators:[grant],allowedOrigins:['https://teaching.test'],authenticate:token=>token==='Bearer synthetic-speech'?grant:undefined});
  const address=server.address ?? server.url;
  const url=typeof address==='string'?address:`http://127.0.0.1:${server.port ?? address?.port}`;
  const adapter=createTeachingSpeechAdapter({url,token:'synthetic-speech',origin:'https://teaching.test',conversionLimits:uploads.capabilities()});
  const recordingId='a'.repeat(32),video=Buffer.from('synthetic video reference');
  const eventBytes=Buffer.from(JSON.stringify({type:'mode',scope:'audio',reason:'source_binding',tMs:0,detail:{inputs:[{sourceId:'mic',epoch:'epoch-1',sessionId:'session-1'}]}})+'\n');
  const manifest={recordingId,owner:'operator',desktopId:'desktop',contextRevision:0,state:'complete',durationMs:2000,pinned:true,start:{slot:1,generation:1,context:{application:'agent-browser',taskId:'task-1',sessionId:'session-1'},sourceTracks:[{sourceId:'mic'}]},videoSha256:hash(video),eventsSha256:hash(eventBytes),sourceSha256:[hash(audio)]};
  let revoked=false;
  const recordings=async input=>{
    if(revoked)throw Error('revoked');
    if(!input.artifact)return {recordings:[manifest]};
    const bytes=input.artifact==='events.jsonl'?eventBytes:audio,chunk=bytes.subarray(input.offset,input.offset+input.length);
    return {recordingId,artifact:input.artifact,offset:input.offset,size:bytes.length,base64:chunk.toString('base64'),sha256:hash(chunk),nextOffset:input.offset+chunk.length,eof:input.offset+chunk.length===bytes.length};
  };
  const client=createTeachingEvidenceClient({recordings,speech:adapter});
  const selection={recordingId,narrationSourceId:'mic',expected:{owner:'operator',desktopId:'desktop',contextRevision:0,slot:1,generation:1,application:'agent-browser',taskId:'task-1',sessionId:'session-1'}};
  const bundle=await client.importNarration(selection,{requestId:'synthetic-lesson',owned:true});
  assert.equal((await client.importNarration(selection,{requestId:'synthetic-lesson',owned:true})).speechSource.artifactId,bundle.speechSource.artifactId);
  const segment=await client.transcribeSegment(bundle,{requestId:'synthetic-range',startMs:500,endMs:1500});
  let result;
  for(let attempt=0;attempt<100;attempt++) {
    result=await client.readTranscript(bundle,segment);
    if(result.status==='available')break;
    await new Promise(resolve=>setTimeout(resolve,20));
  }
  assert.equal(result.status,'available');
  assert.equal(result.transcript.segments[0].startMs,600);
  assert.equal(result.transcript.text,'June twelve fifteen kilograms');
  assert.equal(recognitions,1);
  const reopened=createTeachingEvidenceClient({recordings,speech:adapter});
  assert.equal((await reopened.readTranscript(bundle,segment)).status,'available');
  await reopened.deleteTranscript(segment.operationId);
  assert.equal((await reopened.readTranscript(bundle,segment)).status,'transcript_unavailable');
  assert.equal(saved.get(grant,bundle.speechSource.artifactId).metadata.id,bundle.speechSource.artifactId);
  revoked=true;
  assert.equal((await reopened.readTranscript(bundle,segment)).status,'source_unavailable');
  assert.equal(recognitions,1);
  console.log(JSON.stringify({status:'pass',synthetic:true,actualSpeechOwner:true,actualOpusConversion:true,recordingOwner:'contract fixture',recognition:'deterministic worker, no real ASR quality claim',absoluteTranscriptStartMs:600,originalPreserved:true,recognitions}));
} finally {
  await server?.close();await core?.close();await uploads?.close();saved?.close();await rm(directory,{recursive:true,force:true});
}
