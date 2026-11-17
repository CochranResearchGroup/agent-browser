import assert from 'node:assert/strict';
import {createOwnerLoginPlaybook} from '../packages/client/src/owner-login-playbook.js';

// Public composition seam. Only external Auth and receiving-browser ports are
// injected; no browser process, SDK installation or private credential is used.
function fixture({discovery,discoveryError}={}){
 const binding={taskId:'task',profileId:'profile',browserId:'browser',tabId:'tab',generation:'generation'};
 const options={...binding,guideId:'guide',accountRef:'intended-account',query:'example.test',accountHint:'owner@example.test',origin:'https://example.test',targetId:'target',purpose:'Synthetic login recovery',expiresAt:10000,idempotencyKey:'original'};
 const calls=[],saved=new Map();
 const browser={async verifyLogin(){return {...binding,status:'waiting',reasonCode:'password_required'};},async prepareLogin(){return {...binding,status:'completed',plan:{planId:'plan'}};},async validatePlan(){return {...binding,status:'completed'};}};
 const auth={callerTaskId:'caller',async helper(operation,args){calls.push({operation,args});if(operation==='discover_credentials'){if(discoveryError)throw Object.assign(Error('SYNTHETIC-PRIVATE-DETAIL'),{code:discoveryError});return discovery??{state:'not_found'};}throw Error('Unexpected Auth operation');},async requestLogin(){throw Error('No request expected');},async advanceLogin(){throw Error('No input expected');}};
 const checkpoints={async load(key){return saved.get(key)??null;},async save(key,value){saved.set(key,structuredClone(value));}};
 return {options,binding,auth,browser,checkpoints,calls,playbook:createOwnerLoginPlaybook({auth,browser,checkpoints})};
}

const unavailable=fixture({discoveryError:'auth_unavailable'});
const result=await unavailable.playbook.advance(unavailable.options);
assert.equal(result.reasonCode,'auth_unavailable');
assert.equal(result.effectState,'no_effect');
assert.equal(result.recovery.classification,'integration_failure');
assert.equal(result.recovery.fallbackEligible,true);
assert.equal(result.recovery.repairOwner,'auth-receipts');
assert.deepEqual(result.recovery.nextOperations,[{target:'browser',operation:'verifyLogin'},{target:'auth',operation:'auth_capabilities'}]);
assert.deepEqual(unavailable.calls.map(call=>call.operation),['discover_credentials'],'Recovery guidance performs no additional calls');
assert.ok(!JSON.stringify(result).includes('SYNTHETIC-PRIVATE-DETAIL'));
console.log('Unavailable Auth reports a bounded integration diagnostic, not denial or retry authority');

for(const [scenario,classification,fallbackEligible] of [
 [{discoveryError:'auth_not_authorized'},'authorization_denied',false],
 [{discovery:{state:'not_found'}},'credential_not_found',true],
 [{discovery:{state:'choice_required'}},'account_ambiguous',false],
 [{discovery:{state:'partial',unavailable:['source']}},'source_unavailable',true],
 [{discovery:{state:'partial',truncated:true}},'source_unavailable',true],
 [{discoveryError:'auth_sdk_incompatible'},'capability_unavailable',true],
 [{discoveryError:'auth_channel_discovery_unavailable'},'capability_unavailable',true],
 [{discoveryError:'auth_sdk_invalid'},'integration_failure',true],
 [{discoveryError:'unrecognized_error'},'unclassified',false],
]){
 const f=fixture(scenario),observed=await f.playbook.advance(f.options);
 assert.equal(observed.recovery.classification,classification);
 assert.equal(observed.recovery.fallbackEligible,fallbackEligible);
 assert.equal(observed.effectState,'no_effect');
 assert.ok(observed.recovery.nextOperations.length<=2);
 assert.deepEqual(f.calls.map(call=>call.operation),['discover_credentials']);
}
const unready=fixture();
unready.browser.verifyLogin=async()=>{throw Error('Receiving transport closed');};
const held=await unready.playbook.advance(unready.options);
assert.equal(held.recovery.classification,'receiving_unready');
assert.equal(held.recovery.fallbackEligible,false);
assert.deepEqual(held.recovery.nextOperations,[{target:'browser',operation:'verifyLogin'}]);
assert.deepEqual(unready.calls,[]);
console.log('Denial, no match, ambiguity, unavailable source/capability and receiving unreadiness remain distinct');

for(const [field,value] of Object.entries(unavailable.binding))assert.equal(result[field],value,'Original receiving binding survives pre-request Auth failure');

const uncertain=fixture();
let inputs=0,requests=0;
let request={requestId:'original-request',taskId:'caller',sourceId:'vault',recordId:'record',origin:uncertain.options.origin,targetId:uncertain.options.targetId,expiresAt:uncertain.options.expiresAt,mode:'private',operation:'login',state:'approved'};
uncertain.auth.helper=async(operation,args)=>{
 uncertain.calls.push({operation,args});
 if(operation==='discover_credentials')return {state:'unique',selected:{sourceId:'vault',recordId:'record',origin:uncertain.options.origin}};
 if(['inspect_credential_request','reconcile_credential_request'].includes(operation)){assert.equal(args.requestId,'original-request');return structuredClone(request);}
 throw Error('Unexpected operation');
};
uncertain.auth.requestLogin=async()=>{requests++;return structuredClone(request);};
uncertain.auth.advanceLogin=async()=>{inputs++;request={...request,state:'unknown',attemptId:'original-attempt'};throw Object.assign(Error('SYNTHETIC-PRIVATE-DETAIL'),{code:'auth_unavailable'});};
const unknown=await uncertain.playbook.advance(uncertain.options);
assert.equal(unknown.reasonCode,'auth_unavailable');
assert.equal(unknown.effectState,'unknown');
assert.equal(unknown.recovery.classification,'unknown_outcome');
assert.equal(unknown.recovery.fallbackEligible,false);
assert.equal(unknown.requestId,'original-request');
for(const [field,value] of Object.entries(uncertain.binding))assert.equal(unknown[field],value);
assert.deepEqual(unknown.recovery.nextOperations,[{target:'browser',operation:'verifyLogin'},{target:'auth',operation:'reconcile_credential_request',arguments:{requestId:'original-request'}}]);
assert.equal(uncertain.calls.filter(call=>call.operation==='reconcile_credential_request').length,0,'A recommendation does not execute recovery');
const resumed=await createOwnerLoginPlaybook(uncertain).advance({...uncertain.options,reconcile:true});
assert.equal(resumed.requestId,'original-request');
assert.equal(resumed.recovery.classification,'unknown_outcome');
assert.equal(resumed.recovery.fallbackEligible,false);
assert.equal(inputs,1,'Read-only recovery never repeats unknown input');
assert.equal(requests,1,'Read-only recovery never creates replacement approval');
console.log('Unknown use retains the original receiving request and requires reconciliation before fallback');
