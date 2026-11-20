import assert from 'node:assert/strict';
import {createNativeSavedLoginClient} from '../packages/client/src/native-saved-login.js';
const envelope=data=>({content:[{type:'text',text:JSON.stringify({success:true,data})}]});
function fixture(){
 const episodes=new Map(),operations=new Map(),captures=new Map(),inputs=[];let step=0,clock=1000;
 const binding={taskId:'task',profileId:'profile',browserId:'browser',tabId:'tab',targetId:'target'};
 const field=(kind,targetRef,extra={})=>({kind,targetRef,label:kind,bounds:{x:10,y:20,width:100,height:30},fieldValueState:'empty',...extra});
 const f={episodes,operations,inputs,binding,get step(){return step},set clock(v){clock=v},
 store:{async withEpisode(_id,run){return run()},async load(id){return structuredClone(episodes.get(id)??null)},async save(value){episodes.set(value.episodeId,structuredClone(value))}},
 async callTool(_name,args){
  if(args.action==='task_briefing')return envelope({...binding,control:{owner:'agent',epoch:0}});
  if(args.action==='task_operation'){const operation=operations.get(args.params.requestId);return operation?envelope({operation}):{isError:true,content:[{type:'text',text:JSON.stringify({success:false,error:{code:'browser_task_operation_not_found'}})}]};}
  if(args.action==='native_capture'){
   if(captures.has(args.requestId))return structuredClone(captures.get(args.requestId));
   const controls=step===0?[field('username','username')]:step===1?[field('credential_entry','credential',{configuredAccountMatch:true})]:[field('username','filled-user',{configuredAccountMatch:true,fieldValueState:'text_present'}),field('password','filled-password',{fieldValueState:'masked'}),field('submit','submit')];
   const captured=envelope({source:'remote_view_native',loginControls:{schemaVersion:'agent-browser.native-login-controls.v1',status:'observed',controls,frameSha256:'a'.repeat(64)},binding:{desktopId:'desktop',generation:1,windowId:42,nativeControlEpoch:0,observedAtMs:clock,displayWidth:1000,displayHeight:800,window:{x:0,y:0,width:1000,height:800},region:{x:0,y:0,width:1000,height:800}}});captures.set(args.requestId,structuredClone(captured));return captured;
  }
  if(args.action==='native_input'){
   inputs.push(structuredClone(args));const kind=['username','credential_entry','submit'][step];
   operations.set(args.requestId,{taskId:'task',requestId:args.requestId,action:'native_input',state:'owner_acknowledged',evidenceReferences:{nativeLoginEpisode:'episode',nativeLoginKind:kind,nativeLoginStep:String(step),nativeLoginTargetRef:args.params.loginTarget.targetRef,nativeLoginObservationRequestId:args.params.loginTarget.observationRequestId,nativeDelivery:'dispatched'}});step++;return envelope({source:'remote_view_native',delivery:'dispatched'});
  }
  throw Error('unexpected fixture operation');
 },async verifyAccess(context){return {...binding,state:'verified',accountMatched:step===3,evidenceRef:'independent-account-proof'}},now:()=>clock};
 return f;
}
const f=fixture();
const client=createNativeSavedLoginClient({callTool:f.callTool,store:f.store,verifyAccess:f.verifyAccess,expectedAccount:'configured@example.test',now:f.now});
const result=await client.resolve({taskId:'task',episodeId:'episode'});
assert.equal(result.status,'verified_access_after_native_saved_login');
assert.equal(f.inputs.length,3);
assert.deepEqual(f.inputs.map(x=>x.params.loginTarget.targetRef),['username','credential','submit']);
assert(f.inputs.every(x=>!('actions' in x.params)&&!('x' in x.params.loginTarget)&&!('password' in x.params)));
console.log('native saved login: configured credential workflow passes');
const wrapped=fixture(),base=wrapped.callTool;
wrapped.callTool=async(name,args)=>{
 if(args.action==='task_operation'&&!wrapped.operations.has(args.params.requestId))throw Error('Mcp error: -32602: Invalid params({"message":"browser_task_operation_not_found"})');
 return base(name,args);
};
assert.equal((await createNativeSavedLoginClient({...wrapped,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).status,'verified_access_after_native_saved_login');
// A lost reply rejoins its original acknowledged input; it never clicks twice.
const lost=fixture(),lostBase=lost.callTool;let lose=true;
lost.callTool=async(name,args)=>{const value=await lostBase(name,args);if(args.action==='native_input'&&lose){lose=false;throw Error('reply lost after delivery')}return value};
const lostClient=createNativeSavedLoginClient({...lost,expectedAccount:'configured@example.test'});
assert.equal((await lostClient.resolve({taskId:'task',episodeId:'episode'})).status,'uncertain');
assert.equal((await lostClient.resolve({taskId:'task',episodeId:'episode'})).status,'verified_access_after_native_saved_login');
assert.equal(lost.inputs.length,3);
// A missing original record after an uncertain delivery cannot authorize replay.
const unknown=fixture(),unknownBase=unknown.callTool;
unknown.callTool=async(name,args)=>{if(args.action==='native_input'){unknown.inputs.push(args);throw Error('transport outcome unknown')}return unknownBase(name,args)};
const unknownClient=createNativeSavedLoginClient({...unknown,expectedAccount:'configured@example.test',timeoutMs:60000});
assert.equal((await unknownClient.resolve({taskId:'task',episodeId:'episode'})).status,'uncertain');
assert.equal((await unknownClient.resolve({taskId:'task',episodeId:'episode'})).status,'uncertain');
assert.equal(unknown.inputs.length,1);
// Guest clock movement neither expires evidence nor authorizes uncertain replay.
unknown.clock=61000;
assert.equal((await unknownClient.resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_original_uncertain');
assert.equal(unknown.inputs.length,1);
const wrong=fixture();wrong.verifyAccess=async()=>({...wrong.binding,state:'verified',accountMatched:false,evidenceRef:'wrong-account-proof'});
assert.equal((await createNativeSavedLoginClient({...wrong,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).status,'human_required');
const changed=fixture(),changedBase=changed.callTool;let briefings=0;
changed.callTool=async(name,args)=>args.action==='task_briefing'&&++briefings===2?envelope({...changed.binding,tabId:'foreign-tab',control:{owner:'agent',epoch:0}}):changedBase(name,args);
assert.equal((await createNativeSavedLoginClient({...changed,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_task_binding_changed');
assert.equal(changed.inputs.length,0);
const ambiguous=fixture(),ambiguousBase=ambiguous.callTool;
ambiguous.callTool=async(name,args)=>{const reply=await ambiguousBase(name,args);if(args.action==='native_capture'&&ambiguous.step===1){const data=JSON.parse(reply.content[0].text).data;data.loginControls.controls.push({...data.loginControls.controls[0],targetRef:'another-credential'});return envelope(data)}return reply};
assert.equal((await createNativeSavedLoginClient({...ambiguous,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_target_ambiguous');
assert.equal(ambiguous.inputs.length,1);
console.log('native saved login: retained reply reconciliation, immutable budget, account, binding and ambiguity guards pass');

// A new observation after a supported wait keeps the original input IDs and deadline.
ambiguous.clock=12000;ambiguous.callTool=ambiguousBase;
assert.equal((await createNativeSavedLoginClient({...ambiguous,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).status,'verified_access_after_native_saved_login');
assert.equal(ambiguous.inputs.length,3);
const rebound=createNativeSavedLoginClient({...f,expectedAccount:'other@example.test'});
assert.equal((await rebound.resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_retained_identity_changed');
assert.equal(f.inputs.length,3);
const returned=fixture(),returnedBase=returned.callTool;
returned.callTool=async(name,args)=>args.action==='task_briefing'?envelope({...returned.binding,control:{owner:'agent',epoch:returned.step>0?2:0}}):returnedBase(name,args);
assert.equal((await createNativeSavedLoginClient({...returned,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_task_control_changed');
assert.equal(returned.inputs.length,1);

// A 1440x900 Chrome chooser below y694 remains discoverable through bounded
// owner captures. The input uses a fresh target from the selected complete tile.
const lower=fixture(),lowerBase=lower.callTool,lowerRegions=[];
lower.callTool=async(name,args)=>{
 const reply=await lowerBase(name,args);
 if(args.action!=='native_capture')return reply;
 const data=JSON.parse(reply.content[0].text).data;
 const region=args.params.region??{x:0,y:0,width:1440,height:694};
 lowerRegions.push(region);
 data.binding={...data.binding,displayWidth:1440,displayHeight:900,window:{x:0,y:0,width:1440,height:900},region};
 if(lower.step===1){
  data.loginControls.controls=region.y<=704&&region.y+region.height>=820?[{kind:'credential_entry',targetRef:'credential',configuredAccountMatch:true,bounds:{x:600,y:704,width:300,height:116}}]:[];
  data.loginControls.status=data.loginControls.controls.length?'observed':'not_found';
 }
 return envelope(data);
};
const lowerResult=await createNativeSavedLoginClient({...lower,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'});
assert.equal(lowerResult.status,'verified_access_after_native_saved_login');
assert.equal(lower.inputs.length,3);
assert(lowerRegions.every(r=>r.width*r.height<=1000000));
assert(lowerRegions.some(r=>r.y<=704&&r.y+r.height>=820));
console.log('native saved login: lower-window chooser covered without larger owner captures');

function tiledFixture(change=()=>{}){
 const f=fixture(),base=f.callTool,frames=new Map();let sequence=0;
 f.callTool=async(name,args)=>{
  const reply=await base(name,args);
  if(args.action==='native_capture'){
   sequence++;const data=JSON.parse(reply.content[0].text).data;
   const region=args.params.region??{x:100,y:50,width:1440,height:694};
   data.binding={...data.binding,displayWidth:1600,displayHeight:1000,window:{x:100,y:50,width:1440,height:900},region};
   for(const control of data.loginControls.controls)control.bounds={x:650,y:770,width:300,height:40};
   if(!args.params.region){data.loginControls.controls=[];data.loginControls.status='not_found';}
   change({data,args,sequence,step:f.step});
   for(const control of data.loginControls.controls)control.targetRef+=':'+sequence;
   frames.set(args.requestId,structuredClone(data));return envelope(data);
  }
  if(args.action==='native_input'){
   const selected=frames.get(args.params.loginTarget.observationRequestId);
   assert(selected.loginControls.controls.some(c=>c.targetRef===args.params.loginTarget.targetRef));
   assert.equal(selected.binding.region.y,50);
   assert.equal(selected.binding.region.height,900);
  }
  return reply;
 };return f;
}
const shifted=tiledFixture();
assert.equal((await createNativeSavedLoginClient({...shifted,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).status,'verified_access_after_native_saved_login');
assert.equal(shifted.inputs.length,3);
const competing=tiledFixture(({data,args})=>{if(args.params.region?.x===429)data.loginControls.controls.push({...data.loginControls.controls[0],bounds:{x:1000,y:770,width:200,height:40},targetRef:'competing'});});
assert.equal((await createNativeSavedLoginClient({...competing,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_target_ambiguous');
assert.equal(competing.inputs.length,0);
const moved=tiledFixture(({data,sequence})=>{if(sequence===3)data.binding.window.y++;});
assert.equal((await createNativeSavedLoginClient({...moved,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_native_binding_changed');
assert.equal(moved.inputs.length,0);
const vanished=tiledFixture(({data,sequence})=>{if(sequence===4){data.loginControls.controls=[];data.loginControls.status='not_found';}});
assert.equal((await createNativeSavedLoginClient({...vanished,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_target_ambiguous');
assert.equal(vanished.inputs.length,0);
console.log('native saved login: shifted origins, overlapping duplicates, distinct ambiguity, moved geometry and vanished targets pass');
const freshAmbiguous=tiledFixture(({data,sequence})=>{if(sequence===4)data.loginControls.controls.push({...data.loginControls.controls[0],targetRef:'new-target',bounds:{x:980,y:770,width:100,height:40}});});
assert.equal((await createNativeSavedLoginClient({...freshAmbiguous,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_target_ambiguous');
assert.equal(freshAmbiguous.inputs.length,0);


// New invocation allowances do not rewrite an already retained episode clock.
import {test} from 'node:test';
test('saved login new invocation allowance is configurable and defaults to five minutes',async()=>{
 const f=fixture();await createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'});
 const e=f.episodes.get('episode');assert.equal(e.deadlineAtMs-e.createdAtMs,300000);
 const short=fixture();short.callTool=async()=>new Promise(()=>{});
 const result=await createNativeSavedLoginClient({...short,expectedAccount:'configured@example.test',timeoutMs:20}).resolve({taskId:'task',episodeId:'episode'});
 assert.equal(result.reason,'native_saved_login_invocation_budget_exhausted');assert.equal(short.inputs.length,0);assert.equal(result.continuation.arguments.episodeId,'episode');assert.equal(result.continuation.arguments.allowAcknowledgedResume,false);
 for(const timeoutMs of [0,-1,NaN,Infinity,2147483648])assert.throws(()=>createNativeSavedLoginClient({...fixture(),expectedAccount:'configured@example.test',timeoutMs}),/configuration/);
});
async function acknowledgedPrefix(){
 const f=fixture(),base=f.callTool;let block=true;
 f.callTool=async(...args)=>{if(block&&args[1].action==='native_capture'&&f.step===2)throw Error('fixture deliberate read-only stop');return base(...args)};
 const first=await createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test',timeoutMs:60000}).resolve({taskId:'task',episodeId:'episode'});
 assert.equal(first.status,'human_required');assert.equal(f.inputs.length,2);
 const episode=f.episodes.get('episode');delete episode.timeoutMs;f.clock=episode.createdAtMs+7200000;block=false;
 return f;
}
test('saved login explicit acknowledged resume preserves old clocks and never replays the prefix',async()=>{
 const f=await acknowledgedPrefix(),old=structuredClone(f.episodes.get('episode'));
 const client=createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test',timeoutMs:300000});
 const result=await client.resolve({taskId:'task',episodeId:'episode',allowAcknowledgedResume:true});
 assert.equal(result.status,'verified_access_after_native_saved_login');assert.equal(f.inputs.length,3);
 assert.deepEqual(f.inputs.map(i=>i.requestId),['episode:input1','episode:input2','episode:input3']);
 const after=f.episodes.get('episode');assert.equal(after.createdAtMs,old.createdAtMs);assert.equal(after.deadlineAtMs,old.deadlineAtMs);assert.deepEqual(after.inputs.slice(0,2),old.inputs);
});
test('saved login explicit resume refuses intent or unknown prefix without proof calls or upgrades',async()=>{
 for(const state of ['intent','unknown']){const f=await acknowledgedPrefix();f.episodes.get('episode').inputs[1].state=state;let calls=0;const base=f.callTool;f.callTool=async(...a)=>{calls++;return base(...a)};
 const result=await createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode',allowAcknowledgedResume:true});assert.equal(result.status,'uncertain');assert.equal(calls,0);assert.equal(f.inputs.length,2);assert.equal(f.episodes.get('episode').inputs[1].state,state);}
});
test('saved login acknowledged resume refuses foreign context and bounds read-only proof work',async()=>{
 const foreign=await acknowledgedPrefix(),base=foreign.callTool;foreign.callTool=async(...a)=>a[1].action==='task_briefing'?envelope({...foreign.binding,tabId:'foreign',control:{owner:'agent',epoch:0}}):base(...a);
 const changed=await createNativeSavedLoginClient({...foreign,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode',allowAcknowledgedResume:true});assert.notEqual(changed.status,'verified_access_after_native_saved_login');assert.equal(foreign.inputs.length,2);
 const slow=await acknowledgedPrefix();slow.callTool=async()=>new Promise(()=>{});
 const stopped=await createNativeSavedLoginClient({...slow,expectedAccount:'configured@example.test',timeoutMs:20}).resolve({taskId:'task',episodeId:'episode',allowAcknowledgedResume:true});assert.equal(stopped.reason,'native_saved_login_invocation_budget_exhausted');assert.equal(slow.inputs.length,2);
});

test('saved login acknowledged resume requires positive absence of the next original input',async()=>{
 const f=await acknowledgedPrefix();f.operations.set('episode:input3',{taskId:'task',requestId:'episode:input3',action:'native_input',state:'owner_acknowledged',evidenceReferences:{nativeLoginEpisode:'episode',nativeLoginKind:'submit',nativeLoginStep:'2',nativeDelivery:'dispatched'}});let captures=0;const base=f.callTool;f.callTool=async(...a)=>{if(a[1].action==='native_capture')captures++;return base(...a)};
 const r=await createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode',allowAcknowledgedResume:true});assert.equal(r.reason,'native_saved_login_acknowledged_resume_unverified');assert.equal(captures,0);assert.equal(f.inputs.length,2);
});
test('saved login resume never creates a fresh episode or extends a fully spent episode',async()=>{
 const fresh=fixture();assert.equal((await createNativeSavedLoginClient({...fresh,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode',allowAcknowledgedResume:true})).reason,'native_saved_login_acknowledged_resume_unverified');assert.equal(fresh.episodes.size,0);
 const spent=fixture(),client=createNativeSavedLoginClient({...spent,expectedAccount:'configured@example.test'});await client.resolve({taskId:'task',episodeId:'episode'});const old=structuredClone(spent.episodes.get('episode'));assert.equal((await client.resolve({taskId:'task',episodeId:'episode',allowAcknowledgedResume:true})).reason,'native_saved_login_acknowledged_resume_unverified');assert.equal(spent.inputs.length,3);assert.deepEqual(spent.episodes.get('episode'),old);
});

// An acknowledged completed plan can be extended at runtime for a further page.
const extended=fixture(),extendedBase=extended.callTool;
extended.verifyAccess=async()=>({...extended.binding,state:'verified',accountMatched:extended.inputs.length===4,evidenceRef:'next-page-account-proof'});
extended.callTool=async(name,args)=>{
 const reply=await extendedBase(name,args);
 if(args.action==='native_input'){
  const op=extended.operations.get(args.requestId);
  op.evidenceReferences.nativeLoginEpisode=args.params.loginTarget.episodeId;
  op.evidenceReferences.nativeLoginKind=['username','credential_entry','submit','submit'][extended.inputs.length-1];
 }
 return reply;
};
const extendedClient=createNativeSavedLoginClient({...extended,expectedAccount:'configured@example.test'});
const firstPage=await extendedClient.resolve({taskId:'task',episodeId:'episode'});
assert.equal(firstPage.status,'human_required');
assert(firstPage.continuation,'An unverified completed page must offer executable continuation');
const nextPage=await extendedClient.resolve({taskId:'task',episodeId:'episode',allowAcknowledgedResume:true,steps:['username','credential_entry','submit','submit']});
assert.equal(nextPage.status,'verified_access_after_native_saved_login');
assert.equal(extended.inputs.length,4,'Acknowledged first-page inputs must not replay');
console.log('native saved login: runtime extension continues an acknowledged multi-page plan');


test('saved login guest wall jumps cannot expire owner evidence or bypass unknown originals',async()=>{
 for(const jump of [-7200000,7200000]){
  const f=fixture(),base=f.callTool;let calls=0;
  f.callTool=async(...args)=>{f.clock=1000+(++calls%2?jump:-jump);const reply=await base(...args);if(args[1].action==='native_capture'){const data=JSON.parse(reply.content[0].text).data;data.binding.observedAtMs=Number.MAX_SAFE_INTEGER;return envelope(data);}return reply;};
  const result=await createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'});
  assert.equal(result.status,'verified_access_after_native_saved_login');assert.equal(f.inputs.length,3);
 }
 const f=await acknowledgedPrefix();const result=await createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'});
 assert.equal(result.status,'verified_access_after_native_saved_login');assert.equal(f.inputs.length,3);
});

test('saved login preserves actual owner time refusal and never dispatches through it',async()=>{
 const f=fixture(),base=f.callTool;const failure={code:'browser_native_time_recapture_required',occurrenceId:'time-owner-760',effectState:'not_dispatched',retryDisposition:'inspect_before_retry'};
 f.callTool=async(...args)=>args[1].action==='native_input'?{isError:true,content:[{type:'text',text:JSON.stringify({success:false,failure})}]}:base(...args);
 const result=await createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'});
 assert.equal(result.failure.code,failure.code);assert.equal(result.occurrenceId,failure.occurrenceId);assert.equal(f.inputs.length,0);
});

test('public owner continuation constructor errors expose caller-only field correction without ports',async()=>{
 for(const [module,name] of [['owner-login-playbook','createOwnerLoginPlaybook'],['owner-number-matching','createOwnerNumberMatchingContinuation'],['owner-totp-continuation','createOwnerTotpContinuation']]){
  const exported=await import('../packages/client/src/'+module+'.js');
  assert.throws(()=>exported[name]({}),error=>error instanceof TypeError&&error.code==='client_argument_invalid'&&error.owningFunction===name&&error.recovery.checkedFields.includes('auth')&&error.logging.ownerOccurrenceId===null);
 }
});

test('saved login original owner no-effect settlement is failed, never acknowledged or replayed',async()=>{
 const f=fixture();const operation={...f.binding,requestId:'episode:input1',action:'native_input',state:'owner_refused_no_effect',controlEpoch:0,evidenceReferences:{}};f.operations.set(operation.requestId,operation);
 const result=await createNativeSavedLoginClient({...f,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'});
 assert.equal(result.status,'failed_no_effect');assert.equal(result.originalOperation.requestId,operation.requestId);assert.equal(result.originalOperation.state,'owner_refused_no_effect');assert.equal(result.retryAllowed,false);assert.equal(f.inputs.length,0);
 assert.deepEqual(result.recovery.newIntent.requiredArguments,['newEpisodeId']);assert.equal(f.episodes.get('episode').inputs.length,0);
 const forged=fixture();forged.callTool=async()=>{throw Object.assign(Error('native_saved_login_original_refused_no_effect'),{ownerNoEffectProof:true,originalOperation:operation})};
 const rejected=await createNativeSavedLoginClient({...forged,expectedAccount:'configured@example.test'}).resolve({taskId:'task',episodeId:'episode'});
 assert.notEqual(rejected.status,'failed_no_effect');assert.equal(rejected.originalOperation,undefined);assert.equal(forged.inputs.length,0);
});
