import assert from 'node:assert/strict';
import {test} from 'node:test';
import {createLoginContinuation} from '../packages/client/src/login-continuation.js';
import {createLoginVerifier} from '../packages/client/src/login-verification.js';

const binding={taskId:'browser-task',profileId:'profile-1',browserId:'browser-1',tabId:'tab-1',targetId:'target-1',origin:'https://accounts.example.test'};
function fixture() {
  let record={schemaVersion:1,episodeId:'episode-1',taskId:binding.taskId,inputs:[{requestId:'native-input-1',state:'unknown'}],loginContinuation:{schemaVersion:1,workflowId:'workflow-1',requestId:'request-1',attemptId:'attempt-1',binding,requestedOutcome:{applicationOrigin:'https://app.example.test',expectedAccount:'intended@example.test'},unresolvedAttempts:[]}};
  const request={requestId:'request-1',attemptId:'attempt-1',clientId:'client-1',taskId:'caller-1',targetId:binding.taskId,origin:binding.origin,operation:'login',mode:'private',state:'unknown'};
  let proof={...binding,state:'unverified',accountMatched:false,applicationOrigin:'https://app.example.test',evidenceRef:'app-proof-1'};
  const completion={requestId:'request-1',state:'completed',completion:{outcome:'succeeded',evidenceAuthority:'caller_observed',actor:{clientId:'client-1',taskId:'caller-1'},returnId:'return-1',observation:{applicationOrigin:'https://app.example.test',accountEvidence:'account-proof-1',evidenceRefs:['app-proof-1']}},receivingSettlement:{status:'settled',privacy:'retained'}};
  const operations=[],updates=[];
  const client={async call(operation,args) {
    operations.push(operation);assert.equal(args.requestId,'request-1');
    if(operation==='inspect_credential_request')return structuredClone(request);
    if(operation==='inspect_authentication_completion')return structuredClone(completion);
    throw Error('Unexpected credential effect: '+operation);
  }};
  const store={async withEpisode(_id,run){return run();},async load(){return structuredClone(record);},async save(value){record=structuredClone(value);}};
  const verifier=createLoginVerifier({client,callerTaskId:'caller-1',verifyAccess:async()=>structuredClone(proof)});
  const options={store,callerTaskId:'caller-1',inspectRequest:args=>client.call('inspect_credential_request',args),verify:input=>verifier.verify(input),onUpdate:async update=>{updates.push(update);}};
  return {request,completion,operations,updates,options,get record(){return record;},set record(value){record=value;},get proof(){return proof;},set proof(value){proof=value;}};
}

test('resume preserves original unknown input and coalesces unchanged observations across processes',async()=>{
  const f=fixture();
  const first=await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  const reopened=await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  assert.equal(first.category,'input_outcome_unknown');
  assert.equal(reopened.outcome,'incomplete');
  assert.equal(f.updates.length,1);
  assert.deepEqual(f.record.inputs,[{requestId:'native-input-1',state:'unknown'}]);
  assert.deepEqual(f.record.loginContinuation.unresolvedAttempts,[{requestId:'request-1',attemptId:'attempt-1'}]);
  assert(f.operations.every(operation=>operation.startsWith('inspect_')));
});

test('absence of a callback does not consume a future notification; callback failure stays unknown',async()=>{
  const f=fixture();
  await createLoginContinuation({...f.options,onUpdate:undefined}).resume({episodeId:'episode-1'});
  let calls=0;
  const options={...f.options,onUpdate:async()=>{calls++;throw Error('delivery outcome unknown');}};
  const first=await createLoginContinuation(options).resume({episodeId:'episode-1'});
  assert.equal(first.notification.state,'unknown');
  assert.equal(first.notification.emitted,false);
  const reopened=await createLoginContinuation(options).resume({episodeId:'episode-1'});
  assert.equal(calls,1);
  assert.equal(reopened.notification.state,'unknown');
});

test('source closure and forged stored success never replace fresh verification',async()=>{
  const f=fixture();f.request.state='completed';
  f.record={...f.record,trackerState:'closed',loginContinuation:{...f.record.loginContinuation,progress:{outcome:'verified'},state:'completed'}};
  const result=await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  assert.equal(result.outcome,'incomplete');
  const forged=await createLoginContinuation({...f.options,verify:async()=>({requestId:'request-1',outcome:'verified',category:'verified_access'})}).resume({episodeId:'episode-1'});
  assert.equal(forged.outcome,'incomplete');
});

test('resume retains the actual incomplete application-verifier evidence gap',async()=>{
  const f=fixture();f.request.state='completed';
  f.proof={...f.proof,state:'verified',accountMatched:true};
  f.completion.completion=null;
  const result=await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  assert.equal(result.category,'completion_unverified');
  assert.equal(result.component,'application');
  assert.equal(result.qualification.applicationAccess,'verified');
  assert.deepEqual(result.evidenceReferences,['app-proof-1']);
  assert.equal(result.outcome,'incomplete');
  const detailed=await createLoginContinuation({...f.options,verify:async input=>({...await f.options.verify(input),nextAction:{operation:'inspect_authentication_completion',args:{requestId:input.requestId}},privatePage:'PRIVATE'})}).resume({episodeId:'episode-1'});
  assert.deepEqual(detailed.nextAction,{operation:'inspect_authentication_completion',args:{requestId:'request-1'}});
  assert.equal(JSON.stringify(detailed).includes('PRIVATE'),false);
});

test('expired request stays expired even with a ready wake',async()=>{
  const f=fixture();f.request.state='expired';
  f.record={...f.record,loginContinuation:{...f.record.loginContinuation,wakeIntentId:'wake-1'}};
  const result=await createLoginContinuation({...f.options,verify:async()=>{throw Error('Expired request must not initiate verification');},readWake:async()=>({intentId:'wake-1',state:'registered',armed:true,monitor:{ready:true}})}).resume({episodeId:'episode-1'});
  assert.equal(result.category,'request_expired');
  assert.equal(result.outcome,'incomplete');
  assert.equal(result.waitingForUser,false);
  assert.equal(result.wake.state,'ready');
  assert.deepEqual(f.operations,['inspect_credential_request']);
});

test('missing or mismatched existing workflow refuses without changing the record',async()=>{
  const f=fixture();const before=structuredClone(f.record);f.request.attemptId='another-attempt';
  await assert.rejects(createLoginContinuation(f.options).resume({episodeId:'episode-1'}),{code:'login_continuation_unverified'});
  assert.deepEqual(f.record,before);
  f.record={episodeId:'episode-1',inputs:[{state:'unknown'}]};
  await assert.rejects(createLoginContinuation(f.options).resume({episodeId:'episode-1'}),{code:'login_continuation_unverified'});
  assert.deepEqual(f.record,{episodeId:'episode-1',inputs:[{state:'unknown'}]});
});

test('registered and held wakes never promise readiness; exact armed monitor receipt does',async()=>{
  const f=fixture();f.record={...f.record,loginContinuation:{...f.record.loginContinuation,wakeIntentId:'wake-1'}};
  let receipt={intentId:'wake-1',state:'registered',armed:false,monitor:{ready:true}};
  const options={...f.options,readWake:async()=>receipt};
  assert.equal((await createLoginContinuation(options).resume({episodeId:'episode-1'})).wake.state,'registered');
  receipt={...receipt,state:'held',armed:true};
  assert.equal((await createLoginContinuation(options).resume({episodeId:'episode-1'})).wake.state,'held');
  receipt={...receipt,state:'registered',armed:true};
  assert.equal((await createLoginContinuation(options).resume({episodeId:'episode-1'})).wake.state,'ready');
  receipt={...receipt,intentId:'other-wake'};
  assert.equal((await createLoginContinuation(options).resume({episodeId:'episode-1'})).wake.state,'unverified');
  receipt={...receipt,intentId:'wake-1',state:'cancelled'};
  assert.equal((await createLoginContinuation(options).resume({episodeId:'episode-1'})).wake.state,'cancelled');
});

test('interrupted original workflow reopens, reports changed action, then independently verified access',async()=>{
  const f=fixture();
  await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  f.request.state='pending';f.request.approvalSubmission='receiving_target_unavailable';
  const held=await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  assert.equal(held.category,'receiving_target_unavailable');
  f.request.state='completed';delete f.request.approvalSubmission;
  f.proof={...f.proof,state:'verified',accountMatched:true};
  const completed=await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  assert.equal(completed.outcome,'verified');
  assert.equal(f.updates.length,3);
  assert.deepEqual(completed.unresolvedAttempts,[{requestId:'request-1',attemptId:'attempt-1'}]);
  assert.deepEqual(f.record.inputs,[{requestId:'native-input-1',state:'unknown'}]);
  assert(f.operations.every(operation=>operation.startsWith('inspect_')));
});

test('changed independent evidence emits once; original Native and private obligation details remain private',async()=>{
  const f=fixture();f.request.state='completed';f.proof={...f.proof,state:'verified',accountMatched:true};
  f.record={...f.record,loginContinuation:{...f.record.loginContinuation,unresolvedAttempts:[{requestId:'old-request',attemptId:'old-attempt',privateDetail:'PRIVATE_RECORD'}]}};
  await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  f.proof={...f.proof,evidenceRef:'app-proof-2'};f.completion.completion.observation.evidenceRefs=['app-proof-2'];
  const updated=await createLoginContinuation(f.options).resume({episodeId:'episode-1'});
  assert.equal(f.updates.length,2);
  assert.equal(updated.outcome,'verified');
  assert.equal(updated.qualification.applicationAccess,'verified');
  assert.equal(JSON.stringify(updated).includes('PRIVATE_RECORD'),false);
  assert.equal(f.record.loginContinuation.unresolvedAttempts[0].privateDetail,'PRIVATE_RECORD');
});

test('verified access reports a remaining wake cancellation obligation independently',async()=>{
  const f=fixture();f.request.state='completed';f.proof={...f.proof,state:'verified',accountMatched:true};
  f.record={...f.record,loginContinuation:{...f.record.loginContinuation,wakeIntentId:'wake-1'}};
  let state='held';
  const options={...f.options,readWake:async()=>({intentId:'wake-1',state})};
  const held=await createLoginContinuation(options).resume({episodeId:'episode-1'});
  assert.equal(held.outcome,'verified');assert.equal(held.wakeCleanupRequired,true);
  state='cancelled';
  const cancelled=await createLoginContinuation(options).resume({episodeId:'episode-1'});
  assert.equal(cancelled.outcome,'verified');assert.equal(cancelled.wakeCleanupRequired,false);
});
