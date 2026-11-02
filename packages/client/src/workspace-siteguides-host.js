// Source-free installed composition host. All imports are packaged support assets.
import {readFile,lstat} from 'node:fs/promises';
import {join,isAbsolute} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createWorkspaceSiteGuides} from './workspace-siteguides.js';
import {createPlaybookClient} from './playbooks.js';
import {createTeachingEvidenceClient} from './teaching-evidence.js';
import {createTeachingRecordingReader,createTeachingSpeechAdapter} from './teaching-evidence-http.js';
const fail=reason=>{throw Error(reason);};
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
/** Server passes trusted task allowlist independently from the dashboard request. */
export async function runInstalledSiteGuides({configPath,actor,ledgerDirectory,allowedTasks,request}) {
 if(Number(process.versions.node.split('.')[0])<18)fail('siteguides_node_18_required');
 const config=await readPrivateJson(configPath);
 if(config.schemaVersion!=='agent-browser.workspace-siteguides-host.v1'||!config.candidateDirectory)fail('siteguides_configuration_required');
 const secret=async path=>{const value=await readPrivateJson(path);if(typeof value.token!=='string'||!value.token)fail('siteguides_token_unavailable');return value.token;};
 const mcp=config.mcp;
 const callTool=async(name,args)=>{
  if(!mcp?.url||!mcp.tokenFile)fail('siteguides_existing_mcp_transport_required');
  const url=new URL(mcp.url);if(url.protocol!=='https:'&&!(url.protocol==='http:'&&['127.0.0.1','localhost','[::1]'].includes(url.hostname)))fail('siteguides_transport_origin_invalid');
  const response=await fetch(url,{method:'POST',redirect:'error',headers:{'Content-Type':'application/json','Authorization':'Bearer '+await secret(mcp.tokenFile)},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',params:{name,arguments:args}}),signal:AbortSignal.timeout(30000)});
  if(!response.ok)fail('siteguides_existing_mcp_unavailable');
  const bytes=await response.text();if(bytes.length>1048576)fail('siteguides_response_too_large');const value=JSON.parse(bytes);if(value.error||!value.result)fail('siteguides_existing_mcp_rejected');return value.result;
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
   const reader=createTeachingRecordingReader({url:activeSource.url,accessToken:await secret(activeSource.tokenFile)});return reader(input);
  },
  speech:new Proxy({}, {get:(_,method)=>async(...args)=>{
   if(!config.speech)fail('teaching_speech_configuration_unavailable');
   const adapter=createTeachingSpeechAdapter({...config.speech,token:await secret(config.speech.tokenFile)});return adapter[method](...args);
  }})
 });
 const receipt={ledgerDirectory,actor};
 const owner=createWorkspaceSiteGuides({candidateDirectory:config.candidateDirectory,browser:createPlaybookClient(callTool),evidence,
  listSources:async taskId=>Object.entries(sources).filter(([,value])=>value.selection?.expected?.taskId===taskId).map(([sourceKey,value])=>({sourceKey,recordingId:value.selection.recordingId,narrationSourceId:value.selection.narrationSourceId,state:'not_checked'})),
  authorizeTask:async(taskId,mode)=>allowedTasks[taskId]?.[mode]===true,
  resolveSelection:async input=>{activeSource=await source(input);return activeSource.selection;},
  readOperatorDecision:input=>readTeachingReceipt(receipt,input,'teaching_review'),
  readOperatorAnswer:input=>readTeachingReceipt(receipt,input,'teaching_answer'),
  // The receiving application verifier supplies immutable proof. Missing binding
  // deliberately remains unavailable; caller passed/actor fields are irrelevant.
  readFreshRepeat:async input=>{const path=config.freshRepeatReceipts?.[input.validationRef];if(!path)fail('teaching_repeat_verifier_unavailable');return readPrivateJson(path);},
 });
 return owner.dispatch(request);
}
async function main(){
 try{let input='';for await(const chunk of process.stdin){input+=chunk;if(input.length>1048576)fail('siteguides_request_too_large');}const data=await runInstalledSiteGuides(JSON.parse(input));process.stdout.write(JSON.stringify({success:true,data}));}
 catch(error){process.stdout.write(JSON.stringify({success:false,error:error.message}));process.exitCode=1;}
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href)await main();
