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
const unknownClient=createNativeSavedLoginClient({...unknown,expectedAccount:'configured@example.test'});
assert.equal((await unknownClient.resolve({taskId:'task',episodeId:'episode'})).status,'uncertain');
assert.equal((await unknownClient.resolve({taskId:'task',episodeId:'episode'})).status,'uncertain');
assert.equal(unknown.inputs.length,1);
// The original deadline survives reopen; neither transport uncertainty nor time rollback renews it.
unknown.clock=61000;
assert.equal((await unknownClient.resolve({taskId:'task',episodeId:'episode'})).reason,'native_saved_login_budget_exhausted');
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
