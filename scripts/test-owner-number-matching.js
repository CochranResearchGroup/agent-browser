import assert from 'node:assert/strict';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';
import {createOwnerLoginPlaybook} from '../packages/client/src/owner-login-playbook.js';

async function fixture(direction='browser_to_device',settings={}){
 const source=process.env.AUTH_RECEIPTS_SOURCE,parent=process.env.RECEIPTS_SOURCE;
 assert.ok(source&&parent,'Set reviewed Auth and parent source roots');
 const load=(root,file)=>import(pathToFileURL(join(root,file)).href);
 const [{PNG},{createReceiptsParentAdapter},{openNotificationRelay}]=await Promise.all([
  load(source,'node_modules/pngjs/lib/png.js'),load(source,'src/receipts-parent.mjs'),load(parent,'apps/receipts-interoperation/src/notifications.mjs')]);
 const image=PNG.sync.write({width:1,height:1,data:Buffer.from([0,0,0,255])}),sha256=createHash('sha256').update(image).digest('hex');
 let relay;
 const f=await createOwnerAuthFixture({configure:async({root,state,receiving,receiver,sourcePort,now})=>{
  Object.assign(state,{direction,number:direction==='browser_to_device'?'42':null,challengeAccount:'owner@example.test',observed:true,safeCapture:true,challengeExpiry:state.time+45000,texts:0,uploads:0,receipts:0,reverseInputs:0,imageObserved:true,sends:0,cancelReady:true,...settings});
  sourcePort.discover=async()=>[{recordId:'synthetic:billing',origin:'https://accounts.google.com',name:'Google',account:'owner@example.test',modifiedAt:1}];
  receiver.use=async({authorize})=>{await authorize();state.inputs++;return {released:true};};
  receiver.reconcile=async()=>state.authenticated?{released:true,authenticated:true}:{released:true};
  receiver.observeChallenge=async({request})=>({state:state.observed?'number_matching':'none',observationId:'synthetic-number',observedAt:state.time,
   binding:{...Object.fromEntries(['requestId','attemptId','clientId','taskId','sourceId','recordId','origin','targetId'].map(k=>[k,request[k]])),generation:receiving.generation},
   provider:'google',account:state.challengeAccount,direction:state.direction,number:state.number,expiresAt:state.challengeExpiry,captureSafety:state.safeCapture?'challenge_only':'uncertain',screenshot:{contentType:'image/png',contentBase64:image.toString('base64')}});
  receiver.respondChallenge=async({authorize})=>{await authorize();state.reverseInputs++;return {};};
  const imageAction=(id)=>({id,status:'sent',receipt:{action_id:id,workspace:'synthetic',channel_id:'synthetic',file_id:'file-'+id,content_sha256:sha256,provider_accepted_at:'2026-01-01T00:00:00Z',workspace_observed:state.imageObserved,...(state.imageObserved?{workspace_observed_at:'2026-01-01T00:00:01Z',observed_event_id:'observed-1'}:{})}});
  relay=await openNotificationRelay({directory:join(root,'number-relay'),ownerId:'owner',producerId:'auth-receipts',route:{baseUrl:'http://127.0.0.1:9999',workspace:'synthetic',channelRef:'synthetic'},now,fetch:async(url,request)=>{
   if(request.method==='GET'){state.receipts++;return Response.json({ok:true,action:imageAction(decodeURIComponent(url.split('/').at(-1)))});}
   state.sends++;const body=JSON.parse(request.body);
   if(url.endsWith('/files')){state.uploads++;return Response.json({ok:true,action:imageAction('image-'+state.uploads)});}
   if(body.text?.includes('number'))state.texts++;
   return Response.json({ok:true,action:{id:'text-'+state.sends,status:'sent'}});
  }});
  const notifications=createReceiptsParentAdapter({relay});
  return {parent:{...notifications,async cancel(eventId){if(!state.cancelReady)throw Error('Synthetic lost cancellation acknowledgement');return notifications.cancel(eventId);}}};
 }});
 Object.assign(f.options,{origin:'https://accounts.google.com',query:'google.com',guideId:'google'});
 await f.ownerCall('put_credential_rule',{rule:{id:'owner-number-permission',effect:'allow',clients:['client'],operations:['login'],modes:['private'],approval:'standing',notificationGate:'delivered'}});
 const login=createOwnerLoginPlaybook(f),used=await login.advance(f.options);
 assert.equal(used.reasonCode,'authentication_required');assert.equal(f.state.reads,1);assert.equal(f.state.inputs,1);
 return {...f,requestId:used.requestId,async close(){await f.close();relay.close();}};
}

const {createOwnerNumberMatchingContinuation}=await import('../packages/client/src/owner-number-matching.js');
const first=await fixture();
try{
 const continuation=createOwnerNumberMatchingContinuation(first),options={...first.options,requestId:first.requestId};
 const waiting=await continuation.advance(options);
 assert.equal(waiting.reasonCode,'number_matching_owner_required');assert.equal(waiting.challenge.direction,'browser_to_device');
 assert.equal(waiting.challenge.textDelivered,true);assert.equal(waiting.challenge.screenshotDelivered,true);assert.equal(waiting.challenge.number,'42');
 first.state.authenticated=true;
 const done=await continuation.advance(options);assert.equal(done.status,'completed',JSON.stringify(done));assert.equal(done.effectState,'verified');
 assert.equal(first.state.reads,1);assert.equal(first.state.inputs,1);assert.equal(first.state.uploads,1);
 assert.ok(!JSON.stringify({waiting,done}).includes('contentBase64'));assert.ok(!JSON.stringify({waiting,done}).includes('SYNTHETIC-PASSWORD-ONLY'));
}finally{await first.close();}
console.log('Owner number matching follows the original Auth attempt and independently verifies completion');

const reverse=await fixture('device_to_browser');
try{
 const options={...reverse.options,requestId:reverse.requestId};
 let continuation=createOwnerNumberMatchingContinuation(reverse);
 const waiting=await continuation.advance(options);assert.equal(waiting.reasonCode,'number_matching_owner_required');
 assert.equal(waiting.challenge.direction,'device_to_browser');assert.equal(waiting.challenge.number,null);
 await assert.rejects(continuation.advance({...options,number:'42'}),/owner_number_matching_options_invalid/);
 const input={requestId:reverse.requestId,challengeId:waiting.challenge.challengeId,idempotencyKey:'synthetic-owner-response',number:'42'};
 await assert.rejects(reverse.auth.helper('respond_credential_challenge',input),{code:'auth_not_authorized'});
 assert.equal(reverse.state.reverseInputs,0);
 assert.equal((await reverse.ownerCall('respond_credential_challenge',input)).responseState,'unknown');
 await reverse.restart();continuation=createOwnerNumberMatchingContinuation(reverse);
 const unknown=await continuation.advance(options);assert.equal(unknown.reasonCode,'number_matching_response_unknown');
 assert.equal((await reverse.ownerCall('respond_credential_challenge',input)).responseState,'unknown');
 assert.equal(reverse.state.reverseInputs,1);assert.equal(reverse.state.inputs,1);assert.equal(reverse.state.reads,1);assert.equal(reverse.state.uploads,1);
 reverse.state.authenticated=true;
 assert.equal((await continuation.advance(options)).status,'completed');assert.equal(reverse.state.reverseInputs,1);
}finally{await reverse.close();}
console.log('Only the owner may enter a reverse number; original unknown response survives restart without replay');

const failed=await fixture();
try{
 failed.receiver.reconcile=async()=>({released:true,authenticated:false});
 const result=await createOwnerNumberMatchingContinuation(failed).advance({...failed.options,requestId:failed.requestId});
 assert.equal(result.status,'failed');assert.equal(result.reasonCode,'authentication_failed');
 assert.equal(failed.state.reads,1);assert.equal(failed.state.inputs,1);assert.equal(failed.state.uploads,1);
}finally{await failed.close();}

const partial=await fixture('browser_to_device',{imageObserved:false});
try{
 const options={...partial.options,requestId:partial.requestId};
 const pending=await createOwnerNumberMatchingContinuation(partial).advance(options);
 assert.equal(pending.reasonCode,'number_matching_delivery_required');assert.equal(pending.challenge.textDelivered,true);assert.equal(pending.challenge.screenshotDelivered,false);
 const sends=partial.state.sends;await partial.restart();partial.state.imageObserved=true;
 const recovered=await createOwnerNumberMatchingContinuation(partial).advance(options);
 assert.equal(recovered.challenge.screenshotDelivered,true);assert.equal(recovered.challenge.challengeId,pending.challenge.challengeId);
 assert.equal(partial.state.uploads,1);assert.equal(partial.state.sends,sends);assert.ok(partial.state.receipts>0);assert.equal(partial.state.inputs,1);
}finally{await partial.close();}

const unsafe=await fixture('browser_to_device',{safeCapture:false});
try{
 const result=await createOwnerNumberMatchingContinuation(unsafe).advance({...unsafe.options,requestId:unsafe.requestId});
 assert.equal(result.reasonCode,'number_matching_safe_capture');assert.equal(unsafe.state.uploads,0);assert.equal(unsafe.state.texts,0);
}finally{await unsafe.close();}

const wrongAccount=await fixture('browser_to_device',{challengeAccount:'other@example.test'});
try{
 const result=await createOwnerNumberMatchingContinuation(wrongAccount).advance({...wrongAccount.options,requestId:wrongAccount.requestId});
 assert.equal(result.reasonCode,'challenge_account_mismatch');assert.equal(result.status,'adaptation_required');assert.equal(wrongAccount.state.inputs,1);
}finally{await wrongAccount.close();}

const generation=await fixture();
try{
 const sends=generation.state.sends;
 generation.browser.verifyLogin=async()=>({...generation.options,generation:'replacement',status:'waiting',reasonCode:'mfa_required'});
 const result=await createOwnerNumberMatchingContinuation(generation).advance({...generation.options,requestId:generation.requestId});
 assert.equal(result.reasonCode,'task_target_changed');assert.equal(result.status,'adaptation_required');assert.equal(generation.state.sends,sends);
}finally{await generation.close();}

const company=await fixture();
try{
 company.state.authenticated=true;company.state.verificationOrganization='another-company';
 const options={...company.options,requestId:company.requestId},continuation=createOwnerNumberMatchingContinuation(company);
 assert.notEqual((await continuation.advance(options)).status,'completed');
 company.state.verificationOrganization='company';assert.equal((await continuation.advance(options)).status,'completed');assert.equal(company.state.inputs,1);
}finally{await company.close();}
console.log('Independent image receipts, safe capture, account, generation and company evidence gate continuation');

const lifecycle=await fixture();
try{
 const options={...lifecycle.options,requestId:lifecycle.requestId},continuation=createOwnerNumberMatchingContinuation(lifecycle);
 const first=await continuation.advance(options);lifecycle.state.number='43';
 const replacement=await continuation.advance(options);
 assert.notEqual(replacement.challenge.challengeId,first.challenge.challengeId);assert.equal(replacement.challenge.number,'43');
 assert.equal(lifecycle.state.uploads,2);assert.equal(lifecycle.state.reads,1);assert.equal(lifecycle.state.inputs,1);
 await lifecycle.auth.helper('cancel_credential_request',{requestId:lifecycle.requestId});
 const sends=lifecycle.state.sends;await lifecycle.restart();
 const cancelled=await createOwnerNumberMatchingContinuation(lifecycle).advance(options);
 assert.equal(cancelled.status,'failed');assert.equal(cancelled.reasonCode,'auth_cancelled');assert.equal(cancelled.challenge.status,'cancelled');
 assert.equal(lifecycle.state.sends,sends);assert.equal(lifecycle.state.uploads,2);
}finally{await lifecycle.close();}

const expiry=await fixture();
try{
 const options={...expiry.options,requestId:expiry.requestId};expiry.state.time=expiry.state.challengeExpiry;
 const result=await createOwnerNumberMatchingContinuation(expiry).advance(options);
 assert.equal(result.status,'failed');assert.equal(result.reasonCode,'number_matching_expired');assert.equal(expiry.state.uploads,1);assert.equal(expiry.state.inputs,1);
}finally{await expiry.close();}

const navigation=await fixture();
try{
 const options={...navigation.options,requestId:navigation.requestId};navigation.state.observed=false;
 const result=await createOwnerNumberMatchingContinuation(navigation).advance(options);
 assert.equal(result.reasonCode,'number_matching_navigated');const sends=navigation.state.sends;
 await navigation.restart();navigation.state.observed=true;
 assert.notEqual((await createOwnerNumberMatchingContinuation(navigation).advance(options)).status,'completed');assert.equal(navigation.state.sends,sends);assert.equal(navigation.state.uploads,1);
}finally{await navigation.close();}

const unsupported=await fixture('browser_to_device',{observed:false});
try{
 const result=await createOwnerNumberMatchingContinuation(unsupported).advance({...unsupported.options,requestId:unsupported.requestId});
 assert.equal(result.status,'waiting');assert.equal(result.reasonCode,'number_matching_no_challenge');assert.equal(unsupported.state.uploads,0);
}finally{await unsupported.close();}
console.log('Supersession, cancellation, expiry and navigation retain original authority without replay or stale prompt resurrection');

const terminalReceipt=await fixture('browser_to_device',{imageObserved:false});
try{
 const options={...terminalReceipt.options,requestId:terminalReceipt.requestId};
 const pending=await createOwnerNumberMatchingContinuation(terminalReceipt).advance(options);assert.equal(pending.challenge.screenshotDelivered,false);
 terminalReceipt.state.cancelReady=false;
 await terminalReceipt.auth.helper('cancel_credential_request',{requestId:terminalReceipt.requestId});
 const held=await createOwnerNumberMatchingContinuation(terminalReceipt).advance(options);assert.equal(held.reasonCode,'number_matching_cancellation_required');
 const sends=terminalReceipt.state.sends;
 await terminalReceipt.restart();terminalReceipt.state.cancelReady=true;terminalReceipt.state.imageObserved=true;
 const retired=await createOwnerNumberMatchingContinuation(terminalReceipt).advance(options);
 assert.equal(retired.reasonCode,'auth_cancelled');assert.equal(retired.challenge.challengeId,pending.challenge.challengeId);assert.equal(retired.challenge.screenshotDelivered,true);
 assert.equal(terminalReceipt.state.sends,sends);assert.equal(terminalReceipt.state.uploads,1);assert.equal(terminalReceipt.state.inputs,1);
}finally{await terminalReceipt.close();}
console.log('Unknown cancellation and terminal image receipts reconcile the original event without another send');
