// Source-free installed composition host. All imports are packaged support assets.
import {readFile,lstat} from 'node:fs/promises';
import {join,isAbsolute} from 'node:path';
import {spawnSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';
import {createTeachingAgentSubmissionOwner} from './teaching-draft.js';
import {createWorkspaceSiteGuides} from './workspace-siteguides.js';
import {createPlaybookClient} from './playbooks.js';
import {createTeachingEvidenceClient} from './teaching-evidence.js';
import {createTeachingRecordingReader,createTeachingSpeechAdapter} from './teaching-evidence-http.js';
const fail=reason=>{throw Error(reason);};
const shutdown=new AbortController();
// The Rust owner signals the whole owned process group. Keep Node alive long
// enough for ffmpeg's close/finally cleanup; abort outstanding owner HTTP reads.
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){
 process.on('SIGTERM',()=>{shutdown.abort();process.exitCode=1;});
 process.on('SIGINT',()=>{shutdown.abort();process.exitCode=1;});
}
const ownerRequests=new Set();
shutdown.signal.addEventListener('abort',()=>{for(const controller of ownerRequests)controller.abort();ownerRequests.clear();},{once:true});
async function ownerFetch(url,options={}){
 const controller=new AbortController();ownerRequests.add(controller);
 const abort=()=>{controller.abort();ownerRequests.delete(controller);};
 if(shutdown.signal.aborted||options.signal?.aborted)abort();
 options.signal?.addEventListener('abort',abort,{once:true});
 // Keep the controller connected while the existing bounded owner adapter reads
 // the response body, rather than detaching cancellation after response headers.
 const cleanup=setTimeout(()=>{ownerRequests.delete(controller);options.signal?.removeEventListener('abort',abort);},35000);cleanup.unref();
 try{return await fetch(url,{...options,signal:controller.signal});}
 catch(error){clearTimeout(cleanup);ownerRequests.delete(controller);options.signal?.removeEventListener('abort',abort);throw error;}
}
/** Only private receiver configuration/ledger paths; no dashboard-supplied paths. */
export async function readPrivateJson(path,strictMode=true) {
 if(!isAbsolute(path??''))fail('siteguides_configuration_path_required');
 const stat=await lstat(path);
 if(!stat.isFile()||stat.isSymbolicLink()||stat.size>1048576||(process.getuid&&stat.uid!==process.getuid())||(strictMode&&process.platform!=='win32'&&(stat.mode&0o077)))fail('siteguides_private_file_required');
 return JSON.parse(await readFile(path,'utf8'));
}
export async function readTeachingReceipt({ledgerDirectory,actor},request,purpose) {
 const ref=purpose==='teaching_review'?request.decisionRef:request.answerRef;
 if(!/^[A-Za-z0-9_-]{1,128}$/.test(ref??''))fail('teaching_receipt_reference_invalid');
 const value=await readPrivateJson(join(ledgerDirectory,ref+'.json'),false);
 const saved=value.action?.request;
 if(value.version!=='codex-operator-confirmation.v1'||value.status!=='confirmed'||value.confirmationId!==ref||value.authenticatedUser?.username!==actor||saved?.purpose!==purpose)fail('teaching_receipt_authority_mismatch');
 const keys=purpose==='teaching_review'?['taskId','rootGuideId','reviewedGuideId','reviewedSha256','contentSha256']:['taskId','rootGuideId','answeredGuideId','answeredSha256','contentSha256','questionId','questionSha256'];
 if(keys.some(key=>typeof request[key]!=='string'||saved[key]!==request[key]))fail('teaching_receipt_content_mismatch');
 return {...saved,...(purpose==='teaching_review'?{decisionId:ref}:{answerRef:ref}),actorRef:'dashboard:'+actor,sourceRef:ref,provenance:'authenticated_operator'};
}
/** Existing scoped recording reads plus immutable original agent provenance.
 * Accepted knowledge and auth freshness remain independent from replayability. */
export function createTeachingSourceStatus({evidence,readOperation,resolveSelection}) {
 return async request=>{
  const scope={recordingId:request.recordingId,videoSha256:request.videoSha256,sourceTaskId:request.sourceTaskId};
  if(!request.rootGuideId)return {...scope,state:'unchecked',reason:'source_not_checked'};
  try{
   const original=await readOperation({taskId:request.sourceTaskId,rootGuideId:request.rootGuideId,operationId:request.rootGuideId});
   if(original.status!=='resolved'||!original.mediaSha256||!original.response?.guide?.teaching||!original.references?.sourceKey)return {...scope,state:'unchecked',reason:'source_not_checked'};
   const selection=await resolveSelection({taskId:request.sourceTaskId,sourceKey:original.references.sourceKey});
   const current=await evidence.inspect(selection);
   if(current.recordingId!==scope.recordingId||current.video.sha256!==scope.videoSha256||current.taskId!==scope.sourceTaskId)return {...scope,state:'unavailable',reason:'source_binding_changed'};
   await evidence.readVideoSlice(current,{offset:0,length:1});
   const proofs=original.response.guide.teaching.evidence;
   return {...scope,state:'available',gaps:current.gaps,evidence:request.evidence.map(reference=>({id:reference.id,artifact:reference.artifact,sha256:reference.sha256,state:proofs.some(proof=>['id','kind','artifact','startMs','endMs','sha256'].every(key=>proof[key]===reference[key]))?'available':'unchecked',reason:'scoped_read'}))};
  }catch{return {...scope,state:'unavailable',reason:'source_unavailable'};}
 };
}

/** Server passes trusted task allowlist independently from the dashboard request. */
export async function runInstalledSiteGuides({configPath,browserExecutable,actor,ledgerDirectory,allowedTasks,knownSitePolicies,request}) {
 if(Number(process.versions.node.split('.')[0])<18)fail('siteguides_node_18_required');
 const config=await readPrivateJson(configPath);
 if(config.schemaVersion!=='agent-browser.workspace-siteguides-host.v1'||!config.candidateDirectory)fail('siteguides_configuration_required');
 const secret=async path=>{const value=await readPrivateJson(path);if(typeof value.token!=='string'||!value.token)fail('siteguides_token_unavailable');return value.token;};
 const callTool=async(name,args)=>{
  if(!isAbsolute(browserExecutable??''))fail('siteguides_existing_mcp_transport_required');
  if(name!=='browser_command'||args.action!=='playbook'||!['inspect_guide','validate_teaching_candidate'].includes(args.params?.operation))fail('siteguides_mcp_operation_denied');
  const input=JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name,arguments:args}})+'\n';
  const child=spawnSync(browserExecutable,['mcp','serve'],{input,encoding:'utf8',timeout:30000,maxBuffer:1048576,env:{...process.env,AGENT_BROWSER_SITEGUIDE_DIRECTORY:config.candidateDirectory},stdio:['pipe','pipe','pipe']});
  if(child.error||child.status!==0)fail('siteguides_existing_mcp_unavailable');
  const messages=child.stdout.split('\n').filter(Boolean).map(line=>JSON.parse(line));
  const response=messages.find(value=>value.id===1);if(response?.error||!response?.result)fail('siteguides_existing_mcp_rejected');return response.result;
 };
 const sources=config.sources??{};
 const source=async({taskId,sourceKey})=>{
  const selected=sources[sourceKey];
  if(!selected||selected.selection?.expected?.taskId!==taskId||!allowedTasks[taskId]?.use)fail('teaching_source_context_mismatch');
  return selected;
 };
 // A recording-access grant stays server-side and is already single-record scoped.
 let activeSource;
 const evidence=createTeachingEvidenceClient({
  recordings:async input=>{
   if(!activeSource||input.recordingId!==activeSource.selection.recordingId)fail('teaching_source_context_mismatch');
   const reader=createTeachingRecordingReader({url:activeSource.url,accessToken:await secret(activeSource.tokenFile),fetch:ownerFetch});return reader(input);
  },
  speech:new Proxy({}, {get:(_,method)=>async(...args)=>{
   if(!config.speech)fail('teaching_speech_configuration_unavailable');
   const adapter=createTeachingSpeechAdapter({...config.speech,token:await secret(config.speech.tokenFile),fetch:ownerFetch});return adapter[method](...args);
  }})
 });
 const receipt={ledgerDirectory,actor};
 let originalContext;
 if(request.rootGuideId&&request.taskId){const original=await createTeachingAgentSubmissionOwner({candidateDirectory:config.candidateDirectory}).read({taskId:request.taskId,rootGuideId:request.rootGuideId,operationId:request.rootGuideId});originalContext=original.context;}
 let context;
 if(['begin_agent','prepare_agent_media','stage_agent_result','stage_draft'].includes(request.operation)) {
  const task=allowedTasks[request.taskId];
  const sitePolicyId=request.sitePolicyId??originalContext?.sitePolicyId;
  const site=knownSitePolicies?.find(value=>value.id===sitePolicyId);
  if(!task?.use||!site?.allowedOrigins?.length)fail('teaching_known_site_origins_required');
  const account=request.account??originalContext?.account,organization=request.organization??originalContext?.organization;
  if(typeof account!=='string'||typeof organization!=='string')fail('teaching_intended_identity_choice_required');
  activeSource=await source(request);
  context={taskId:request.taskId,profileId:task.profileId,profileKind:originalContext?.profileKind??task.profileKind,sitePolicyId,account,organization,allowedOrigins:site.allowedOrigins};
  if(activeSource.context&&['taskId','profileId','sitePolicyId','account','organization'].some(key=>activeSource.context[key]!==context[key]))fail('teaching_source_context_mismatch');
 }

 const catalogTaskId=Object.keys(allowedTasks).find(taskId=>allowedTasks[taskId]?.read);
 const submissions=createTeachingAgentSubmissionOwner({candidateDirectory:config.candidateDirectory});
 const sourceStatus=createTeachingSourceStatus({evidence,readOperation:input=>submissions.read(input),resolveSelection:async input=>{activeSource=await source(input);return activeSource.selection;}});
 const owner=createWorkspaceSiteGuides({sourceStatus,isCurrentTask:async taskId=>allowedTasks[taskId]?.currentBrowser===true,catalogTaskId,context,candidateDirectory:config.candidateDirectory,browser:createPlaybookClient(callTool),evidence,
  listSources:async taskId=>Object.entries(sources).filter(([,value])=>value.selection?.expected?.taskId===taskId).map(([sourceKey,value])=>({sourceKey,recordingId:value.selection.recordingId,narrationSourceId:value.selection.narrationSourceId,state:'not_checked'})),
  authorizeTask:async(taskId,mode)=>allowedTasks[taskId]?.[mode]===true,
  resolveSelection:async input=>{activeSource=await source(input);return activeSource.selection;},
  readOperatorDecision:input=>readTeachingReceipt(receipt,input,'teaching_review'),
  readOperatorAnswer:input=>readTeachingReceipt(receipt,input,'teaching_answer'),
  // The receiving application verifier supplies immutable proof. Missing binding
  // deliberately remains unavailable; caller passed/actor fields are irrelevant.
  readFreshRepeat:async input=>{const path=config.freshRepeatReceipts?.[input.validationRef];if(!path)fail('teaching_repeat_verifier_unavailable');return readPrivateJson(path);},
 });
 if(request.operation==='prepare_agent_media') {
  const {loadTeachingMediaPacket}=await import('./teaching-media.js');
  const observedSource=await evidence.inspect(activeSource.selection);
  const bundle={...observedSource,speechSource:request.bundle?.speechSource??request.speechSource};
  const transcripts=[];
  for(const segment of request.segments??[]) {
   const observed=await evidence.readTranscript(bundle,segment);
   if(observed.status!=='available')fail('teaching_transcript_unavailable');
   transcripts.push(observed);
  }
  const packet=await loadTeachingMediaPacket({evidence,bundle,transcripts,context,operationId:request.operationId});
  if(request.kind==='answer'){const trusted=await owner.dispatch({...request,operation:'agent_revision_context'});const guide=structuredClone(trusted.guide);if(guide.teaching.evidence.some(item=>/^(?:file:|\/|[A-Za-z]:[\\/])/.test(item.artifact??'')))fail('teaching_revision_private_path_refused');for(const key of ['lineage','review','validation','clarification'])delete guide.teaching[key];packet.revision={guide,question:trusted.question,authenticatedAnswer:{...trusted.binding,answer:trusted.authenticatedAnswer.answer}};}
  if(Buffer.byteLength(JSON.stringify(packet))>900*1024)fail('teaching_media_packet_limit');
  return packet;
 }

 return owner.dispatch(request);
}
async function main(){
 const deadline=setTimeout(()=>shutdown.abort(),150000);deadline.unref();
 try{let input='';for await(const chunk of process.stdin){input+=chunk;if(input.length>1048576)fail('siteguides_request_too_large');}const data=await runInstalledSiteGuides(JSON.parse(input));process.stdout.write(JSON.stringify({success:true,data}));}
 catch(error){const reason=/^(teaching_|siteguides_)/.test(error.message??'')?error.message:'siteguides_owner_unavailable';process.stdout.write(JSON.stringify({success:false,error:reason}));process.exitCode=1;}finally{clearTimeout(deadline);}
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)await main();
