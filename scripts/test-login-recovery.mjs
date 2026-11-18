import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createLoginRecovery} from '../packages/client/src/login-recovery.js';

test('unknown original uses owner reconciliation and never dispatches credential input',async()=>{
  const operations=[];
  const original={requestId:'request-1',attemptId:'attempt-1',state:'unknown',running:false};
  const client={async call(operation,args){operations.push(operation);assert.equal(args.requestId,'request-1');return {...original};}};
  const recovery=createLoginRecovery({client});
  assert.equal((await recovery.reconcile({requestId:'request-1'})).category,'input_outcome_unknown');
  assert.equal((await recovery.reconcile({requestId:'request-1'})).category,'input_outcome_unknown');
  assert.deepEqual(operations,['inspect_credential_request','reconcile_credential_request','inspect_credential_request','reconcile_credential_request']);
});

test('changed attempt or running original cannot be settled or prepared again',async()=>{
  let calls=0;
  const running={async call(operation){calls++;assert.equal(operation,'inspect_credential_request');return {requestId:'r',attemptId:'a',state:'unknown',running:true};}};
  const recovery=createLoginRecovery({client:running});
  await assert.rejects(recovery.recordFailure({requestId:'r'}),{code:'login_recovery_unverified'});
  assert.equal(calls,1);
  const changed={async call(operation){return {requestId:'r',attemptId:operation==='inspect_credential_request'?'a':'b',state:'unknown',running:false};}};
  await assert.rejects(createLoginRecovery({client:changed}).reconcile({requestId:'r'}),{code:'login_recovery_unverified'});
});

test('fresh preparation is separately owner-authorized and never reuses old input',async()=>{
  const operations=[];
  const original={requestId:'request-1',attemptId:'attempt-1',state:'completed',running:false,origin:'https://identity.example',targetId:'target-1',recipientAccount:'configured@example.test'};
  const client={async call(operation,args){operations.push(operation);
    if(operation==='inspect_credential_request')return original;
    if(operation==='inspect_authentication_completion')return {requestId:'request-1',state:'completed',completion:{outcome:'failed'},receivingSettlement:{status:'settled'}};
    if(operation==='prepare_credential_login')return {loginId:'login-2',request:{requestId:'request-2',state:'approved',origin:args.origin,targetId:args.targetId}};
    throw Error('Unexpected private effect');
  }};
  const recovery=createLoginRecovery({client});
  const intent={idempotencyKey:'fresh-1',origin:original.origin,targetId:original.targetId,accountHint:original.recipientAccount,expiresAt:2000};
  const result=await recovery.prepareFresh({previousRequestId:'request-1',intent});
  assert.equal(result.requestId,'request-2');
  assert.equal(result.previousRequestId,'request-1');
  assert.equal(operations.includes('use_credential'),false);
  await assert.rejects(recovery.prepareFresh({previousRequestId:'request-1',intent:{...intent,targetId:'other'}}),{code:'login_recovery_unverified'});
  assert.equal(operations.filter(op=>op==='prepare_credential_login').length,1);
});

test('business failure settlement retains unknown history and is not retry permission',async()=>{
  const original={requestId:'request-1',attemptId:'attempt-1',state:'unknown',running:false};
  let completion;
  const client={async call(operation,args){
    if(operation==='inspect_credential_request')return {...original};
    if(operation==='complete_authentication'){
      assert.equal(args.outcome,'failed');
      completion={requestId:'request-1',state:'completed',completion:{outcome:'failed',previousState:'unknown'},receivingSettlement:{status:'settled',privacy:'retained'}};
      return completion;
    }
    if(operation==='inspect_authentication_completion')return completion;
    throw Error('Unexpected credential operation');
  }};
  const result=await createLoginRecovery({client}).recordFailure({requestId:'request-1',idempotencyKey:'failure-1',observedAt:1000,observation:{applicationOrigin:'https://app.example',accountEvidence:'identity-proof',evidenceRefs:['failure-proof']}});
  assert.equal(result.previousInputOutcome,'unknown');
  assert.equal(result.settlement,'settled');
  assert.equal(result.retryAllowed,false);
  assert.equal(original.state,'unknown');
});
