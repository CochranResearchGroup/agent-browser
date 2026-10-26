import assert from 'node:assert/strict';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';
import {createOwnerTotpContinuation} from '../packages/client/src/owner-totp-continuation.js';

async function fixture({loseInput=false,notificationGate='delivered',withPassword=false}={}){
 const f=await createOwnerAuthFixture({configure({state,receiving,receiver,now}){
  state.time=45000;state.factorKind='totp';state.factorAccount='owner@example.test';state.factorInputs=0;state.factorReconciles=0;
  return {materials:{key:Buffer.alloc(32,3),readClock:async()=>({synchronized:true,timeMs:state.time}),minimumRemainingMs:0,verifyEnrollment:async()=>true},
   receiver:{...receiver,
    async observeAuthentication({principal,targetId,origin}){return {method:state.factorKind,observationId:'synthetic-totp-form',account:state.factorAccount,binding:{clientId:principal.id,taskId:principal.taskId,targetId,origin,generation:receiving.generation}};},
    async use({credential,authorize}){await authorize();if(withPassword&&credential.type!=='otp'){state.inputs++;return {released:true};}assert.equal(credential.type,'otp');assert.equal(credential.code,'287082','Trusted receiver verifies the RFC synthetic factor');state.factorInputs++;state.authenticated=true;if(loseInput)throw Error('Synthetic lost factor acknowledgement');return {released:true,authenticated:true};},
    async reconcile({request}){state.factorReconciles++;return {released:state.factorInputs>0,authenticated:state.authenticated,terminal:true,evidenceId:request.attemptId};}
   }};
 }});
 try{
  const staged=await f.ownerCall('stage_mfa',{sourceId:'synthetic',recordId:'synthetic:billing',origin:f.options.origin,input:{kind:'key',value:'GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ'},options:{account:'owner@example.test',issuer:'Synthetic'}});
  await f.ownerCall('validate_mfa',{enrollmentId:staged.id});
  for(const [id,operations,approval]of [['discover',['discover'],'standing'],['login',['login'],'per-use']])await f.ownerCall('put_credential_rule',{rule:{id,effect:'allow',clients:['client'],sources:['mfa'],credentialTypes:['otp'],operations,origins:[f.options.origin],modes:['private'],approval,notificationGate}});
  const verify=f.browser.verifyLogin;f.browser.verifyLogin=async options=>{const observed=await verify(options);return {...observed,reasonCode:f.state.authenticated?observed.reasonCode:'mfa_required'};};
  return f;
 }catch(error){await f.close();throw error;}
}

{
 const f=await fixture();
 try{
  const flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const pending=await flow.advance(f.options);
  assert.equal(pending.reasonCode,'approval_required');assert.ok(pending.requestId);assert.equal(f.state.factorInputs,0);assert.equal(f.state.reads,0);
  const request=await f.auth.helper('inspect_credential_request',{requestId:pending.requestId});
  assert.equal(request.sourceId,'mfa');assert.equal(request.credentialType,'otp');assert.equal(request.continuationOf,undefined,'SSO needs no fictitious password request');
  f.respond();const complete=await flow.advance(f.options);
  assert.equal(complete.status,'completed');assert.equal(complete.requestId,pending.requestId);assert.equal(f.state.factorInputs,1);assert.equal(f.state.reads,0);
  await f.ownerCall('flush_credential_events');
  const history=await f.ownerCall('credential_history',{});const events=history.events.filter(e=>e.requestId===pending.requestId);
  assert.ok(events.some(e=>e.type==='requested'));assert.ok(events.some(e=>e.type==='released'));
  assert.equal(JSON.stringify({pending,complete,events}).includes('GEZDGNBV'),false);
 }finally{await f.close();}
}
console.log('TOTP continuation after SSO uses the original owner-approved factor request with no password retrieval');

{
 const f=await fixture({loseInput:true});
 try{
  let flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const pending=await flow.advance(f.options);f.respond();
  const unknown=await flow.advance(f.options);
  assert.equal(unknown.status,'waiting');assert.equal(unknown.effectState,'unknown');assert.equal(f.state.factorInputs,1);
  const before=await f.auth.helper('inspect_credential_request',{requestId:pending.requestId});assert.equal(before.state,'unknown');assert.ok(before.attemptId);
  await f.restart();flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const recovered=await flow.advance(f.options);
  assert.equal(recovered.status,'completed');assert.equal(recovered.requestId,pending.requestId);assert.equal(f.state.factorInputs,1);assert.equal(f.state.reads,0);
  const after=await f.auth.helper('inspect_credential_request',{requestId:pending.requestId});assert.equal(after.attemptId,before.attemptId);
  const history=await f.ownerCall('credential_history',{});assert.equal(history.requests.length,1);assert.ok(f.state.factorReconciles>0);
  assert.equal(JSON.stringify({pending,unknown,recovered,history}).includes('287082'),false);
  assert.equal(JSON.stringify({pending,unknown,recovered,history}).includes('GEZDGNBV'),false);
 }finally{await f.close();}
}
console.log('Lost TOTP input acknowledgement survives authority/orchestrator restart without a replacement request or input');

{
 const f=await fixture();
 try{
  f.state.notificationsReady=false;
  const flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const pending=await flow.advance(f.options);f.respond();
  const held=await flow.advance(f.options);assert.equal(held.reasonCode,'notification_required');assert.equal(f.state.factorInputs,0);
  f.state.notificationsReady=true;
  const completed=await flow.advance(f.options);assert.equal(completed.status,'completed');assert.equal(completed.requestId,pending.requestId);assert.equal(f.state.factorInputs,1);
 }finally{await f.close();}
}

for(const mode of ['unsupported','missing-account','wrong-account','generation','wrong-company','forged-owner']){
 const f=await fixture();
 try{
  const flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  if(mode==='unsupported')f.state.factorKind='push';
  if(mode==='missing-account')f.state.factorAccount='';
  if(mode==='wrong-account')f.state.factorAccount='other@example.test';
  const pending=await flow.advance(f.options);
  if(['unsupported','missing-account','wrong-account'].includes(mode)){
   assert.equal(pending.status,'waiting');assert.equal(pending.requestId,undefined);assert.equal(f.state.prompts.length,0);
  }else{
   if(mode==='generation'){const validate=f.browser.validatePlan;f.browser.validatePlan=async options=>({...await validate(options),generation:'other-generation'});}
   if(mode==='wrong-company')f.state.verificationOrganization='other-company';
   f.respond('allow_once',{actor:mode==='forged-owner'?'slack:forged':'slack:owner'});
   const result=await flow.advance(f.options);
   assert.notEqual(result.status,'completed');
   assert.equal(f.state.factorInputs,mode==='wrong-company'?1:0);
  }
  assert.equal(f.state.reads,0);
 }finally{await f.close();}
}
console.log('Required delivery, factor classification/account, exact generation, owner identity and company verification remain enforced');

for(const loseInput of [false,true]){
 const f=await fixture({withPassword:true,loseInput});
 try{
  const found=await f.auth.helper('discover_credentials',{query:f.options.query,accountHint:f.options.accountHint});
  const original=await f.auth.requestLogin({sourceId:found.selected.sourceId,recordId:found.selected.recordId,origin:f.options.origin,targetId:f.options.targetId,purpose:'Synthetic preceding password attempt',expiresAt:f.options.expiresAt,idempotencyKey:'preceding-password'});
  f.respond();const attempted=await f.auth.advanceLogin({requestId:original.requestId});assert.ok(attempted.attemptId);assert.equal(attempted.state,'unknown');
  let flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const options={...f.options,originalRequestId:original.requestId};
  const pending=await flow.advance(options);assert.equal(pending.reasonCode,'approval_required');assert.equal(pending.originalRequestId,original.requestId);
  f.respond();let completed=await flow.advance(options);
  if(loseInput){assert.equal(completed.effectState,'unknown');assert.equal(f.state.factorInputs,1);await f.restart();flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});completed=await flow.advance(options);}
  assert.equal(completed.status,'completed');assert.equal(completed.originalRequestId,original.requestId);assert.equal(f.state.inputs,1);assert.equal(f.state.factorInputs,1);
  const factor=await f.auth.helper('inspect_credential_request',{requestId:completed.requestId});
  assert.equal(factor.authenticationBinding,undefined);assert.equal(factor.continuationOf,original.requestId,'Same-target factor must transfer original receiving custody');assert.equal(factor.factorAccountId,original.recordId);
 }finally{await f.close();}
}
console.log('A preceding password attempt transfers exact provider continuation custody into the selected factor');

{
 const f=await fixture();
 try{
  const flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const pending=await flow.advance(f.options);f.respond();
  const save=f.checkpoints.save;
  f.checkpoints.save=async(key,value)=>{if(value.useIntent)throw Error('Synthetic durable storage refusal');return save(key,value);};
  await assert.rejects(()=>flow.advance(f.options),/durable storage refusal/);
  assert.equal(f.state.factorInputs,0);
  assert.equal((await f.auth.helper('inspect_credential_request',{requestId:pending.requestId})).attemptId,undefined);
 }finally{await f.close();}
}
console.log('A failed durable factor intent prevents private input');
