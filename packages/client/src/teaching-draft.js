import {lstat,open,link,unlink} from 'node:fs/promises';
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
