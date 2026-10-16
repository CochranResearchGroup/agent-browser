import assert from 'node:assert/strict';
import {createLoginPlaybook} from '../packages/client/src/login-playbook.js';

// Public producer version2 contract: caller work and receiving work differ.
const callerTaskId='1961b982-3bd1-4472-8539-a64be683bff0';
const binding={taskId:'receiving-task',profileId:'owned-profile',browserId:'owned-browser',tabId:'owned-tab'};
const receivingTarget={provider:'agent-browser',...binding};
const scope={person:'operator',tenant:'owned',account:'fixture',origin:'https://fixture.invalid',source:'dummy-ref',capabilities:['login']};
const expiresAt=Date.now()+60000;
const request={contractVersion:2,requestId:'hosted-request',taskId:callerTaskId,receivingTarget,state:'pending',scope:structuredClone(scope),expiresAt};
let stored=null,requested,submitted=0,verified=false;
const checkpoints={load:async()=>structuredClone(stored),save:async(_key,value)=>{stored=structuredClone(value);}};
const browser={
 verifyLogin:async()=>({...binding,status:verified?'completed':'waiting',reasonCode:verified?'authenticated':'password_required'}),
 prepareLogin:async()=>({plan:{guideDigest:'guide',controlFingerprint:'controls'}}),
 validatePlan:async()=>({...binding,status:'completed',reasonCode:'plan_current'}),
};
const helper=async(operation,args)=>{
 if(operation==='request_login'){requested=structuredClone(args);return structuredClone(request);}
 if(operation==='inspect_request')return structuredClone(request);
 if(operation==='begin_authentication'){submitted++;request.state='authenticated';request.captureState='ready';verified=true;return structuredClone(request);}
 throw Error('Unexpected hosted operation');
};
const options={taskId:binding.taskId,guideId:'fixture',accountRef:'fixture',organizationRef:'company',scope,expiresAt,callerTaskId};
const playbook=createLoginPlaybook({browser,helper,checkpoints});
assert.equal((await playbook.advance(options)).reasonCode,'approval_required');
assert.equal(requested.contractVersion,2);assert.deepEqual(requested.receivingTarget,receivingTarget);
assert.notEqual(stored.callerTaskId,binding.taskId);assert.equal(stored.callerTaskId,callerTaskId);
request.state='approved';
const result=await playbook.advance(options);
assert.equal(result.status,'completed');assert.equal(result.effectState,'verified');assert.equal(result.taskId,binding.taskId);assert.equal(result.requestId,request.requestId);assert.equal(submitted,1);
console.log('Hosted version2 caller/receiving identity and independent receiving verification passed');

for(const field of ['taskId','profileId','browserId','tabId']) {
 const saved=request.receivingTarget[field];request.receivingTarget[field]='foreign-or-stale';
 await assert.rejects(playbook.advance(options),/helper_binding_mismatch/);
 assert.equal(submitted,1,'Mismatched receiving must refuse before another submission');request.receivingTarget[field]=saved;
}
const originalCaller=request.taskId;request.taskId='another-caller';
await assert.rejects(playbook.advance(options),/helper_binding_mismatch/);request.taskId=originalCaller;
const originalAccount=request.scope.account;request.scope.account='wrong-account';
await assert.rejects(playbook.advance(options),/helper_binding_mismatch/);request.scope.account=originalAccount;
request.state='uncertain';request.captureState='ready';
assert.equal((await playbook.advance(options)).effectState,'no_effect');
assert.equal((await playbook.advance(options)).reasonCode,'password_source_unavailable');
request.captureState='held';
assert.equal((await playbook.advance(options)).reasonCode,'helper_running_or_uncertain');
assert.equal(submitted,1);

let interruptedStore=null,starts=0;
request.state='approved';request.captureState='ready';verified=false;
const interrupted=createLoginPlaybook({browser,checkpoints:{load:async()=>structuredClone(interruptedStore),save:async(_k,v)=>{interruptedStore=structuredClone(v);}},helper:async operation=>{
 if(operation==='begin_authentication'){starts++;request.startedAt=Date.now();request.inputAttemptedAt=Date.now();throw Error('Connection ended before start acknowledgement');}
 return structuredClone(request);
}});
assert.equal((await interrupted.advance(options)).effectState,'unknown');
assert.equal((await interrupted.advance(options)).reasonCode,'helper_running_or_uncertain');assert.equal(starts,1);
request.state='authenticated';request.captureState='ready';
const wrongCompany=createLoginPlaybook({browser:{...browser,verifyLogin:async()=>({...binding,status:'failed',reasonCode:'organization_mismatch'})},checkpoints:{load:async()=>structuredClone(interruptedStore),save:async()=>{throw Error('No rebinding');}},helper:async()=>structuredClone(request)});
assert.equal((await wrongCompany.advance(options)).status,'failed','Helper success cannot replace independent company proof');
assert.equal(starts,1);
console.log('Hosted wrong receiving/caller/account, no-effect, private hold, lost-ack no-replay and wrong-company checks passed');

request.state='uncertain';request.captureState='held';request.running=false;verified=false;const recoveryCalls=[];
const recovery=createLoginPlaybook({browser,checkpoints,helper:async operation=>{
 recoveryCalls.push(operation);if(operation==='inspect_request')return structuredClone(request);
 assert.equal(operation,'reconcile_authentication','Readback cannot start/repeat private authentication');request.state='authenticated';request.captureState='ready';verified=true;return structuredClone(request);
}});
const recovered=await recovery.advance({...options,reconcile:true});assert.equal(recovered.status,'completed');assert.equal(recovered.requestId,request.requestId);assert.deepEqual(recoveryCalls,['inspect_request','reconcile_authentication']);assert.equal(submitted,1);
const emptyRecovery=createLoginPlaybook({browser,checkpoints:{load:async()=>null,save:async()=>assert.fail('No replacement checkpoint')},helper:async()=>assert.fail('No replacement request')});
await assert.rejects(emptyRecovery.advance({...options,reconcile:true}),/checkpoint_required_for_reconciliation/);
request.state='uncertain';request.captureState='held';request.running=true;recoveryCalls.length=0;
assert.equal((await recovery.advance({...options,reconcile:true})).reasonCode,'helper_running_or_uncertain');assert.deepEqual(recoveryCalls,['inspect_request']);delete request.running;
console.log('Hosted playbook: explicit same-checkpoint readback cannot create a request or restart running/private input');
