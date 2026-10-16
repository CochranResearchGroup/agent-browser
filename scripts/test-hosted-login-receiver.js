import assert from 'node:assert/strict';
import {createHostedLoginReceiver} from '../packages/client/src/hosted-login-receiver.js';
import {homedir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
const installed=process.env.CODEX_BOTS_INSTALLATION_ROOT??join(homedir(),'.local/share/codex-bots/current');
const {createAuthService}=await import(pathToFileURL(join(installed,'src/auth-service.mjs')));
const {authPrivacy}=await import(pathToFileURL(join(installed,'src/auth-privacy.mjs')));

// Public producer service with controlled receiving I/O: this proves the real
// request contract and shared state orchestration, not live provider acceptance.
const scope={person:'operator',tenant:'owned',account:'owned-account',origin:'https://accounts.example.test',source:'owned-source',capabilities:['login']};
const binding={provider:'agent-browser',taskId:'browser-task',profileId:'owned-profile',browserId:'owned-browser',tabId:'owned-tab'};
const principal={role:'caller',id:'codex',taskId:'ordinary-native-thread'};
let state='identifier_required',identifier='',password='',reads=0;
const actions=[];
const receiver=createHostedLoginReceiver({binding,scope,expectedUsername:'owned-user',
 browser:{
  async observe(){return {binding:{...binding},origin:scope.origin,state,accountMatched:state==='identifier_required'||identifier==='owned-user',visiblePassword:state==='password_required',maskedPassword:true};},
  async fill({control,value}){actions.push(control);if(control==='identifier')identifier=value;else if(control==='password')password=value;else throw Error('Unexpected private field');},
  async activate({control}){actions.push(control);if(control==='identifierSubmit'&&identifier==='owned-user')state='password_required';else if(control==='submit'&&password==='owned-password')state='authenticated';else throw Error('Website rejected submission');},
  async waitForChange(){},
 },
 readPrincipal:async()=>({authenticated:state==='authenticated',origin:scope.origin,username:identifier}),
 readCaptureSafety:async()=>({safe:state==='authenticated'}),
});
const service=createAuthService({accounts:[{...scope,callers:['codex'],approvers:['operator']}],store:{async read(){reads++;return {username:'owned-user',password:'owned-password'};}},privacy:authPrivacy({records:{},persist:async()=>{}}),receiving:{validate:async()=>{},create:async()=>receiver}});
const request=await service.call(principal,'request_login',{...scope,expiresAt:Date.now()+60000,contractVersion:2,receivingTarget:binding});
await service.call({role:'approver',id:'operator'},'approve',{requestId:request.requestId});
const result=await service.call(principal,'authenticate',{requestId:request.requestId});
assert.equal(result.state,'authenticated');assert.equal(result.captureState,'ready');assert.equal(result.taskId,principal.taskId);assert.deepEqual(result.receivingTarget,binding);
assert.deepEqual(actions,['identifier','identifierSubmit','password','submit']);assert.equal(reads,1);
console.log('Hosted receiver: separate caller/receiving identity and identifier-first password journey passed');

// Actual QBO exposed identifier -> email challenge, rather than both fields.
// Keep this same producer request live while its owner performs MFA privately.
let challengeState='identifier_required',challengeIdentifier='',mfaCalls=0,releaseMfa,enteredMfa;
const mfaEntered=new Promise(resolve=>{enteredMfa=resolve;}),mfaGate=new Promise(resolve=>{releaseMfa=resolve;});
const challengeReceiver=createHostedLoginReceiver({binding,scope,expectedUsername:'owned-user',
 browser:{
  async observe(){return {binding:{...binding},origin:scope.origin,state:challengeState,accountMatched:challengeState==='identifier_required'||challengeIdentifier==='owned-user',visiblePassword:false,maskedPassword:true};},
  async fill({control,value}){assert.equal(control,'identifier');challengeIdentifier=value;},
  async activate({control}){assert.equal(control,'identifierSubmit');challengeState='mfa_required';},
  async waitForChange(){},
 },
 async awaitMfa({receivingTarget,guard}){assert.deepEqual(receivingTarget,binding);mfaCalls++;await guard();enteredMfa();await mfaGate;await guard();challengeState='authenticated';return {dispatched:true};},
 readPrincipal:async()=>({authenticated:challengeState==='authenticated',origin:scope.origin,username:challengeIdentifier}),
 readCaptureSafety:async()=>({safe:challengeState==='authenticated'}),
});
const challengeService=createAuthService({accounts:[{...scope,callers:['codex'],approvers:['operator']}],store:{async read(){return {username:'owned-user',password:'owned-password'};}},privacy:authPrivacy({records:{},persist:async()=>{}}),receiving:{validate:async()=>{},create:async()=>challengeReceiver}});
const challengeRequest=await challengeService.call(principal,'request_login',{...scope,expiresAt:Date.now()+60000,contractVersion:2,receivingTarget:binding});
await challengeService.call({role:'approver',id:'operator'},'approve',{requestId:challengeRequest.requestId});
const authentication=challengeService.call(principal,'authenticate',{requestId:challengeRequest.requestId});
await Promise.race([mfaEntered,authentication.then(()=>{throw Error('Authentication terminated before MFA continuation');})]);
const waiting=await challengeService.call(principal,'inspect_request',{requestId:challengeRequest.requestId});
assert.equal(waiting.running,true);assert.equal(waiting.requestId,challengeRequest.requestId);assert.equal(waiting.taskId,principal.taskId);assert.deepEqual(waiting.receivingTarget,binding);
releaseMfa();const challengeResult=await authentication;
assert.equal(challengeResult.state,'authenticated');assert.equal(challengeResult.requestId,waiting.requestId);assert.equal(challengeResult.captureState,'ready');assert.equal(mfaCalls,1);
console.log('Hosted receiver: same-request private MFA wait and return passed');

async function scenario({wrongTarget=false,wrongAccount=false,hiddenPassword=false,lostAck=false,wrongPrincipal=false,unsafeCapture=false,redirect=false,prematureRedirect=false}={}){
 let current='password_required',sourceReads=0,inputs=0,submits=0;
 const instance=createHostedLoginReceiver({binding,scope,expectedUsername:'owned-user',maxTransitions:1,authenticatedOrigins:redirect?['https://app.example.test']:[scope.origin],
  browser:{
   async observe(){return {binding:{...binding,...(wrongTarget?{tabId:'peer-tab'}:{})},origin:redirect&&(prematureRedirect||current==='authenticated')?'https://app.example.test':scope.origin,state:current,accountMatched:!wrongAccount,visiblePassword:current==='password_required'&&!hiddenPassword,maskedPassword:true};},
   async fill({control}){assert.equal(control,'password');inputs++;if(lostAck)throw Error('Simulated lost acknowledgement with private owned-password');},
   async activate(){submits++;current='authenticated';},async waitForChange(){},
  },
  readPrincipal:async()=>({authenticated:true,origin:redirect?'https://app.example.test':scope.origin,username:wrongPrincipal?'other-user':'owned-user'}),
  readCaptureSafety:async()=>({safe:!unsafeCapture}),
 });
 const host=createAuthService({accounts:[{...scope,callers:['codex'],approvers:['operator']}],store:{async read(){sourceReads++;return {username:'owned-user',password:'owned-password'};}},privacy:authPrivacy({records:{},persist:async()=>{}}),receiving:{validate:async()=>{},create:async()=>instance}});
 const grant=await host.call(principal,'request_login',{...scope,expiresAt:Date.now()+60000,contractVersion:2,receivingTarget:binding});
 await host.call({role:'approver',id:'operator'},'approve',{requestId:grant.requestId});
 let error;try{await host.call(principal,'authenticate',{requestId:grant.requestId});}catch(e){error=e;assert(!String(e).includes('owned-password'));}
 const summary=await host.call(principal,'inspect_request',{requestId:grant.requestId});
 return {instance,host,grant,summary,error,counts:()=>({sourceReads,inputs,submits})};
}
for(const input of [{wrongTarget:true},{wrongAccount:true},{hiddenPassword:true},{redirect:true,prematureRedirect:true}]){
 const fixture=await scenario(input);assert.deepEqual(fixture.counts(),{sourceReads:0,inputs:0,submits:0});assert.equal(fixture.summary.effectState,'no_effect');
}
const unknown=await scenario({lostAck:true});
assert.equal(unknown.summary.state,'uncertain');assert.equal(unknown.summary.captureState,'held');assert.equal(unknown.summary.effectState,'unknown');
await assert.rejects(unknown.host.call(principal,'authenticate',{requestId:unknown.grant.requestId}));assert.deepEqual(unknown.counts(),{sourceReads:1,inputs:1,submits:0});
const wrong=await scenario({wrongPrincipal:true});assert.equal(wrong.summary.state,'uncertain');assert.equal(wrong.summary.captureState,'held');
const unsafe=await scenario({unsafeCapture:true});assert.equal(unsafe.summary.state,'authenticated');assert.equal(unsafe.summary.captureState,'held');
console.log('Hosted receiver: wrong target/account/hidden password no source read, lost-ack no replay, wrong principal and held native capture passed');

const redirected=await scenario({redirect:true});assert.equal(redirected.summary.state,'authenticated');assert.equal(redirected.summary.captureState,'ready');console.log('Hosted receiver: reviewed authenticated-origin redirect preserves credential-origin scope');

let readbackState='identifier_required',identity='other-user',nativeSafe=false,privateCalls=0;
const readback=createHostedLoginReceiver({binding,scope,expectedUsername:'owned-user',
 browser:{async observe(){return {binding:{...binding},origin:scope.origin,state:readbackState,accountMatched:false,visiblePassword:false,maskedPassword:true};},async fill(){privateCalls++;assert.fail();},async activate(){privateCalls++;assert.fail();},async waitForChange(){privateCalls++;assert.fail();}},
 readPrincipal:async()=>({authenticated:true,origin:scope.origin,username:identity}),readCaptureSafety:async()=>{assert.fail('Reconciliation must not need the old credential');},readReconciliationSafety:async()=>({safe:nativeSafe}),
});
assert.equal((await readback.reconcile(scope)).authenticated,false);
readbackState='unknown';assert.equal((await readback.reconcile(scope)).authenticated,false);
readbackState='authenticated';assert.equal((await readback.reconcile(scope)).authenticated,false);
identity='owned-user';const heldReadback=await readback.reconcile(scope);assert.equal(heldReadback.authenticated,true);assert.equal(heldReadback.captureSafe,false);
nativeSafe=true;const safeReadback=await readback.reconcile(scope);assert.equal(safeReadback.authenticated,true);assert.equal(safeReadback.captureSafe,true);assert.equal(safeReadback.account,scope.account);assert.equal(privateCalls,0);
console.log('Hosted receiver: independent same-browser reconciliation needs no prior credential or input replay');
