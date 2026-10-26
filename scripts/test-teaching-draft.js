import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile,mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {writeTeachingDraftCandidate} from '../packages/client/src/teaching-draft.js';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
const fixture=new URL('./fixtures/teaching-report/',import.meta.url);
const guide=JSON.parse(await readFile(new URL('draft.json',fixture),'utf8'));
const video=await readFile(new URL('demonstration.webm',fixture));
assert.equal(createHash('sha256').update(video).digest('hex'),guide.teaching.videoSha256);
for(const evidence of guide.teaching.evidence) {
 const bytes=await readFile(new URL(evidence.artifact,fixture));
 assert.equal(createHash('sha256').update(bytes).digest('hex'),evidence.sha256);
}
const directory=await mkdtemp(join(tmpdir(),'teaching-draft-client-'));
try {
 await assert.rejects(writeTeachingDraftCandidate({candidateDirectory:directory,guide,candidateOnly:false}),/candidate/);
 const written=await writeTeachingDraftCandidate({candidateDirectory:directory,guide,candidateOnly:true});
 assert.equal(written.executionAllowed,false);
 await assert.rejects(writeTeachingDraftCandidate({candidateDirectory:directory,guide,candidateOnly:true}),/exists/);
 const calls=[];
 const browser=createPlaybookClient(async(name,args)=>{
  calls.push({name,args});
  const value=JSON.parse(await readFile(join(directory,args.params.guideId+'.json'),'utf8'));
  return {content:[{type:'text',text:JSON.stringify({success:true,data:value})}]};
 });
 const inspected=await browser.inspectTeachingDraft('task','daily-sales-draft-v1');
 assert.equal(inspected.teaching.state,'draft');
 assert.deepEqual(inspected.teaching.parameters.map(item=>[item.id,item.example]),[['startDate','2026-05-01'],['endDate','2026-05-31'],['includeRefunds',true]]);
 assert.deepEqual(inspected.teaching.playbook.routineIds,['choose-report','set-report-period','generate-and-check','export-and-verify']);
 assert.equal(inspected.teaching.claims.find(item=>item.id==='refunds-shown').attribution,'observed');
 assert.equal(inspected.teaching.claims.find(item=>item.id==='refunds-default').attribution,'inferred');
 assert.equal(inspected.teaching.claims.find(item=>item.id==='context-unverified').attribution,'unknown');
 assert.equal(inspected.teaching.unresolved.some(item=>item.question.includes('Should future runs always include refunds')),true);
 assert.equal(inspected.teaching.unresolved.some(item=>/what.*(?:start|end) date/i.test(item.question)),false,'known example dates are extracted, not asked again');
 assert.match(inspected.teaching.routines[3].outcomes[0].verification,/2026-05-01\/125.00\/5/);
 assert.match(inspected.teaching.routines[3].recovery[0].next,/no blind duplicate export/);
 assert.deepEqual(calls,[{name:'browser_command',args:{taskId:'task',action:'playbook',params:{operation:'inspect_guide',guideId:'daily-sales-draft-v1'}}}]);
 console.log('Teaching draft public client: exact media lineage, substantive draft, attribution, no overwrite or execution pass');
} finally {await rm(directory,{recursive:true,force:true});}
