/** Existing MCP + verified Auth/parent composition; provider transport is synthetic.
 * This does not qualify a live provider, Native input, vault or credential factor. */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';
import {preflightAuthComposition} from './lib/auth-composition-preflight.mjs';

export async function runLoginJourneyFixture({call,software,taskId,directory,origin,guideId,appHtml,accountRef,organizationRef}){
 assert(origin.startsWith('https://127.0.0.1:'),'Only the owned synthetic TLS site');
 const source=process.env.AUTH_RECEIPTS_SOURCE,parentSource=process.env.RECEIPTS_SOURCE,sdkSource=process.env.AUTH_RECEIPTS_SDK_SOURCE;
 const qualification=await preflightAuthComposition({source,parentSource,sdkSource});
 assert(Object.values(qualification.sources).every(value=>value.dirty===false),'Pin clean reviewed source companions');
 const path=join(directory,guideId+'.json'),original=readFileSync(path,'utf8'),guide=JSON.parse(original);
 const calls=[],actions=[],authCalls=[];
 const invoke=async(name,args)=>{calls.push({tool:name,operation:args.params?.operation??args.action??name});return call(name,args);};
 const evaluate=script=>invoke('browser_command',{taskId,action:'evaluate',params:{script}});
 const verify=async()=>{calls.push({tool:'browser_command',operation:'verify_login'});return software.verifyLogin({taskId,guideId,accountRef,organizationRef});};
 let authority,provider=false;
 const generation='synthetic-external-provider-generation';
 try{
  guide.loginGuidance={version:'journey-v1',source:'synthetic-composed-journey',validationClass:'authored',evidenceDate:'2026-11-17',limitations:['External provider transport is synthetic.'],providerBranches:[{branchId:'google',startingOrigin:origin,providerOrigin:'https://accounts.google.com',returnOrigin:origin,selector:'#google-sso'}]};
  writeFileSync(path,JSON.stringify(guide));
  await call('browser_command',{taskId,action:'evaluate',params:{script:'document.body.innerHTML="<input id=password type=password><button id=google-sso>Continue with Google</button>"'}});
  const observed=await invoke('browser_snapshot',{taskId});
  const candidate=observed.loginGuidance.candidates.find(item=>item.guideId===guideId);
  assert(candidate.providerBranches.some(branch=>branch.branchId==='google'&&branch.observed));
  const {tool,...inspection}=candidate.nextOperation;
  const inspected=await invoke(tool,inspection);
  const branch=inspected.loginGuidance.providerBranches.find(item=>item.branchId==='google');
  const binding=observed.loginGuidance;
  // Auth's trusted receiving boundary owns these synthetic provider events.
  // Website evidence is always observed independently through actual public MCP.
  authority=await createOwnerAuthFixture({source,parentSource,sdkSource,configure:({receiver})=>{
   receiver.sso={
    async observe(){
     const common={targetId:taskId,resourceId:binding.browserId,runtimeGeneration:generation,observationId:randomUUID()};
     if(provider)return {...common,origin:branch.providerOrigin,kind:'provider_authenticated',account:'synthetic-provider@example.invalid'};
     const state=await evaluate(`(()=>{const app=document.querySelector(${JSON.stringify(guide.authenticatedSelector)});const account=document.querySelector(${JSON.stringify(guide.account.selector)});return {authenticated:!!app,account:account?${guide.account.attribute?'account.getAttribute('+JSON.stringify(guide.account.attribute)+')':'account.textContent'}:null,provider:!!document.querySelector('#google-sso')};})()`);
     return {...common,origin,kind:state.result.authenticated?'website_authenticated':state.result.provider?'website_signin':'rendering',account:state.result.account,providers:state.result.provider?['Google']:[]};
    },
    async perform({action,authorize}){
     await authorize();actions.push(action.kind);
     if(action.kind==='select_provider'){
      assert.equal(action.provider,'Google');
      await invoke('browser_click',{taskId,selector:branch.selector});provider=true;
     }else if(action.kind==='return_to_website'){
      provider=false;
      await evaluate('document.body.innerHTML="<p>Completing synthetic provider return</p>"');
     }else throw Error('Unexpected synthetic provider action');
     return {acknowledged:true};
    },
   };
   return {receiver};
  }});
  const auth=(operation,args)=>{authCalls.push(operation);return authority.auth.helper(operation,args);};
  const intent={idempotencyKey:'original-observed-sso',targetId:taskId,websiteOrigin:origin,websiteAccount:accountRef,providerHint:branch.branchId,providerAccount:'synthetic-provider@example.invalid',expiresAt:authority.state.time+60000};
  const prepared=await auth('prepare_sso_login',intent);
  const recovered=await auth('prepare_sso_login',intent);
  assert.equal(recovered.journeyId,prepared.journeyId,'Lost metadata response cannot replace the journey');
  const progress=await auth('continue_sso_login',{journeyId:prepared.journeyId,provider:branch.branchId});
  assert.equal(progress.providerAuthentication,'verified');
  assert.equal(progress.websiteAuthentication,'not_verified');
  assert.notEqual(progress.state,'completed','Provider success is not website authentication');
  assert.notEqual((await verify()).status,'completed','Transient return remains unverified');
  await evaluate(`document.body.innerHTML=${JSON.stringify(appHtml.replaceAll(accountRef,'wrong-synthetic-account'))}`);
  const wrong=await auth('continue_sso_login',{journeyId:prepared.journeyId});
  assert.equal(wrong.heldFor,'website_account_mismatch');
  assert.equal((await verify()).reasonCode,'account_mismatch');
  // Simulate the external website's corrected return, never another login input.
  await evaluate(`document.body.innerHTML=${JSON.stringify(appHtml)}`);
  const done=await auth('continue_sso_login',{journeyId:prepared.journeyId});
  assert.equal(done.state,'completed');assert.equal(done.websiteAuthentication,'verified');
  const final=await verify();assert.equal(final.status,'completed');
  for(const field of ['taskId','profileId','browserId','tabId'])assert.equal(final[field],binding[field]);
  assert.equal((await auth('inspect_sso_login',{journeyId:prepared.journeyId})).journeyId,prepared.journeyId);
  assert.equal((await auth('continue_sso_login',{journeyId:prepared.journeyId})).state,'completed');
  assert.deepEqual(actions,['select_provider','return_to_website']);
  assert.equal(authority.state.reads,0);assert.equal(authority.state.inputs,0);assert.equal(authority.state.prompts.length,0);
  console.log(JSON.stringify({loginJourney:{browserCalls:calls.length,authCalls:authCalls.length,journeys:1,providerSelections:1,syntheticReturns:1,credentialUses:0,approvals:0,originalTaskPreserved:true,providerTransport:'synthetic',eliminatedDetours:['source-code investigation','manual SSO discovery','manual journey reconstruction'],residualWork:['intended website/provider account selection','live factor qualification in ticket732']}}));
 }finally{await authority?.close();writeFileSync(path,original);}
}
