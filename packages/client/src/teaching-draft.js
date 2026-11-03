import {lstat,open,link,unlink,readFile,readdir} from 'node:fs/promises';
import {isAbsolute,join} from 'node:path';
import {randomUUID,createHash} from 'node:crypto';

/**
 * Stage a new inert SiteGuide in the caller's existing owned candidate directory.
 * The caller supplies agent-prepared knowledge; this helper never synthesizes a
 * website model, reads media, executes routines, promotes or overwrites a guide.
 * Full contract validation is the existing public inspect_guide operation.
 */
export async function writeTeachingDraftCandidate({candidateDirectory,candidateOnly,guide}) {
  if(candidateOnly!==true || typeof candidateDirectory!=='string' || !isAbsolute(candidateDirectory)) throw Error('teaching_candidate_intent_required');
  if(!guide || guide.schemaVersion!=='agent-browser.siteguide.v1' || !/^[a-z0-9][a-z0-9-]{0,119}$/.test(guide.guideId) ||
      guide.teaching?.schemaVersion!=='agent-browser.teaching-draft.v1' || guide.teaching.state!=='draft' ||
      ['lineage','review','validation'].some(key=>key in guide.teaching) ||
      !Array.isArray(guide.teaching.routines) || !guide.teaching.routines.length || !guide.teaching.playbook?.routineIds?.length) throw Error('teaching_candidate_invalid');
  const metadata=await lstat(candidateDirectory);
  if(!metadata.isDirectory() || metadata.isSymbolicLink() || process.getuid && metadata.uid!==process.getuid() || process.platform!=='win32' && (metadata.mode&0o077)) throw Error('teaching_candidate_directory_invalid');
  const bytes=Buffer.from(JSON.stringify(guide,null,2)+'\n');
  if(bytes.length>65536) throw Error('teaching_candidate_too_large');
  const temporary=join(candidateDirectory,`.teaching-${randomUUID()}.tmp`);
  const path=join(candidateDirectory,guide.guideId+'.json');
  let file;
  try {
    file=await open(temporary,'wx',0o600);await file.writeFile(bytes);await file.sync();await file.close();file=null;
    try {await link(temporary,path);} catch(error) {if(error.code==='EEXIST')throw Error('teaching_candidate_exists');throw error;}
    return {guideId:guide.guideId,path,sha256:createHash('sha256').update(bytes).digest('hex'),state:'draft',executionAllowed:false};
  } finally {await file?.close();await unlink(temporary).catch(error=>{if(error.code!=='ENOENT')throw error;});bytes.fill(0);}
}

/**
 * Immutable model-operation provenance inside the existing private teaching
 * directory. These sidecars are never guide heads, approvals or reuse authority.
 * An admitted operation cannot call a model again merely because its reply is
 * absent. The receiving adapter stores a resolved response before staging it.
 * @param {{candidateDirectory:string}} configuration
 */
export function createTeachingAgentSubmissionOwner({candidateDirectory}) {
 const digest=value=>createHash('sha256').update(JSON.stringify(value)).digest('hex');
 async function paths(scope) {
  if(!isAbsolute(candidateDirectory??'')||!['rootGuideId','operationId'].every(key=>typeof scope[key]==='string'&&/^[a-z0-9][a-z0-9-]{0,119}$/.test(scope[key]))||typeof scope.taskId!=='string'||!scope.taskId||scope.taskId.length>4000)throw Error('teaching_agent_scope_invalid');
  const stat=await lstat(candidateDirectory);
  if(!stat.isDirectory()||stat.isSymbolicLink()||process.getuid&&stat.uid!==process.getuid()||process.platform!=='win32'&&(stat.mode&0o077))throw Error('teaching_candidate_directory_invalid');
  const stem=join(candidateDirectory,'.teaching-agent-'+digest([scope.rootGuideId,scope.operationId]));
  return {intent:stem+'.intent',result:stem+'.result',media:stem+'.media'};
 }
 async function read(path) {
  try{const stat=await lstat(path);if(!stat.isFile()||stat.isSymbolicLink()||stat.size>131072)throw Error('teaching_agent_record_invalid');const bytes=await readFile(path);if(bytes.length>131072)throw Error('teaching_agent_record_invalid');return JSON.parse(bytes.toString('utf8'));}
  catch(error){if(error.code==='ENOENT')return null;throw error;}
 }
 async function publish(path,value) {
  const bytes=Buffer.from(JSON.stringify(value)+'\n');if(bytes.length>131072)throw Error('teaching_agent_result_too_large');
  const temporary=join(candidateDirectory,`.teaching-agent-${randomUUID()}.tmp`);let file;
  try{file=await open(temporary,'wx',0o600);await file.writeFile(bytes);await file.sync();await file.close();file=null;
   try{await link(temporary,path);}catch(error){if(error.code==='EEXIST')return false;throw error;}
   if(process.platform!=='win32'){const parent=await open(candidateDirectory,'r');try{await parent.sync();}finally{await parent.close();}}
   return true;
  }finally{await file?.close();await unlink(temporary).catch(error=>{if(error.code!=='ENOENT')throw error;});bytes.fill(0);}
 }
 function checked(record,scope) {
  if(!record||record.schemaVersion!=='agent-browser.teaching-agent-operation.v1'||['taskId','rootGuideId','operationId'].some(key=>record[key]!==scope[key])||scope.inputSha256&&record.inputSha256!==scope.inputSha256)throw Error('teaching_agent_intent_changed');
  return record;
 }
 async function readOriginal(scope) {
  const path=await paths(scope),intent=await read(path.intent);
  if(!intent)return {status:'not_observed',retryAuthority:'not_inferred'};
  checked(intent,scope);const result=await read(path.result);
  if(!result)return {...intent,status:'pending',retryAuthority:'not_inferred'};
  checked(result,intent);if(result.responseSha256!==digest(result.response))throw Error('teaching_agent_result_changed');
  if(result.mediaSha256){const media=checked(await read(path.media),scope);if(media.mediaSha256!==result.mediaSha256)throw Error('teaching_agent_media_changed');}
  else if(!['unavailable','unknown'].includes(result.response?.status))throw Error('teaching_agent_media_digest_required');
  return {...result,status:'resolved',retryAuthority:'not_inferred'};
 }
 return {
  async begin(scope) {
   if(!/^[a-f0-9]{64}$/.test(scope.inputSha256??''))throw Error('teaching_agent_input_digest_required');
   if(scope.context&&(scope.context.taskId!==scope.taskId||!['named','disposable'].includes(scope.context.profileKind)||['profileId','sitePolicyId'].some(key=>typeof scope.context[key]!=='string'||!scope.context[key]||scope.context[key].length>4000)||['account','organization'].some(key=>typeof scope.context[key]!=='string'||scope.context[key].length>4000)||!Array.isArray(scope.context.allowedOrigins)||!scope.context.allowedOrigins.length))throw Error('teaching_agent_context_invalid');
   if(scope.acceptedVersion&&(!/^[a-z0-9][a-z0-9-]{0,119}$/.test(scope.acceptedVersion.guideId??'')||!/^[a-f0-9]{64}$/.test(scope.acceptedVersion.sha256??'')))throw Error('teaching_accepted_version_invalid');
   const path=await paths(scope),value={schemaVersion:'agent-browser.teaching-agent-operation.v1',taskId:scope.taskId,rootGuideId:scope.rootGuideId,operationId:scope.operationId,inputSha256:scope.inputSha256,...(scope.acceptedVersion?{acceptedVersion:structuredClone(scope.acceptedVersion)}:{}),...(scope.context?{context:structuredClone(scope.context)}:{}),...(scope.references?{references:structuredClone(scope.references)}:{})};
   if(await publish(path.intent,value))return {...value,status:'created',retryAuthority:'not_inferred'};
   return readOriginal(scope);
  },
  async retainedContexts() {
   // Reuse the same private directory validation before enumerating provenance.
   await paths({taskId:'metadata',rootGuideId:'metadata',operationId:'metadata'});
   const names=(await readdir(candidateDirectory)).filter(name=>/^\.teaching-agent-[a-f0-9]{64}\.intent$/.test(name));if(names.length>4096)throw Error('teaching_agent_history_limit');
   const contexts=[];
   for(const name of names){const record=await read(join(candidateDirectory,name));if(record?.schemaVersion!=='agent-browser.teaching-agent-operation.v1'||record.rootGuideId!==record.operationId||!record.context)continue;checked(record,record);if(record.context.taskId!==record.taskId)throw Error('teaching_agent_context_invalid');contexts.push({taskId:record.taskId,rootGuideId:record.rootGuideId,context:record.context});}
   return contexts;
  },
  read:readOriginal,
  async bindMedia(scope) {
   if(!/^[a-f0-9]{64}$/.test(scope.mediaSha256??''))throw Error('teaching_agent_media_digest_required');
   const path=await paths(scope),intent=checked(await read(path.intent),scope),value={...intent,mediaSha256:scope.mediaSha256};
   if(!await publish(path.media,value)){const previous=checked(await read(path.media),scope);if(previous.mediaSha256!==scope.mediaSha256)throw Error('teaching_agent_media_changed');}
   return value;
  },
  async recordResult(scope) {
   const path=await paths(scope),intent=checked(await read(path.intent),scope);
   const response=structuredClone(scope.response);
   const media=await read(path.media);if(media)checked(media,scope);else if(!['unavailable','unknown'].includes(response?.status))throw Error('teaching_agent_media_digest_required');
   const value={...intent,mediaSha256:media?.mediaSha256??null,response,responseSha256:digest(response)};
   if(!await publish(path.result,value)){const prior=await readOriginal(scope);if(prior.responseSha256!==value.responseSha256)throw Error('teaching_agent_result_changed');return prior;}
   return readOriginal(scope);
  },
 };
}
