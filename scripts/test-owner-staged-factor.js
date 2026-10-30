import assert from 'node:assert/strict';
import {mkdir} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';
import {createOwnerTotpContinuation} from '../packages/client/src/owner-totp-continuation.js';

// Actual published Auth authority, encrypted KeePass archive, native receiver and
// SDK with synthetic desktop/provider ports. No installed discovery or browser.
const source=process.env.AUTH_RECEIPTS_SOURCE;
if(!source)throw Error('Set reviewed AUTH_RECEIPTS_SOURCE');
const load=file=>import(pathToFileURL(join(source,file)).href);
const [{openKeePassBackend},{createKeePassSource},{openNativeBrowserReceiver}]=await Promise.all([load('src/keepass-backend.mjs'),load('src/credential-sources.mjs'),load('src/native-browser-receiver.mjs')]);
const seed='GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ',account='owner@example.test',origin='https://billing.example';
async function fixture({format='legacy',loseSubmit=false,materials=true,staleHeader=false,extraFactor=false}={}){
 let backend,archive;
 const f=await createOwnerAuthFixture({configure:async({root,state,receiving,sourcePort,now})=>{
  state.time=45000;state.password=true;state.factorInputs=0;state.factorSubmits=0;state.providerAccepted=false;state.proofVisible=!loseSubmit;state.focused=false;
  const directory=join(root,'vault');await mkdir(directory,{mode:0o700});
  backend=await openKeePassBackend({directory,password:async()=>'SYNTHETIC-VAULT-ONLY'});const authorize=async()=>true;
  await backend.initialize({authorize});const expectedVersion=await backend.version(),uri=`otpauth://totp/Example:%20${account}?secret=${seed}&issuer=Example`;
  archive=format==='legacy'?await backend.archiveMfa({uri},{authorize,expectedVersion,attemptId:'synthetic-archive'}):await backend.archiveMaterials({fingerprint:'a'.repeat(64),records:[{kind:'mfa',name:'Example',input:{kind:'uri',value:uri}}]},{authorize,expectedVersion,attemptId:'synthetic-archive'});
  if(extraFactor)await backend.archiveMfa({uri:`otpauth://totp/Alternate:${account}?secret=${seed}&issuer=Alternate`},{authorize,expectedVersion:await backend.version(),attemptId:'extra-archive'});
  const box=(x,y,width,height)=>({x,y,width,height});
  const element=(path,role,name,bounds,parentPath)=>({path,role,name,text:name,bounds,parentPath,states:['showing','visible',...(path==='code'&&state.focused?['focused']:[])]});
  const elements=()=>[
   {...element('url','entry','Address and search bar',box(0,0,800,30)),text:origin},
   element('doc','document web','Synthetic provider',box(0,40,800,560)),
   ...(state.password?[element('account','label',account,box(30,110,400,30),'doc'),element('code','password text','Password',box(30,160,300,40),'doc')]:state.providerAccepted&&state.proofVisible?[element('profile','push button',`Account manager for ${account}`,box(450,60,300,30),'doc')]:[
    element('prompt','label','Check your authenticator app',box(30,60,400,40),'doc'),
    element('account','label',account,box(30,110,400,30),'doc'),
    {...element('code','entry','Verification code',box(30,160,300,40),'doc'),text:state.providerCode??''},
    ...(staleHeader?[element('profile','push button',`Account manager for ${account}`,box(450,60,300,30),'doc')]:[]),
   ]),
  ];
  const runtime={async resolve(){return {generation:receiving.generation,resourceId:'synthetic-native-resource'};},async inspect(){return {complete:true,elements:elements()};},async capture(){return {guard:{generation:1}};},async input(_binding,actions){
   if(actions.some(a=>a.kind==='click'))state.focused=true;
   for(const action of actions){
    if(action.kind==='text'){if(state.password){state.password=false;state.focused=false;}else{state.factorInputs++;state.providerCode=action.text;}}
    if(action.kind==='key'&&action.chord==='Return'&&state.providerCode!==undefined){state.factorSubmits++;state.providerAccepted=state.providerCode==='287082';state.authenticated=state.providerAccepted;if(loseSubmit)throw Error('Synthetic lost submission acknowledgement');}
   }
   return {delivery:'dispatched'};
  }};
  const nativeRoot=join(root,'native');await mkdir(nativeRoot,{mode:0o700});
  const native=await openNativeBrowserReceiver({root:nativeRoot,runtime,routes:[{clientId:'client',targetId:'target',profile:'synthetic',desktopId:'synthetic-desktop',desktopGeneration:1}],identityKey:Buffer.alloc(32,9),now,terminalOrigins:[origin]});
  // The fixed RFC code window has no preparation delay in this synthetic desktop.
  return {sources:[{...sourcePort,async version(){return 'synthetic-target-v1';},async catalog(){return [{sourceId:sourcePort.id,recordId:'synthetic:billing',website:origin,account,name:'Synthetic billing'}];}},createKeePassSource({backend})],receiver:native.receiver,...(materials?{materials:{key:Buffer.alloc(32,5),readClock:async()=>({synchronized:true,timeMs:state.time}),minimumRemainingMs:0,totpPreparationMs:0}}:{})};
 }});
 async function preceding(auth,key){
  f.state.password=true;f.state.focused=false;f.state.providerAccepted=false;f.state.authenticated=false;delete f.state.providerCode;
  const found=await auth.helper('discover_credentials',{query:f.options.query,accountHint:account});
  const original=await auth.requestLogin({sourceId:found.selected.sourceId,recordId:found.selected.recordId,origin,targetId:'target',purpose:'Synthetic preceding password',expiresAt:f.options.expiresAt,idempotencyKey:key});
  f.respond();const attempted=await auth.advanceLogin({requestId:original.requestId});assert.ok(attempted.attemptId);assert.equal(attempted.state,'unknown');
  return original.requestId;
 }
 try{f.options.originalRequestId=await preceding(f.auth,'preceding-password');return {...f,backend,archive,preceding};}catch(error){await f.close();throw error;}
}

for(const [format,staleHeader] of [['legacy',false],['material',false],['legacy',true]]){
 const f=await fixture({format,staleHeader});
 try{
  const before=await f.backend.version(),flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const pending=await flow.advance(f.options);
  assert.equal(pending.reasonCode,'approval_required');assert.equal(f.state.factorInputs,0);assert.equal(f.state.reads,1);
  const request=await f.auth.helper('inspect_credential_request',{requestId:pending.requestId});
  assert.equal(request.stagedFactor.status,'validation_required');assert.equal(request.stagedFactor.account,account);assert.deepEqual(request.approvalChoices,{standing:false,durations:[]});
  f.respond();const complete=await flow.advance(f.options);
  assert.equal(complete.status,'completed');assert.equal(f.state.factorInputs,1);assert.equal(f.state.factorSubmits,1);
  const done=await f.auth.helper('inspect_credential_request',{requestId:pending.requestId});assert.equal(done.stagedFactor.status,'active');assert.equal(done.authentication,'verified');
  assert.equal(await f.backend.version(),before,'Archival bytes stay unchanged');
  const fresh=await f.connectClient('synthetic-fresh-task'),originalRequestId=await f.preceding(fresh,'fresh-password');
  const need=await fresh.helper('observe_authentication',{origin,targetId:'target',originalRequestId});
  assert.equal(need.candidates.length,1);assert.equal(need.candidates[0].recordId,done.recordId);assert.notEqual(need.candidates[0].staged,true);
  const ordinary=await fresh.helper('continue_authentication',{...need.nextAction.args,recordId:done.recordId,idempotencyKey:'ordinary-use',purpose:'Synthetic later normal use'});
  assert.equal(ordinary.state,'pending');assert.equal(ordinary.stagedFactor,undefined);assert.equal(ordinary.recordId,done.recordId);
  assert.equal(f.state.factorInputs,1);assert.equal(f.state.factorSubmits,1);
  assert.equal(JSON.stringify({pending,complete,done}).includes(seed),false);assert.equal(JSON.stringify({pending,complete,done}).includes('287082'),false);
 }finally{await f.close();}
}
console.log('Normal owner helper binds and privately validates both archived MFA formats through actual Auth and native receiver');

{
 const f=await fixture({loseSubmit:true});
 try{
  let flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const pending=await flow.advance(f.options);f.respond();const unknown=await flow.advance(f.options);
  assert.equal(unknown.effectState,'unknown');assert.equal(f.state.factorInputs,1);assert.equal(f.state.factorSubmits,1);
  const original=await f.auth.helper('inspect_credential_request',{requestId:pending.requestId});assert.equal(original.authentication,undefined);
  await f.restart();f.state.proofVisible=true;flow=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints});
  const recovered=await flow.advance(f.options);assert.equal(recovered.status,'completed');assert.equal(recovered.requestId,pending.requestId);
  const after=await f.auth.helper('inspect_credential_request',{requestId:pending.requestId});assert.equal(after.attemptId,original.attemptId);assert.equal(after.stagedFactor.status,'active');
  assert.equal(f.state.factorInputs,1);assert.equal(f.state.factorSubmits,1);
 }finally{await f.close();}
}
console.log('Lost native validation acknowledgement recovers the original helper request without another code or submit');

{
 const f=await fixture({extraFactor:true});
 try{
  const auth={callerTaskId:f.auth.callerTaskId,async helper(operation,args){const value=await f.auth.helper(operation,args);return operation==='observe_authentication'?{...value,candidates:value.candidates.map(c=>({...c,seed,unreviewedField:'SYNTHETIC-EXTRA'}))}:value;}};
  const flow=createOwnerTotpContinuation({auth,browser:f.browser,checkpoints:f.checkpoints}),choice=await flow.advance(f.options);
  assert.equal(choice.reasonCode,'factor_choice_required');assert.equal(choice.candidates.length,2);
  assert.deepEqual(choice.candidates.map(c=>c.issuer).sort(),['Alternate','Example']);
  assert.ok(choice.candidates.every(c=>c.account===account&&c.method==='totp'&&c.staged===true));
  assert.equal(JSON.stringify(choice).includes(seed),false);assert.equal(JSON.stringify(choice).includes('SYNTHETIC-EXTRA'),false);assert.equal(f.state.factorInputs,0);
  const recordId=choice.candidates.find(c=>c.issuer==='Example').recordId;await f.restart();
  const resumed=createOwnerTotpContinuation({auth:f.auth,browser:f.browser,checkpoints:f.checkpoints}),options={...f.options,recordId};
  const pending=await resumed.advance(options);assert.equal(pending.reasonCode,'approval_required');
  f.respond();const complete=await resumed.advance(options);assert.equal(complete.status,'completed');
  const selected=await f.auth.helper('inspect_credential_request',{requestId:pending.requestId});assert.equal(selected.recordId,recordId);assert.equal(f.state.factorInputs,1);
 }finally{await f.close();}
}
console.log('Ambiguous archived authenticators retain issuer/account labels for one explicit choice');
