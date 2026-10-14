import assert from 'node:assert/strict';
import {createLoginPlaybook} from '../packages/client/src/login-playbook.js';

// Invariant: an interrupted submission is never retried, including by a new
// orchestrator. Helper custody and fresh receiving verification determine success.
const binding={taskId:'task',profileId:'profile',browserId:'browser',tabId:'tab'};
let stored=null,submissions=0,authenticated=false;
const request={requestId:'request',taskId:'task',state:'pending',scope:{person:'person',tenant:'tenant',account:'account',source:'source',origin:'https://fixture.invalid',capabilities:['login']},expiresAt:Date.now()+60000};
const options={...binding,guideId:'fixture',accountRef:'account',organizationRef:'company',scope:request.scope,expiresAt:request.expiresAt};
const checkpoints={load:async()=>structuredClone(stored),save:async(_key,value)=>{stored=structuredClone(value);}};
const browser={prepareLogin:async()=>({plan:{guideDigest:'guide',controlFingerprint:'controls'}}),validatePlan:async()=>({...binding,status:'completed',reasonCode:'plan_current'}),verifyLogin:async()=>({...binding,status:authenticated?'completed':'waiting',reasonCode:authenticated?'authenticated':'password_required',effectState:'no_effect'})};
const helper=async(name)=>{
 if(name==='request_login')return structuredClone(request);
 if(name==='inspect_request')return structuredClone(request);
 if(name==='authenticate'){submissions++;request.startedAt=Date.now();request.inputAttemptedAt=Date.now();throw Error('lost acknowledgement');}
 throw Error('unexpected operation');
};
let playbook=createLoginPlaybook({browser,helper,checkpoints});
assert.equal((await playbook.advance(options)).reasonCode,'approval_required');
request.state='approved';
assert.equal((await playbook.advance(options)).effectState,'unknown');
assert.equal(submissions,1);
playbook=createLoginPlaybook({browser,helper,checkpoints});
assert.equal((await playbook.advance(options)).reasonCode,'helper_running_or_uncertain');
assert.equal(submissions,1,'Never resubmit after an unknown effect');
request.state='authenticated';request.captureState='ready';authenticated=true;
assert.equal((await playbook.advance(options)).status,'completed');
assert.equal(submissions,1);
request.state='uncertain';
assert.equal((await playbook.advance(options)).status,'completed','Safe uncertain outcomes reconcile through receiving readback');
request.state='authenticated';
const noInput={...request,state:'uncertain'};delete noInput.inputAttemptedAt;
const locked=createLoginPlaybook({browser,helper:async()=>structuredClone(noInput),checkpoints});
assert.equal((await locked.advance(options)).reasonCode,'password_source_unavailable');
assert.equal((await locked.advance(options)).effectState,'no_effect');
const mismatch={...options,organizationRef:'other-company'};
await assert.rejects(()=>playbook.advance(mismatch),/checkpoint_binding_mismatch/);
request.captureState='held';
assert.equal((await playbook.advance(options)).reasonCode,'private_capture_held');
request.captureState='ready';
const moved=createLoginPlaybook({browser:{...browser,verifyLogin:async()=>({...binding,tabId:'other',status:'completed'})},helper,checkpoints});
assert.equal((await moved.advance(options)).reasonCode,'task_target_changed');
await assert.rejects(()=>playbook.advance({...options,authenticationMethod:'passkey'}),/checkpoint_binding_mismatch/);
request.state='revoked';
assert.equal((await playbook.advance(options)).reasonCode,'helper_revoked');
let reads=0;
const unavailable=createLoginPlaybook({browser:{...browser,verifyLogin:async()=>({...binding,status:'waiting',reasonCode:'password_required'})},helper:async()=>{throw Error('offline');},checkpoints:{load:async()=>null,save:async()=>{reads++;}}});
assert.equal((await unavailable.advance(options)).reasonCode,'helper_unavailable');
assert.equal(reads,0);
assert.equal((await unavailable.advance({...options,authenticationMethod:'passkey'})).reasonCode,'authentication_method_unsupported');
request.state='approved';delete request.startedAt;delete request.inputAttemptedAt;
stored={...stored,attempted:false};
const brokenStore=createLoginPlaybook({browser,helper,checkpoints:{load:checkpoints.load,save:async()=>{throw Error('disk full');}}});
await assert.rejects(()=>brokenStore.advance(options),/disk full/);
assert.equal(submissions,1,'Failed durable intent makes no submission');
console.log('Login playbook restart reconciliation and exact binding passed');

// A typed pre-input refusal can be reconciled; transport uncertainty cannot.
let retrySaved=null,retries=0,requests=0;
const retryRequest={...request,state:'approved',captureState:'ready'};
const retryStore={load:async()=>structuredClone(retrySaved),save:async(_key,value)=>{retrySaved=structuredClone(value);}};
const retryHelper=async operation=>{
 if(operation==='request_login'){requests++;return structuredClone(retryRequest);}
 if(operation==='inspect_request')return structuredClone(retryRequest);
 retries++;if(retries===1)throw Object.assign(Error('held'),{code:'auth_capture_held'});
 retryRequest.startedAt=Date.now();retryRequest.inputAttemptedAt=Date.now();
 retryRequest.state='authenticated';return structuredClone(retryRequest);
};
const retryPlaybook=createLoginPlaybook({browser:{...browser,verifyLogin:async()=>({...binding,status:retryRequest.state==='authenticated'?'completed':'waiting',reasonCode:'password_required'})},helper:retryHelper,checkpoints:retryStore});
assert.equal((await retryPlaybook.advance(options)).reasonCode,'private_capture_held');
assert.equal(retries,1);assert.equal((await retryPlaybook.advance(options)).status,'completed');
assert.equal(retries,2);assert.equal(requests,1,'Reuse exact approved request after proven pre-input refusal');

// Reviewed replacement has fresh exact-target validation and unchanged action.
let adaptationSaved=null,planVersion='old',adaptationSubmissions=0;
const adaptationBrowser={...browser,
 prepareLogin:async()=>({plan:{version:planVersion}}),
 validatePlan:async({plan})=>({...binding,status:plan.version===planVersion?'completed':'adaptation_required',reasonCode:plan.version===planVersion?'plan_current':'prepared_controls_changed'}),
 verifyLogin:async()=>({...binding,status:adaptationSubmissions?'completed':'waiting',reasonCode:'password_required'})};
const adaptationRequest={...request,state:'pending',captureState:'ready'};
const adaptation=createLoginPlaybook({browser:adaptationBrowser,checkpoints:{load:async()=>structuredClone(adaptationSaved),save:async(_key,value)=>{adaptationSaved=structuredClone(value);}},
 helper:async operation=>{if(operation==='authenticate'){adaptationSubmissions++;adaptationRequest.state='authenticated';}return structuredClone(adaptationRequest);}});
const retained=await adaptation.advance(options);adaptationRequest.state='approved';planVersion='reviewed';
assert.equal((await adaptation.advance(options)).reasonCode,'prepared_controls_changed');assert.equal(adaptationSubmissions,0);
assert.equal((await adaptation.advance({...options,reviewedPlan:{version:'invalid'}})).status,'adaptation_required');assert.equal(adaptationSubmissions,0);
const adapted=await adaptation.advance({...options,reviewedPlan:{version:'reviewed'}});
assert.equal(adapted.status,'completed');assert.equal(adapted.actionId,retained.actionId);assert.equal(adaptationSubmissions,1);
assert.throws(()=>createLoginPlaybook({browser:{verifyLogin:browser.verifyLogin},helper,checkpoints}),/dependencies_required/);
console.log('Required plan checks, reviewed same-action adaptation and proven pre-input refusal recovery passed');
