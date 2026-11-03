import assert from 'node:assert/strict';import{mkdtemp,rm}from'node:fs/promises';import{join}from'node:path';import{tmpdir}from'node:os';
import{createTeachingAgentSubmissionOwner}from'../packages/client/src/teaching-draft.js';
const directory=await mkdtemp(join(tmpdir(),'teaching-agent-'));
try{
 const owner=createTeachingAgentSubmissionOwner({candidateDirectory:directory});const scope={taskId:'task',rootGuideId:'teaching-original',operationId:'teaching-original',inputSha256:'a'.repeat(64),context:{taskId:'task',profileId:'disposable-fixture',profileKind:'disposable',sitePolicyId:'site',account:'',organization:'',allowedOrigins:['https://synthetic.invalid']}};
 const results=await Promise.all([owner.begin(scope),owner.begin(scope)]);assert.equal(results.filter(item=>item.status==='created').length,1);assert.equal(results.filter(item=>item.status==='pending').length,1);
 assert.equal((await owner.retainedContexts())[0].context.profileId,'disposable-fixture');
 assert.equal((await owner.begin(scope)).retryAuthority,'not_inferred');
 await assert.rejects(owner.begin({...scope,inputSha256:'b'.repeat(64)}),/intent_changed/);
 await owner.bindMedia({...scope,mediaSha256:'c'.repeat(64)});
 await assert.rejects(owner.bindMedia({...scope,mediaSha256:'d'.repeat(64)}),/media_changed/);
 const response={guide:{guideId:'teaching-original'},questions:[]};await owner.recordResult({...scope,response});
 const reopened=createTeachingAgentSubmissionOwner({candidateDirectory:directory});const read=await reopened.read(scope);assert.equal(read.status,'resolved');assert.deepEqual(read.response,response);assert.equal(read.retryAuthority,'not_inferred');
 await assert.rejects(owner.recordResult({...scope,response:{...response,questions:[{}]}}),/result_changed/);
 assert.deepEqual(await owner.read({...scope,operationId:'missing',rootGuideId:'missing'}),{status:'not_observed',retryAuthority:'not_inferred'});
 console.log('Teaching draft owner: immutable original intent/result, concurrent admission, exact digest, fresh-process readback and no model retry authority.');
}finally{await rm(directory,{recursive:true,force:true});}
