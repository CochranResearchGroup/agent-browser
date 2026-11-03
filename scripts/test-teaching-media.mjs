import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {loadTeachingMediaPacket} from '../packages/client/src/teaching-media.js';
const hash=value=>createHash('sha256').update(value).digest('hex');
const video=Buffer.from('authorized synthetic finalized video');
const image=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jF9sAAAAASUVORK5CYII=','base64');
const bundle={recordingId:'a'.repeat(32),taskId:'task',state:'complete',durationMs:1000,selection:{recordingId:'a'.repeat(32)},video:{artifact:'video.webm',sha256:hash(video)},narration:{sha256:'b'.repeat(64)},speechSource:{artifactId:'audio',revision:'revision',durationMs:1000},gaps:[],alignment:{state:'unverified'},inputEvidence:[{tMs:200}]};
let reads=0,decoded=0;
const evidence={inspect:async()=>structuredClone(bundle),readVideoSlice:async(_bundle,{offset,length})=>{reads++;const bytes=video.subarray(offset,offset+length);return{recordingId:bundle.recordingId,artifact:'video.webm',offset,size:video.length,base64:bytes.toString('base64'),sha256:hash(bytes),nextOffset:offset+bytes.length,eof:offset+bytes.length===video.length}}};
const context={taskId:'task',profileId:'profile',sitePolicyId:'site',allowedOrigins:['https://example.test'],account:'account',organization:'workspace'};
const transcripts=[{operationId:'speech-one',artifactId:'audio',revision:'revision',startMs:0,endMs:500,text:'Open the report.'},{operationId:'speech-two',artifactId:'audio',revision:'revision',startMs:500,endMs:1000,text:'Verify the selected dates.'}];
const packet=await loadTeachingMediaPacket({evidence,bundle,transcripts,context,operationId:'guide',decodeFrame:async({videoBytes})=>{decoded++;assert.equal(hash(videoBytes),bundle.video.sha256);return{bytes:image,mimeType:'image/png'}}});
assert.equal(packet.schemaVersion,'agent-browser.teaching-agent-media.v1');assert.deepEqual(packet.transcripts,transcripts);assert.equal(packet.taskId,'task');assert.equal(packet.frames.length>0,true);assert.equal(packet.frames[0].sha256,hash(image));assert.equal(packet.alignment.state,'unverified');assert.ok(reads>0&&decoded>0);assert.ok(Buffer.byteLength(JSON.stringify(packet))<=900*1024);
let called=false;
await assert.rejects(()=>loadTeachingMediaPacket({evidence,bundle,context,operationId:'bad',transcripts:[{...transcripts[0],revision:'different'}],decodeFrame:async()=>{called=true}}),/transcript_binding/);assert.equal(called,false);
await assert.rejects(()=>loadTeachingMediaPacket({evidence,bundle:{...bundle,video:{...bundle.video,sha256:'f'.repeat(64)}},context,operationId:'bad',transcripts,decodeFrame:async()=>{called=true}}),/source_changed|video_changed/);assert.equal(called,false);
const wrappers=transcripts.map(row=>({status:'available',operationId:row.operationId,transcript:{operationId:row.operationId,source:{artifactId:row.artifactId,revision:row.revision,startMs:row.startMs,endMs:row.endMs},text:row.text}}));
const realShape=await loadTeachingMediaPacket({evidence,bundle,transcripts:wrappers,context,operationId:'wrapped',decodeFrame:async()=>({bytes:image,mimeType:'image/png'})});assert.deepEqual(realShape.transcripts,transcripts);
console.log('Teaching media public consumer passed: scoped bytes, full digest, all transcript ranges, actual frame bytes and unavailable binding.');

// Actual decoder boundary over synthetic WebM, with no user recording or profile.
const {execFileSync}=await import('node:child_process');
const fs=await import('node:fs/promises');const os=await import('node:os');const path=await import('node:path');
const scratch=await fs.mkdtemp(path.join(os.tmpdir(),'teaching-media-decoder-'));
try {
 const file=path.join(scratch,'synthetic.webm');
 execFileSync('ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-f','lavfi','-i','color=c=blue:s=320x200:d=1','-c:v','libvpx-vp9','-an',file],{timeout:15000,stdio:['ignore','ignore','pipe']});
 const real=await fs.readFile(file);const realBundle={...bundle,video:{artifact:'video.webm',sha256:hash(real)},inputEvidence:[]};
 const reader={inspect:async()=>structuredClone(realBundle),readVideoSlice:async(_,{offset,length})=>{const bytes=real.subarray(offset,offset+length);return{recordingId:bundle.recordingId,artifact:'video.webm',offset,size:real.length,base64:bytes.toString('base64'),sha256:hash(bytes),nextOffset:offset+bytes.length,eof:offset+bytes.length===real.length}}};
 const decoded=await loadTeachingMediaPacket({evidence:reader,bundle:realBundle,transcripts,context,operationId:'decoded',intervals:[{startMs:0,endMs:500}]});
 assert.equal(decoded.frames.length,1);assert.equal(decoded.frames[0].mimeType,'image/jpeg');assert.ok(Buffer.from(decoded.frames[0].dataBase64,'base64').subarray(0,2).equals(Buffer.from([255,216])));console.log('Actual bounded ffmpeg consumer decoded a synthetic owner-authorized WebM frame.');
} finally {await fs.rm(scratch,{recursive:true,force:true})}
