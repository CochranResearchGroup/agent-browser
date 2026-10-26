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
