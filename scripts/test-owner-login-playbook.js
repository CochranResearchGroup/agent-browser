import assert from 'node:assert/strict';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';

const {createOwnerLoginPlaybook}=await import('../packages/client/src/owner-login-playbook.js');
const f=await createOwnerAuthFixture();
try{
 const login=createOwnerLoginPlaybook(f);
 const pending=await login.advance(f.options);
 assert.equal(pending.reasonCode,'approval_required');assert.equal(f.state.reads,0);assert.equal(f.state.inputs,0);
 assert.deepEqual((await f.auth.helper('inspect_credential_request',{requestId:pending.requestId})).approvalChoices,{standing:true,durations:['5 minutes','15 minutes','1 hour','8 hours']});
 f.respond('allow_once');
 const done=await login.advance(f.options);assert.equal(done.status,'completed');assert.equal(done.effectState,'verified');
 assert.equal(f.state.reads,1);assert.equal(f.state.inputs,1);assert.equal(done.requestId,pending.requestId);
 assert.equal((await login.advance(f.options)).status,'completed');assert.equal(f.state.inputs,1);
 assert.ok(!JSON.stringify({pending,done}).includes('SYNTHETIC-PASSWORD-ONLY'));
 console.log('Owner login reaches review without a site rule and completes one private use through actual Auth and parent source');
}finally{await f.close();}

const forged=await createOwnerAuthFixture();
try{
 const login=createOwnerLoginPlaybook(forged);await login.advance(forged.options);forged.respond('allow_once',{actor:'slack:intruder'});
 const held=await login.advance(forged.options);assert.notEqual(held.status,'completed');assert.equal(forged.state.reads,0);assert.equal(forged.state.inputs,0);
}finally{await forged.close();}

const outage=await createOwnerAuthFixture({configure:()=>({initialRules:[{id:'delivery-required',effect:'allow',clients:['client'],operations:['login'],modes:['private'],approval:'per-use',notificationGate:'delivered'}]})});
try{
 outage.state.notificationsReady=false;
 const login=createOwnerLoginPlaybook(outage),pending=await login.advance(outage.options);outage.respond();
 const held=await login.advance(outage.options);assert.equal(held.reasonCode,'notification_required');assert.equal(outage.state.reads,0);assert.equal(outage.state.inputs,0);
 outage.state.notificationsReady=true;
 const done=await login.advance(outage.options);assert.equal(done.status,'completed');assert.equal(done.requestId,pending.requestId);assert.equal(outage.state.inputs,1);assert.equal(outage.state.prompts.length,1);
}finally{await outage.close();}

const changed=await createOwnerAuthFixture();
try{
 const login=createOwnerLoginPlaybook(changed);await login.advance(changed.options);changed.respond();
 changed.browser.validatePlan=async()=>({...changed.options,generation:'replacement-generation',status:'completed'});
 assert.equal((await login.advance(changed.options)).reasonCode,'task_target_changed');assert.equal(changed.state.reads,0);assert.equal(changed.state.inputs,0);
}finally{await changed.close();}

const identity=await createOwnerAuthFixture();
try{
 const login=createOwnerLoginPlaybook(identity);await login.advance(identity.options);identity.respond();identity.state.verificationOrganization='wrong-company';
 assert.notEqual((await login.advance(identity.options)).status,'completed');assert.equal(identity.state.inputs,1);
 identity.state.verificationOrganization='company';assert.equal((await login.advance(identity.options)).status,'completed');assert.equal(identity.state.inputs,1);
}finally{await identity.close();}
console.log('Forged decisions, notification outage, changed generation and wrong-company evidence cannot complete owner login');

// Public SiteGuide metadata plus actual Auth/parent authority: only age expires.
async function ageFixture(){
 const f=await createOwnerAuthFixture();f.options.expiresAt=f.state.time+300000;
 const binding=Object.fromEntries(['taskId','profileId','browserId','tabId','generation'].map(k=>[k,f.options[k]]));
 f.preparations=0;f.validations=[];
 f.browser.prepareLogin=async()=>({...binding,status:'completed',reasonCode:'plan_prepared',plan:{planId:'plan-'+(++f.preparations),guideId:f.options.guideId,guideDigest:'reviewed-guide',bindingDigest:'reviewed-binding',controlFingerprint:'reviewed-controls',expectedState:'password_required',taskId:binding.taskId,profileId:binding.profileId,browserId:binding.browserId,tabId:binding.tabId,preparedAtMs:f.state.time,maximumAgeMs:60000}});
 f.browser.validatePlan=async({plan})=>{f.validations.push(plan.planId);const stale=f.state.time-plan.preparedAtMs>plan.maximumAgeMs;return {...binding,status:stale?'adaptation_required':'completed',reasonCode:stale?'plan_stale':'plan_current'};};
 return f;
}
const aged=await ageFixture();
try{
 const login=createOwnerLoginPlaybook(aged),pending=await login.advance(aged.options);
 const original=await aged.auth.helper('inspect_credential_request',{requestId:pending.requestId});
 assert.equal(original.state,'pending');aged.state.time+=60001;
 const key=`owner:${aged.options.taskId}/${aged.options.guideId}`;
 const prior=await aged.checkpoints.load(key);assert.equal((await aged.browser.validatePlan({...aged.options,plan:prior.preparedPlan})).reasonCode,'plan_stale');
 const renewed=await login.advance(aged.options);assert.equal(renewed.reasonCode,'approval_required','An age-only stale plan renews while retaining owner review');
 assert.equal(renewed.requestId,pending.requestId);assert.equal(aged.state.prompts.length,1);assert.equal(aged.state.reads,0);assert.equal(aged.state.inputs,0);
 const saved=await aged.checkpoints.load(key);assert.equal(saved.preparedPlan.planId,'plan-2');assert.equal(saved.advanceIntent,false);assert.ok(aged.validations.includes('plan-2'),'Renewed plan receives fresh validation');
 const readback=await aged.auth.helper('inspect_credential_request',{requestId:pending.requestId});assert.equal(readback.createdAt,original.createdAt);assert.deepEqual(readback.approvalChoices,original.approvalChoices);
 aged.respond();assert.equal((await login.advance(aged.options)).status,'completed');assert.equal(aged.state.reads,1);assert.equal(aged.state.inputs,1);assert.equal(aged.state.prompts.length,1);
}finally{await aged.close();}
console.log('Age-only SiteGuide renewal retains the original owner request and freshly validates durable metadata before private use');

for(const change of ['guideId','guideDigest','bindingDigest','controlFingerprint','expectedState','taskId','profileId','browserId','tabId','generation','missing-original-proof','missing-renewed-proof','unsupported-refusal','renewed-validation','save-failure','uncertain-intent','attempted']){
 const f=await ageFixture();
 try{
  const prepare=f.browser.prepareLogin,validate=f.browser.validatePlan,save=f.checkpoints.save;let saves=0;
  f.browser.prepareLogin=async()=>{const value=await prepare();if(change==='missing-original-proof'&&f.preparations===1)delete value.plan.bindingDigest;if(f.preparations>1){if(change==='generation')value.generation='replacement';else if(change==='missing-renewed-proof')delete value.plan.bindingDigest;else if(Object.hasOwn(value.plan,change))value.plan[change]='replacement';}return value;};
  f.browser.validatePlan=async options=>{const value=await validate(options);if(change==='unsupported-refusal'&&value.reasonCode==='plan_stale')value.reasonCode='observation_stale';if(change==='renewed-validation'&&options.plan.planId==='plan-2')return {...value,status:'adaptation_required',reasonCode:'prepared_controls_changed'};return value;};
  f.checkpoints.save=async(key,value)=>{if(value.preparedPlan.planId==='plan-2'){assert.ok(f.validations.includes('plan-2'),'Fresh validation precedes durable renewal');if(change==='save-failure')throw Error('synthetic_renewal_save_failure');saves++;}return save(key,value);};
  const login=createOwnerLoginPlaybook(f),pending=await login.advance(f.options);
  if(change==='uncertain-intent'){f.respond();f.auth.advanceLogin=async()=>{throw Error('synthetic_lost_dispatch_ack');};await login.advance(f.options);}
  if(change==='attempted'){f.respond();assert.equal((await login.advance(f.options)).status,'completed');}
  f.state.time+=60001;
  if(change==='save-failure')await assert.rejects(login.advance(f.options),/synthetic_renewal_save_failure/);
  else{const held=await login.advance(f.options);assert.equal(held.requestId,pending.requestId);if(change!=='attempted')assert.notEqual(held.status,'completed',change);}
  assert.equal(f.state.prompts.length,1,change+': no replacement owner review');assert.equal(f.state.reads,change==='attempted'?1:0);assert.equal(f.state.inputs,change==='attempted'?1:0);assert.equal(saves,0,change+': no eligible renewal saved');
  const checkpoint=await f.checkpoints.load(`owner:${f.options.taskId}/${f.options.guideId}`);assert.equal(checkpoint.preparedPlan.planId,'plan-1');
  if(['attempted','uncertain-intent','unsupported-refusal','missing-original-proof'].includes(change))assert.equal(f.preparations,1,change+': no preparation after unsupported proof or dispatch custody');
 }finally{await f.close();}
}
console.log('Incomplete or changed plan proof, unsupported refusals, failed save and attempted/unknown custody retain the original request without effects');
