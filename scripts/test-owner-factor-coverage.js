import assert from 'node:assert/strict';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';

// The existing real SDK/authority/parent fixture supplies caller/approval custody.
// Only external authenticator and mail/IM ports are synthetic. No installed state.
// The enrolled receiver starts at the already-observed second-factor screen.
// precedingPath labels that entry context; Native prefix effects are tested separately.
const code='287082',account='owner@example.test';
async function factorFixture(method,{available=true,qualified=true,precedingPath='browser_saved'}={}){
 const effects={arms:0,sends:0,reads:0,inputs:0,signatures:0,triggers:0,order:[],precedingPath};
 const f=await createOwnerAuthFixture({configure({state,receiving,receiver}){
  const origin='https://billing.example';
  const messageAuthentication={accounts:[{id:'mail',origin,account,recipient:'owner@mail.example.test',method:'email_code'},{id:'im',origin,account,recipient:'+15555550128',method:'sms_code'}],adapter:{
   async arm(binding){await binding.authorize();effects.arms++;effects.order.push('arm');return {ready:available,fence:'synthetic-fresh-fence'};},
   async send(binding){await binding.authorize();assert.equal(binding.fence,'synthetic-fresh-fence');effects.sends++;effects.order.push('send');return {acknowledged:true};},
   async poll(binding){await binding.authorize();return [{messageId:'synthetic-'+method,recipient:binding.recipient,origin,account,method,challengeId:binding.challengeId,receivedAt:state.time,expiresAt:state.time+30000}];},
   async receive(binding,consume){await binding.authorize();effects.reads++;effects.order.push('receive');await consume({code});},
  }};
  const passkeyAuthentication={adapter:{
   async discover({authorize}){await authorize();return available?[{id:'selected',origin,account,rpId:'billing.example',credentialRef:'c3ludGhldGljLWtleQ',userHandle:'b3duZXI'}]:[];},
   async use(_native,{authorize,trigger}){await authorize();await trigger();effects.signatures++;return {acknowledged:true};},
  }};
  return {messageAuthentication,passkeyAuthentication,receiver:{...receiver,
   async observeAuthentication({principal,targetId,origin:receivingOrigin}){return {method,account,observationId:'observed-'+method,requirementsSatisfied:qualified,binding:{clientId:principal.id,taskId:principal.taskId,targetId,origin:receivingOrigin,generation:receiving.generation}};},
   async use({credential,authorize}){
    await authorize();
    if(credential.type==='passkey')await credential.perform({generation:receiving.generation,rpId:credential.rpId,credentialRef:credential.credentialRef,userHandle:credential.userHandle,operation:'assertion'},{authorize,async trigger(){effects.triggers++;return {acknowledged:true};}});
    else {assert.equal(credential.type,'otp');assert.equal(credential.code,code);effects.inputs++;}
    state.authenticated=true;return {released:true,authenticated:true};
   },
  }};
 }});
 try{
  const source=method==='passkey'?'passkey':'message',credentialType=method==='passkey'?'passkey':'otp';
  for(const [id,operations,approval]of [['discover',['discover'],'standing'],['login',['login'],'per-use']])await f.ownerCall('put_credential_rule',{rule:{id,effect:'allow',clients:['client'],sources:[source],credentialTypes:[credentialType],operations,origins:[f.options.origin],modes:['private'],approval,notificationGate:'delivered'}});
  return {...f,effects};
 }catch(error){await f.close();throw error;}
}

for(const method of ['passkey','email_code','sms_code']){
 const f=await factorFixture(method,{precedingPath:method==='sms_code'?'lastpass':'browser_saved'});
 try{
  const before=await f.auth.helper('observe_authentication',{origin:f.options.origin,targetId:f.options.targetId});
  assert.equal(before.method,method);assert.equal(before.state,'authentication_required');
  const request=await f.auth.helper(before.nextAction.operation,{...before.nextAction.args,idempotencyKey:'factor-'+method,purpose:'Synthetic additional factor after '+f.effects.precedingPath});
  assert.equal(request.state,'pending');assert.equal(f.effects.sends,0);assert.equal(f.effects.signatures,0);assert.equal(f.state.reads,0);
  f.respond();const approved=await f.auth.helper('receive_credential_approval',{requestId:request.requestId});assert.equal(approved.state,'approved');
  const used=await f.auth.helper('use_credential',{requestId:request.requestId});
  assert.equal(used.authentication,'verified');assert.equal(used.requestId,request.requestId);assert.equal(used.targetId,f.options.targetId);
  assert.equal((await f.browser.verifyLogin()).status,'completed');
  const beforeEffects=structuredClone(f.effects);
  await f.auth.helper('use_credential',{requestId:request.requestId});
  assert.deepEqual(f.effects,beforeEffects,'A completed factor must never repeat an external effect');
  assert.equal(f.state.reads,0,'A browser/LastPass factor must not discover or read a fictitious password');
  if(method==='passkey'){assert.equal(f.effects.signatures,1);assert.equal(f.effects.triggers,1);assert.equal(f.effects.inputs,0);}
  else {assert.deepEqual(f.effects.order,['arm','send','receive']);assert.equal(f.effects.inputs,1);assert.equal(f.effects.reads,1);}
  assert.ok(!JSON.stringify({before,request,used}).includes(code));
  assert.ok(!JSON.stringify({before,request,used}).includes('c3ludGhldGljLWtleQ'));
  console.log(JSON.stringify({factor:method,qualification:'synthetic_supported',precedingPath:f.effects.precedingPath,requests:1,passwordReads:f.state.reads,effects:f.effects}));
 }finally{await f.close();}
}
for(const method of ['email_code','sms_code']){
 const f=await factorFixture(method,{available:false});
 try{
  const observed=await f.auth.helper('observe_authentication',{origin:f.options.origin,targetId:f.options.targetId});
  const request=await f.auth.helper(observed.nextAction.operation,{...observed.nextAction.args,idempotencyKey:'unavailable-'+method,purpose:'Synthetic source readiness'});
  f.respond();await f.auth.helper('receive_credential_approval',{requestId:request.requestId});
  const held=await f.auth.helper('use_credential',{requestId:request.requestId});
  assert.equal(held.state,'approved');assert.equal(held.heldFor,'source_readiness');assert.equal(held.attemptId,undefined);
  assert.equal(f.effects.sends,0);assert.equal(f.effects.reads,0);assert.equal(f.effects.inputs,0);
  console.log(JSON.stringify({factor:method,qualification:'synthetic_source_unavailable',privateInput:0}));
 }finally{await f.close();}
}
{
 const f=await factorFixture('passkey',{qualified:false});
 try{
  const held=await f.auth.helper('observe_authentication',{origin:f.options.origin,targetId:f.options.targetId});
  assert.equal(held.state,'held');assert.equal(held.heldFor,'authenticator_requirements');assert.equal(held.nextAction,undefined);
  assert.equal(f.effects.signatures,0);assert.equal(f.state.prompts.length,0);assert.equal(f.state.reads,0);
  console.log(JSON.stringify({factor:'passkey',qualification:'synthetic_requirements_unqualified',privateInput:0}));
 }finally{await f.close();}
}

{
 const f=await factorFixture('passkey',{available:false});
 try{
  const held=await f.auth.helper('observe_authentication',{origin:f.options.origin,targetId:f.options.targetId});
  assert.equal(held.method,'passkey');assert.equal(held.heldFor,'credential_visibility');assert.deepEqual(held.candidates,[]);
  assert.equal(f.effects.signatures,0);assert.equal(f.state.prompts.length,0);
  console.log(JSON.stringify({factor:'passkey',qualification:'synthetic_visibility_unavailable',privateInput:0}));
 }finally{await f.close();}
}
