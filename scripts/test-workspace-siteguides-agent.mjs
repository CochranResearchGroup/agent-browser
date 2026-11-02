import assert from 'node:assert/strict';import{mkdtemp,readFile,rm}from'node:fs/promises';import{join}from'node:path';import{tmpdir}from'node:os';import{createHash}from'node:crypto';
import{createWorkspaceSiteGuides}from'../packages/client/src/workspace-siteguides.js';import{createTeachingEvidenceClient}from'../packages/client/src/teaching-evidence.js';import{createPlaybookClient}from'../packages/client/src/playbooks.js';
const hash=bytes=>createHash('sha256').update(bytes).digest('hex'),root=await mkdtemp(join(tmpdir(),'workspace-agent-'));
try{
 const recordingId='a'.repeat(32),audio=Buffer.from('synthetic narration transport'),video=Buffer.from('synthetic video transport'),events=Buffer.from('');let expired=false,reads=0;
 const selection={recordingId,narrationSourceId:'mic',expected:{owner:'operator',desktopId:'desktop',contextRevision:2,slot:1,generation:3,application:'agent-browser',taskId:'task',sessionId:'session'}};
 const manifest={recordingId,owner:'operator',desktopId:'desktop',contextRevision:2,state:'complete',durationMs:1000,pinned:true,expiresAtUnixMs:null,start:{slot:1,generation:3,context:{application:'agent-browser',taskId:'task',sessionId:'session'},sourceTracks:[{sourceId:'mic'}]},videoSha256:hash(video),eventsSha256:hash(events),sourceSha256:[hash(audio)]};
 const artifacts={'source-0.webm':audio,'video.webm':video,'events.jsonl':events};
 const evidence=createTeachingEvidenceClient({recordings:async input=>{reads++;if(expired)throw Error('revoked');if(!input.artifact)return{recordings:[manifest]};const bytes=artifacts[input.artifact],part=bytes.subarray(input.offset,input.offset+input.length);return{recordingId,artifact:input.artifact,offset:input.offset,size:bytes.length,base64:part.toString('base64'),sha256:hash(part),nextOffset:input.offset+part.length,eof:input.offset+part.length===bytes.length};},speech:{}});
 const browser=createPlaybookClient(async(name,args)=>({content:[{type:'text',text:JSON.stringify({success:true,data:args.params.operation==='validate_teaching_candidate'?{valid:true}:JSON.parse(await readFile(join(root,args.params.guideId+'.json'),'utf8'))})}]}));
 const options={candidateDirectory:root,browser,evidence,authorizeTask:async task=>task==='task',resolveSelection:async()=>selection};const owner=createWorkspaceSiteGuides(options);
 const request={taskId:'task',sourceKey:'recording-source',rootGuideId:'teaching-original',operationId:'teaching-original'};
 const inspected=await owner.dispatch({...request,operation:'inspect_source'});assert.equal(inspected.recordingId,recordingId);
 const bytes=await owner.dispatch({...request,operation:'read_video',range:{offset:0,length:65536}});assert.equal(Buffer.from(bytes.base64,'base64').toString(),video.toString());
 assert.equal((await owner.dispatch({...request,operation:'begin_agent'})).status,'created');
 assert.equal((await owner.dispatch({...request,operation:'begin_agent'})).status,'pending');
 await owner.dispatch({...request,operation:'bind_agent_media',packet:{recordingId,videoSha256:hash(video),frames:[{sha256:'b'.repeat(64)}]}});
 const guide=JSON.parse(await readFile(new URL('./fixtures/teaching-report/draft.json',import.meta.url),'utf8'));guide.guideId=request.rootGuideId;guide.teaching.sourceTaskId='task';guide.teaching.recordingId=recordingId;guide.teaching.videoSha256=hash(video);
 await owner.dispatch({...request,operation:'record_agent_result',response:{guide,questions:[]}});
 const staged=await owner.dispatch({...request,operation:'stage_agent_result'});assert.equal(staged.guideId,request.rootGuideId);assert.equal(staged.ordinaryReuseAllowed,false);
 const recovered=await createWorkspaceSiteGuides(options).dispatch({...request,operation:'read_agent'});assert.equal(recovered.status,'resolved');assert.deepEqual(recovered.response.guide,guide);
 assert.equal((await owner.dispatch({...request,operation:'stage_agent_result'})).sha256,staged.sha256);
 expired=true;await assert.rejects(owner.dispatch({...request,operation:'inspect_source'}),/revoked/);
 await assert.rejects(owner.dispatch({...request,taskId:'peer',operation:'agent_packet'}),/task_denied/);assert(reads>0);
 console.log('Workspace teaching composition: exact real recording protocol, bounded evidence, immutable intent/result, useful inert publication and source/policy refusal.');
}finally{await rm(root,{recursive:true,force:true});}
