// @ts-check
import {readdir,readFile,lstat} from 'node:fs/promises';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {createTeachingReviewClient} from './teaching-review.js';
import {createTeachingGrillingClient} from './teaching-grilling.js';
import {writeTeachingDraftCandidate,createTeachingAgentSubmissionOwner} from './teaching-draft.js';
const sha=value=>createHash('sha256').update(value).digest('hex');
const fail=reason=>{throw Error(reason);};

/**
 * Composition of the existing candidate directory and teaching owners. Trusted
 * receiving ports belong to server configuration, never dashboard JSON. There
 * is no second guide registry, approval store, recorder or viewer here.
 * @param {import('./workspace-siteguides.js').WorkspaceSiteGuidesOptions} options
 */
export function createWorkspaceSiteGuides(options) {
 const {candidateDirectory,browser,evidence,resolveSelection,authorizeTask}=options;
 const submissions=createTeachingAgentSubmissionOwner({candidateDirectory});
 const review=createTeachingReviewClient(options);
 const grilling=createTeachingGrillingClient({review,readOperatorAnswer:options.readOperatorAnswer});
 async function records() {
  const stat=await lstat(candidateDirectory);
  if(!stat.isDirectory()||stat.isSymbolicLink()||(process.getuid&&stat.uid!==process.getuid())||(process.platform!=='win32'&&(stat.mode&0o077)))fail('teaching_review_directory_invalid');
  const names=(await readdir(candidateDirectory)).filter(name=>/^[a-z0-9][a-z0-9-]{0,119}\.json$/.test(name));
  if(names.length>4096)fail('siteguides_catalog_limit');
  const result=[];
  for(const name of names) {
   const path=join(candidateDirectory,name),entry=await lstat(path);
   if(!entry.isFile()||entry.isSymbolicLink()||entry.size>65536)fail('teaching_review_file_invalid');
   const bytes=await readFile(path);if(bytes.length>65536)fail('teaching_review_file_invalid');
   const guide=JSON.parse(bytes.toString('utf8'));
   if(guide.guideId+'.json'!==name)fail('teaching_review_identity_changed');
   result.push({guide,sha256:sha(bytes)});
  }
  return result;
 }
 /** @param {string} taskId @param {'read'|'use'} [mode] */
 async function allowed(taskId,mode='read') {if(authorizeTask&&!await authorizeTask(taskId,mode))fail('siteguides_task_denied');}
 async function selection(request) {
  if(!evidence||!resolveSelection)fail('teaching_source_configuration_unavailable');
  await allowed(request.taskId,'use');
  const selected=await resolveSelection({taskId:request.taskId,sourceKey:request.sourceKey});
  if(!selected||selected.expected?.taskId!==request.taskId)fail('teaching_source_context_mismatch');
  return selected;
 }
 async function open(request) {
  await allowed(request.taskId);
  const view=await grilling.open({taskId:request.taskId,guideId:request.guideId});
  await allowed(view.guide.teaching.sourceTaskId);
  const history=(await records()).filter(item=>(item.guide.teaching?.lineage?.rootGuideId??item.guide.guideId)===view.rootGuideId).map(item=>({guideId:item.guide.guideId,sha256:item.sha256,revision:item.guide.teaching?.lineage?.revision??1,operationId:item.guide.teaching?.lineage?.operationId})).sort((a,b)=>a.revision-b.revision);
  return {view,history};
 }
 return {async dispatch(request) {
  if(!request||typeof request!=='object')fail('siteguides_request_required');
  switch(request.operation) {
   case 'list': {
    const all=await records();
    const roots=all.filter(item=>item.guide.teaching&&!item.guide.teaching.lineage);
    const guides=[];
    for(const item of roots) {
     const taskId=item.guide.teaching.sourceTaskId;
     if(authorizeTask&&!await authorizeTask(taskId,'read'))continue;
     const versions=all.filter(version=>(version.guide.teaching?.lineage?.rootGuideId??version.guide.guideId)===item.guide.guideId);
     const latest=versions.sort((a,b)=>(b.guide.teaching?.lineage?.revision??1)-(a.guide.teaching?.lineage?.revision??1))[0];
     const {view,history}=await open({taskId,guideId:latest.guide.guideId});
     guides.push({guideId:view.guideId,rootGuideId:view.rootGuideId,name:view.guide.name,purpose:view.guide.teaching.playbook.goal,allowedOrigins:view.guide.allowedOrigins,sourceTaskId:taskId,review:view.review.state,repeat:view.validation.state,source:view.source,acceptedVersion:view.acceptedVersion,ordinaryReuseAllowed:view.ordinaryReuseAllowed,history});
    }
    return {guides};
   }
   case 'open':return open(request);
   case 'lookup_operation': {
    await allowed(request.taskId);
    if(request.rootGuideId===request.operationId) {
     const existing=(await records()).find(item=>item.guide.guideId===request.rootGuideId);
     if(existing){await allowed(existing.guide.teaching.sourceTaskId);return {status:'saved',guideId:existing.guide.guideId,rootGuideId:request.rootGuideId,operationId:request.operationId,sha256:existing.sha256,recoveredOriginal:true,ordinaryReuseAllowed:false};}
    }
    return review.lookupOperation(request);
   }
   case 'begin_agent': {
    await allowed(request.taskId,'use');await selection(request);
    if(request.rootGuideId!==request.operationId)fail('teaching_stage_original_operation_required');
    const inputSha256=sha(JSON.stringify({taskId:request.taskId,rootGuideId:request.rootGuideId,operationId:request.operationId,sourceKey:request.sourceKey,bundle:request.bundle,segments:request.segments,context:options.context}));
    return submissions.begin({...request,inputSha256});
   }
   case 'read_agent':await allowed(request.taskId);return submissions.read(request);
   case 'bind_agent_media':await allowed(request.taskId,'use');return submissions.bindMedia({...request,mediaSha256:sha(JSON.stringify(request.packet))});
   case 'record_agent_result':await allowed(request.taskId,'use');return submissions.recordResult({...request,response:request.response});
   case 'stage_agent_result': {
    await allowed(request.taskId,'use');
    const original=await submissions.read(request);
    if(original.status!=='resolved')return original;
    const response=/** @type {{status?:string;guide?:import('./teaching-draft.js').TeachingSiteGuide;questions?:Array<Pick<import('./teaching-draft.js').TeachingClarificationQuestion,'id'|'category'|'stepRefs'|'dependsOn'>>}} */(original.response);
    if(response?.status==='unavailable'||response?.status==='unknown')return response;
    if(!response?.guide)fail('teaching_agent_result_invalid');
    const guide=response.guide;
    const selected=await selection(request),source=await evidence.inspect(selected);
    if(guide.guideId!==request.rootGuideId||guide.teaching?.sourceTaskId!==request.taskId||guide.teaching.recordingId!==source.recordingId||guide.teaching.videoSha256!==source.video.sha256)fail('teaching_source_context_mismatch');
    const validation=await browser.validateTeachingCandidate({taskId:request.taskId,guide});if(validation?.valid!==true)fail('teaching_candidate_validation_failed');
    let staged=(await records()).find(item=>item.guide.guideId===request.rootGuideId);
    if(staged){if(JSON.stringify(staged.guide)!==JSON.stringify(guide))fail('teaching_agent_published_content_changed');}
    else {const written=await writeTeachingDraftCandidate({candidateDirectory,candidateOnly:true,guide});staged={guide,sha256:written.sha256};}
    if(response.questions?.length){const prepared=await grilling.prepare({taskId:request.taskId,guideId:guide.guideId,expectedSha256:staged.sha256,operationId:request.operationId+'-questions',questions:response.questions});return {...prepared,rootGuideId:request.rootGuideId,agentOperationId:request.operationId};}
    return {status:'saved',guideId:guide.guideId,rootGuideId:request.rootGuideId,sha256:staged.sha256,agentOperationId:request.operationId,ordinaryReuseAllowed:false};
   }
   case 'list_sources':await allowed(request.taskId,'use');if(!options.listSources)fail('teaching_source_configuration_unavailable');return {sources:await options.listSources(request.taskId)};
   case 'read_video': {const selected=await selection(request);return evidence.readVideoSlice(await evidence.inspect(selected),request.range);}
   case 'inspect_source':return evidence.inspect(await selection(request));
   case 'reconcile_narration':return evidence.reconcileNarration(await selection(request),{requestId:request.requestId});
   case 'import_narration':return evidence.importNarration(await selection(request),{requestId:request.requestId,owned:true});
   case 'transcribe': {const selected=await selection(request);const current=await evidence.inspect(selected);if(current.recordingId!==request.bundle?.recordingId||current.video.sha256!==request.bundle?.video?.sha256)fail('teaching_source_context_mismatch');return evidence.transcribeSegment(request.bundle,request.range);}
   case 'read_transcript':await selection(request);return evidence.readTranscript(request.bundle,request.segment);
   case 'agent_packet': {
    const selected=await selection(request),source=await evidence.inspect(selected);
    return {schemaVersion:'agent-browser.teaching-agent-packet.v1',state:'awaiting_agent',taskId:request.taskId,sourceKey:request.sourceKey,source,readEvidence:{operation:'read_video',taskId:request.taskId,sourceKey:request.sourceKey},rootGuideId:request.rootGuideId,operationId:request.operationId,instruction:'Inspect the exact authorized video and narration evidence. Stage a useful inert SiteGuide candidate before proposing focused questions. Never infer speaker identity, approve content, or claim fresh-repeat success.'};
   }
   case 'stage_draft': {
    const selected=await selection(request),source=await evidence.inspect(selected),guide=request.guide;
    if(request.rootGuideId!==request.operationId||guide?.guideId!==request.rootGuideId)fail('teaching_stage_original_operation_required');
    if(guide?.teaching?.sourceTaskId!==request.taskId||guide.teaching.recordingId!==source.recordingId||guide.teaching.videoSha256!==source.video.sha256)fail('teaching_source_context_mismatch');
    const validation=await browser.validateTeachingCandidate({taskId:request.taskId,guide});if(validation?.valid!==true)fail('teaching_candidate_validation_failed');
    return writeTeachingDraftCandidate({candidateDirectory,candidateOnly:true,guide});
   }
   case 'save':case 'prepare_questions':case 'answer':case 'record_decision':case 'record_repeat':case 'promote': {
    await allowed(request.taskId,'use');await open(request);
    const methods={save:()=>review.save(request),prepare_questions:()=>grilling.prepare(request),answer:()=>grilling.answer(request),record_decision:()=>review.recordDecision(request),record_repeat:()=>review.recordFreshRepeat(request),promote:()=>review.promote(request)};
    return methods[request.operation]();
   }
   default:fail('siteguides_operation_unsupported');
  }
 }};
}
