import assert from 'node:assert/strict';
import {test} from 'node:test';
import {createLoginVerifier} from '../packages/client/src/login-verification.js';

const binding={taskId:'browser-task',profileId:'profile-1',browserId:'browser-1',tabId:'tab-1',targetId:'target-1',origin:'https://accounts.example.test'};
const input={requestId:'request-1',attemptId:'attempt-1',binding,applicationOrigin:'https://app.example.test',expectedAccount:'intended@example.test'};
function fixture() {
  const request={requestId:'request-1',attemptId:'attempt-1',clientId:'client-1',taskId:'caller-1',targetId:'browser-task',origin:binding.origin,state:'completed',operation:'login',mode:'private'};
  let access={...binding,state:'unverified',accountMatched:false,evidenceRef:'app-proof-1',applicationOrigin:input.applicationOrigin};
  let completion={requestId:'request-1',state:'completed',completion:{outcome:'succeeded',evidenceAuthority:'caller_observed',actor:{clientId:'client-1',taskId:'caller-1'},returnId:'return-1',observation:{applicationOrigin:input.applicationOrigin,accountEvidence:'account-proof-1',evidenceRefs:['app-proof-1']}},receivingSettlement:{status:'settled',privacy:'retained'}};
  const client={async call(operation,args) {
    assert.equal(args.requestId,'request-1');
    if(operation==='inspect_credential_request')return structuredClone(request);
    if(operation==='inspect_authentication_completion')return structuredClone(completion);
    throw Error('Unexpected effect operation: '+operation);
  }};
  const verifyAccess=async context=>{
    assert.equal(context.expectedAccount,input.expectedAccount);
    assert.deepEqual(context.binding,binding);
    return structuredClone(access);
  };
  return {request,client,verifyAccess,set access(value){access=value;},get access(){return access;},set completion(value){completion=value;},get completion(){return completion;}};
}

test('closed source tickets and completed Auth bookkeeping do not prove application access',async()=>{
  const f=fixture();
  const verifier=createLoginVerifier({client:f.client,verifyAccess:f.verifyAccess,callerTaskId:'caller-1'});
  const result=await verifier.verify({...input,trackerState:'closed',sourceTests:'passed'});
  assert.equal(result.outcome,'incomplete');
  assert.equal(result.category,'verification_incomplete');
  assert.equal(result.qualification.applicationAccess,'unverified');
});

test('org-required access cannot succeed without independent organization match',async()=>{
  const f=fixture();f.access={...f.access,state:'verified',accountMatched:true};
  const verifier=createLoginVerifier({client:f.client,verifyAccess:f.verifyAccess,callerTaskId:'caller-1'});
  assert.equal((await verifier.verify({...input,expectedOrganization:'intended-org'})).outcome,'incomplete');
  f.access={...f.access,organizationMatched:true};
  assert.equal((await verifier.verify({...input,expectedOrganization:'intended-org'})).outcome,'verified');
});

test('filled forms, chooser, provider-only success and wrong account stay incomplete',async()=>{
  for (const patch of [
    {state:'filled',accountMatched:true},
    {state:'remembered_account',accountMatched:true},
    {state:'technical_error',accountMatched:true},
    {state:'verified',accountMatched:false},
    {state:'verified',accountMatched:true,applicationOrigin:binding.origin},
    {state:'verified',accountMatched:true,tabId:'foreign-tab'},
  ]) {
    const f=fixture();f.access={...f.access,...patch};
    const result=await createLoginVerifier({client:f.client,verifyAccess:f.verifyAccess,callerTaskId:'caller-1'}).verify(input);
    assert.equal(result.outcome,'incomplete');
  }
});

test('owner bookkeeping cannot substitute for proof and must refer to original evidence',async()=>{
  for (const patch of [
    {outcome:'unresolved'},
    {outcome:'failed'},
    {actor:{taskId:'foreign-caller',clientId:'client-1'}},
    {actor:{taskId:'caller-1',clientId:'foreign-client'}},
    {observation:{applicationOrigin:input.applicationOrigin,accountEvidence:'account-proof-1',evidenceRefs:['old-proof']}},
    {observation:{applicationOrigin:binding.origin,accountEvidence:'account-proof-1',evidenceRefs:['app-proof-1']}},
  ]) {
    const f=fixture();f.access={...f.access,state:'verified',accountMatched:true};
    f.completion={...f.completion,completion:{...f.completion.completion,...patch}};
    const result=await createLoginVerifier({client:f.client,verifyAccess:f.verifyAccess,callerTaskId:'caller-1'}).verify(input);
    assert.equal(result.outcome,'incomplete');
    assert.equal(result.qualification.applicationAccess,'verified');
  }
});

test('independently verified account and original owner completion produce verified access',async()=>{
  const f=fixture();f.access={...f.access,state:'verified',accountMatched:true};
  const result=await createLoginVerifier({client:f.client,verifyAccess:f.verifyAccess,callerTaskId:'caller-1'}).verify(input);
  assert.equal(result.outcome,'verified');
  assert.equal(result.category,'verified_access');
  assert.deepEqual(result.evidenceReferences,['app-proof-1','return-1']);
  assert.deepEqual(result.qualification,{applicationAccess:'verified',authCompletion:'caller_observed',installation:'unverified',otherLoginPaths:'unverified'});
  assert.equal(result.receivingSettlement,'settled');
});

test('receiver settlement hold does not erase independently verified application access',async()=>{
  const f=fixture();f.access={...f.access,state:'verified',accountMatched:true};
  f.completion={...f.completion,receivingSettlement:{status:'held',reason:'receiver_unavailable'}};
  const result=await createLoginVerifier({client:f.client,verifyAccess:f.verifyAccess,callerTaskId:'caller-1'}).verify(input);
  assert.equal(result.outcome,'verified');
  assert.equal(result.receivingSettlement,'held');
});

test('changed request or browser binding cannot verify and no raw identity is emitted',async()=>{
  const f=fixture();f.access={...f.access,state:'verified',accountMatched:true,account:'intended@example.test',page:'PRIVATE_PAGE',cookie:'PRIVATE_COOKIE'};
  const client={async call(operation,args){const value=await f.client.call(operation,args);if(operation==='inspect_authentication_completion')f.request.attemptId='another-attempt';return value;}};
  const result=await createLoginVerifier({client,verifyAccess:f.verifyAccess,callerTaskId:'caller-1'}).verify(input);
  assert.equal(result.outcome,'incomplete');
  assert.equal(result.category,'request_binding_unverified');
  assert.equal(JSON.stringify(result).includes('intended@example.test'),false);
  assert.equal(JSON.stringify(result).includes('PRIVATE_'),false);
});

test('Auth completion read failure reports the gap without inventing provider failure',async()=>{
  const f=fixture();f.access={...f.access,state:'verified',accountMatched:true};
  const client={async call(operation,args){if(operation==='inspect_authentication_completion')throw Error('private transport detail');return f.client.call(operation,args);}};
  const result=await createLoginVerifier({client,verifyAccess:f.verifyAccess,callerTaskId:'caller-1'}).verify(input);
  assert.equal(result.outcome,'incomplete');
  assert.equal(result.category,'verification_unavailable');
  assert.equal(result.qualification.applicationAccess,'verified');
  assert.equal(JSON.stringify(result).includes('private transport detail'),false);
});
