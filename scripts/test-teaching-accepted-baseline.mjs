import assert from 'node:assert/strict';import{mkdtemp,readFile,writeFile,rm}from'node:fs/promises';import{tmpdir}from'node:os';import{join}from'node:path';import{createHash}from'node:crypto';
import{resolveTeachingAcceptedBaseline,createTeachingReviewClient}from'../packages/client/src/teaching-review.js';import{writeTeachingDraftCandidate}from'../packages/client/src/teaching-draft.js';
const hash=value=>createHash('sha256').update(value).digest('hex'),directory=await mkdtemp(join(tmpdir(),'teaching-baseline-'));
try{
 const prior={schemaVersion:'agent-browser.siteguide.v1',guideId:'accepted-existing',name:'Existing accepted procedure',allowedOrigins:['https://teaching-report.example.invalid'],states:[]};const bytes=JSON.stringify(prior)+'\n';await writeFile(join(directory,prior.guideId+'.json'),bytes,{mode:0o600});
 const browser={inspectGuide:async(taskId,guideId)=>JSON.parse(await readFile(join(directory,guideId+'.json'),'utf8')),inspectTeachingDraft:async(taskId,guideId)=>JSON.parse(await readFile(join(directory,guideId+'.json'),'utf8')),validateTeachingCandidate:async({taskId,guide})=>({valid:true,guideId:guide.guideId,taskId,executionAllowed:false,operatorReviewQualified:false})};
 const baseline=await resolveTeachingAcceptedBaseline({candidateDirectory:directory,browser,taskId:'task',guideId:prior.guideId,allowedOrigins:prior.allowedOrigins});assert.deepEqual(baseline,{guideId:prior.guideId,sha256:hash(bytes)});
 await assert.rejects(resolveTeachingAcceptedBaseline({candidateDirectory:directory,browser,taskId:'task',guideId:prior.guideId,allowedOrigins:['https://peer.invalid']}),/scope_mismatch/);
 const guide=JSON.parse(await readFile(new URL('./fixtures/teaching-report/draft.json',import.meta.url),'utf8'));await writeTeachingDraftCandidate({candidateDirectory:directory,candidateOnly:true,guide});
 const owner=createTeachingReviewClient({candidateDirectory:directory,browser,acceptedVersion:baseline});const opened=await owner.open({taskId:'task',guideId:guide.guideId});assert.equal(opened.acceptedVersion.guideId,prior.guideId);assert.equal(opened.acceptedVersion.state,'retained');
 const saved=await owner.save({taskId:'task',guideId:guide.guideId,expectedSha256:opened.sha256,operationId:'edit-1',guide:{...guide,name:'Replacement candidate'}});assert.equal(saved.status,'saved');
 await assert.rejects(owner.promote({taskId:'task',guideId:saved.guideId,expectedSha256:saved.sha256,operationId:'bad-promote'}),/repeat_authority_unconfigured/);
 assert.equal((await owner.open({taskId:'task',guideId:saved.guideId})).acceptedVersion.guideId,prior.guideId);assert.equal(await readFile(join(directory,prior.guideId+'.json'),'utf8'),bytes);
 console.log('Legacy accepted guide: existing-owner scope+raw digest resolved, retained through incomplete replacement without modifying original bytes.');
}finally{await rm(directory,{recursive:true,force:true});}
