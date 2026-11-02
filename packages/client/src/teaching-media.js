/** Authorized, bounded recording consumption for an inert agent interpretation.
 * Remote View retains recording/playback ownership. No media paths or credentials
 * are returned, and Speech range/source revisions are preserved independently.
 */
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
const digest=value=>createHash('sha256').update(value).digest('hex');
const fail=code=>{throw new Error(code)};
const integer=value=>Number.isSafeInteger(value)&&value>=0;
const MAX_VIDEO=64*1024*1024, MAX_FRAME=128*1024, MAX_PACKET=900*1024;

async function decodeFrame({videoBytes,timeMs}) {
 const directory=await mkdtemp(join(tmpdir(),'agent-browser-teaching-frame-'));
 try {
  const input=join(directory,'source.webm');await writeFile(input,videoBytes,{mode:0o600});
  const bytes=await new Promise((resolve,reject)=>{
   const child=spawn('ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-max_alloc','67108864','-protocol_whitelist','file,pipe','-f','matroska','-ss',String(timeMs/1000),'-i',input,'-frames:v','1','-vf',"scale=w='min(960,iw)':h='min(960,ih)':force_original_aspect_ratio=decrease",'-q:v','8','-f','image2pipe','-vcodec','mjpeg','pipe:1'],{stdio:['ignore','pipe','ignore'],windowsHide:true,env:{PATH:process.env.PATH??'',HOME:directory,XDG_CONFIG_HOME:directory,XDG_CACHE_HOME:directory,XDG_DATA_HOME:directory,...(process.env.SystemRoot?{SystemRoot:process.env.SystemRoot}:{}),...(process.env.SYSTEMROOT?{SYSTEMROOT:process.env.SYSTEMROOT}:{})}});
   const chunks=[];let size=0,error=null;
   const timer=setTimeout(()=>{error='teaching_frame_timeout';child.kill('SIGKILL')},15000);
   child.stdout.on('data',chunk=>{size+=chunk.length;if(size>MAX_FRAME){error='teaching_frame_limit';child.kill('SIGKILL')}else chunks.push(chunk)});
   child.on('error',()=>{clearTimeout(timer);reject(new Error('teaching_frame_decoder_unavailable'))});
   child.on('close',code=>{clearTimeout(timer);if(error||code!==0)return reject(new Error(error||'teaching_frame_decode_failed'));if(!size)return reject(new Error('teaching_frame_empty'));resolve(Buffer.concat(chunks))});
  });
  return {bytes,mimeType:'image/jpeg'};
 } finally {await rm(directory,{recursive:true,force:true})}
}
function sameSource(observed,bundle) {
 return observed?.recordingId===bundle.recordingId&&observed.taskId===bundle.taskId&&observed.video?.sha256===bundle.video?.sha256&&observed.narration?.sha256===bundle.narration?.sha256;
}
/** Test ports replace only scoped owner transport/decoder, never ownership checks. */
export async function loadTeachingMediaPacket({evidence,bundle,transcripts,context,operationId,intervals,decodeFrame:decoder=decodeFrame}) {
 if(!evidence?.readVideoSlice||!evidence?.inspect||!bundle||!context||context.taskId!==bundle.taskId||!['complete','interrupted'].includes(bundle.state)||!integer(bundle.durationMs)||bundle.durationMs<1)fail('teaching_media_context_invalid');
 if(typeof operationId!=='string'||!operationId||operationId.length>128||['taskId','profileId','sitePolicyId'].some(key=>typeof context[key]!=='string'||!context[key]||context[key].length>4000))fail('teaching_media_context_invalid');
 if(!Array.isArray(transcripts)||!transcripts.length||transcripts.length>12)fail('teaching_transcript_required');
 if(!Array.isArray(context.allowedOrigins)||!context.allowedOrigins.length||context.allowedOrigins.length>32||context.allowedOrigins.some(origin=>typeof origin!=='string'||origin.length>2048||!/^https?:\/\//.test(origin))||['account','organization'].some(key=>context[key]!==undefined&&(typeof context[key]!=='string'||context[key].length>4000)))fail('teaching_media_context_invalid');
 let textBytes=0;const speech=bundle.speechSource;
 const normalized=transcripts.map(input=>{
  if(input.status!==undefined&&input.status!=='available')fail('teaching_transcript_unavailable');
  const raw=input.transcript??input;const source=raw.source??raw;
  const row={operationId:raw.operationId??input.operationId,artifactId:source.artifactId,revision:source.revision,startMs:source.startMs,endMs:source.endMs,text:raw.text};
  if(input.transcript&&input.operationId!==row.operationId)fail('teaching_transcript_binding_changed');
  if(!speech||row.artifactId!==speech.artifactId||row.revision!==speech.revision||!integer(row.startMs)||!integer(row.endMs)||row.endMs<=row.startMs||row.endMs>speech.durationMs||typeof row.operationId!=='string'||!row.operationId||typeof row.text!=='string'||!row.text.trim())fail('teaching_transcript_binding_changed');
  textBytes+=Buffer.byteLength(row.text);return {operationId:row.operationId,artifactId:row.artifactId,revision:row.revision,startMs:row.startMs,endMs:row.endMs,text:row.text};
 });
 if(textBytes>65536)fail('teaching_transcript_limit');
 if(!sameSource(await evidence.inspect(bundle.selection),bundle))fail('teaching_media_source_changed');
 const parts=[];let offset=0,total=null,video;
 try {
  for(;;){
   const reply=await evidence.readVideoSlice(bundle,{offset,length:65536});
   const bytes=Buffer.from(reply.base64??'','base64');
   if(reply.recordingId!==bundle.recordingId||reply.artifact!=='video.webm'||reply.offset!==offset||!integer(reply.size)||reply.size>MAX_VIDEO||(total!==null&&reply.size!==total)||reply.nextOffset!==offset+bytes.length||reply.nextOffset>reply.size||bytes.length>65536||bytes.toString('base64')!==reply.base64||digest(bytes)!==reply.sha256||reply.eof!==(reply.nextOffset===reply.size)||(!reply.eof&&!bytes.length))fail('teaching_media_video_invalid');
   total=reply.size;parts.push(bytes);offset=reply.nextOffset;if(reply.eof)break;
  }
  video=Buffer.concat(parts);if(!video.length||digest(video)!==bundle.video.sha256)fail('teaching_media_video_changed');
  let ranges=intervals;
  if(ranges===undefined){const times=[0,...(bundle.inputEvidence??[]).filter(row=>integer(row.tMs)&&row.tMs<bundle.durationMs).map(row=>row.tMs),Math.floor(bundle.durationMs/3),Math.floor(bundle.durationMs*2/3)];ranges=[...new Set(times)].slice(0,4).map(startMs=>({startMs,endMs:Math.min(bundle.durationMs,startMs+1000)}))}
  if(!Array.isArray(ranges)||!ranges.length||ranges.length>4||ranges.some(row=>!integer(row.startMs)||!integer(row.endMs)||row.endMs<=row.startMs||row.endMs>bundle.durationMs))fail('teaching_frame_range_invalid');
  const frames=[],gaps=structuredClone(bundle.gaps??[]);
  for(const range of ranges){
   let frame;try{frame=await decoder({videoBytes:video,timeMs:range.startMs,startMs:range.startMs,endMs:range.endMs})}catch(error){if(error.message==='teaching_frame_empty'){gaps.push({tMs:range.startMs,scope:'agent_visual_sampling',reason:'frame_unavailable'});continue}throw error}
   const bytes=Buffer.from(frame.bytes);if(!bytes.length||bytes.length>MAX_FRAME||!['image/jpeg','image/png'].includes(frame.mimeType))fail('teaching_frame_invalid');
   const sha256=digest(bytes);frames.push({evidenceId:'frame:'+digest(`${bundle.recordingId}:${bundle.video.sha256}:${range.startMs}:${sha256}`),...range,sha256,mimeType:frame.mimeType,dataBase64:bytes.toString('base64')});
  }
  if(!frames.length)fail('teaching_frame_unavailable');
  if(!sameSource(await evidence.inspect(bundle.selection),bundle))fail('teaching_media_source_changed');
  const packet={schemaVersion:'agent-browser.teaching-agent-media.v1',operationId,taskId:context.taskId,profileId:context.profileId,sitePolicyId:context.sitePolicyId,allowedOrigins:[...context.allowedOrigins],intendedIdentity:{account:context.account??'',organization:context.organization??''},recordingId:bundle.recordingId,videoSha256:bundle.video.sha256,narrationSha256:bundle.narration.sha256,gaps,alignment:structuredClone(bundle.alignment),frames,transcripts:normalized};
  if(Buffer.byteLength(JSON.stringify(packet))>MAX_PACKET)fail('teaching_media_packet_limit');return packet;
 } finally {video?.fill(0);for(const bytes of parts)bytes.fill(0)}
}
