import assert from 'node:assert/strict';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {createHash} from 'node:crypto';
import {writeTeachingDraftCandidate} from '../packages/client/src/teaching-draft.js';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
import {createWorkspaceSiteGuides} from '../packages/client/src/workspace-siteguides.js';
const hash=value=>createHash('sha256').update(value).digest('hex');
const directory=await mkdtemp(join(tmpdir(),'p300-siteguides-'));
const guide=JSON.parse(await readFile(new URL('./fixtures/teaching-report/draft.json',import.meta.url),'utf8'));
const browser=createPlaybookClient(async(name,args)=>({content:[{type:'text',text:JSON.stringify({success:true,data:args.params.operation==='validate_teaching_candidate'?{valid:true,guideId:args.params.guide.guideId,taskId:args.taskId,executionAllowed:false,operatorReviewQualified:false}:JSON.parse(await readFile(join(directory,args.params.guideId+'.json'),'utf8'))})}]}));
try {
 await writeTeachingDraftCandidate({candidateDirectory:directory,candidateOnly:true,guide});
 const owner=createWorkspaceSiteGuides({candidateDirectory:directory,browser});
 const list=await owner.dispatch({operation:'list'});
 assert.equal(list.guides[0].name,'Daily sales teaching candidate');
 assert.equal(list.guides[0].purpose,'Produce an independently verified Daily sales CSV for caller-selected dates.');
 assert.equal(list.guides[0].review,'not_reviewed');assert.equal(list.guides[0].repeat,'not_validated');
 assert.equal(list.guides[0].ordinaryReuseAllowed,false);
 const opened=await owner.dispatch({operation:'open',taskId:'task',guideId:guide.guideId});
 assert.equal(opened.view.steps.length,guide.teaching.routines.reduce((total,routine)=>total+routine.steps.length,0));
 assert.equal(opened.view.source.replayable,false);
 assert.equal(opened.history.length,1);assert.equal(opened.history[0].sha256,hash(await readFile(join(directory,guide.guideId+'.json'))));
 console.log('Workspace SiteGuides public catalog/open keeps purpose, progress, source uncertainty and immutable history.');
} finally {await rm(directory,{recursive:true,force:true});}
