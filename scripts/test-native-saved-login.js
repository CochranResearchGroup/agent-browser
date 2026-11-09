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
   const captured=envelope({source:'remote_view_native',loginControls:{schemaVersion:'agent-browser.native-login-controls.v1',status:'observed',controls,frameSha256:'a'.repeat(64)},binding:{desktopId:'desktop',generation:1,windowId:42,nativeControlEpoch:0,observedAtMs:clock}});captures.set(args.requestId,structuredClone(captured));return captured;
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
