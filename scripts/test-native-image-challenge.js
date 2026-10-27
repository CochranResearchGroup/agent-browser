import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {deflateSync} from 'node:zlib';
import {createNativeImageChallengeClient} from '../packages/client/src/native-image-challenge.js';
function png(color) {
  const crc=bytes=>{let value=0xffffffff;for(const byte of bytes){value^=byte;for(let bit=0;bit<8;bit++)value=value&1?0xedb88320^(value>>>1):value>>>1;}return (value^0xffffffff)>>>0;};
  const chunk=(name,data)=>{const type=Buffer.from(name),size=Buffer.alloc(4),checksum=Buffer.alloc(4);size.writeUInt32BE(data.length);checksum.writeUInt32BE(crc(Buffer.concat([type,data])));return Buffer.concat([size,type,data,checksum]);};
  const header=Buffer.alloc(13);header.writeUInt32BE(640);header.writeUInt32BE(480,4);header[8]=8;header[9]=2;
  const pixels=Buffer.alloc((640*3+1)*480,color);for(let row=0;row<480;row++)pixels[row*(640*3+1)]=0;
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',deflateSync(pixels)),chunk('IEND',Buffer.alloc(0))]);
}
const beforePng=png(255),afterPng=png(250),sha=createHash('sha256').update(beforePng).digest('hex'),afterSha=createHash('sha256').update(afterPng).digest('hex'),puzzle='c'.repeat(64),now=100000;
const identity={taskId:'task',profileId:'profile',browserId:'browser',tabId:'tab',targetId:'target'};
const binding={...identity,control:{owner:'agent',epoch:1}};
const observation={desktopId:'desktop',generation:9,windowId:7,processDigest:'d'.repeat(64),window:{x:0,y:0,width:640,height:480},displayWidth:640,displayHeight:480,region:{x:0,y:0,width:640,height:480},frameSha256:sha,nativeControlEpoch:4,observedAtMs:now,registeredActionId:'hcaptcha-image-grid-3x3-v1',puzzleSha256:puzzle,candidates:Array.from({length:9},(_,index)=>({candidateId:`tile:${index+1}:${sha.slice(0,16)}`,bounds:{x:100+index%3*66,y:200+Math.floor(index/3)*66,width:64,height:64}})),verify:{x:100,y:414,width:80,height:30}};
const provider={id:'image-provider',kind:'captcha',enabled:true,configRef:'agent-browser:remote-view-native-image-static-3x3:v1',capabilities:['captcha_solve']};
const policy={taskId:'task',binding:identity,sourceRef:'owned-config',origin:'https://owned.example.test',profile:{id:'profile',sitePolicyIds:['site']},sitePolicy:{id:'site',originPattern:'https://owned.example.test',origins:[],challengePolicy:'provider_allowed',allowedChallengeProviders:[provider.id]},provider,taskPolicy:{taskId:'task',hostId:'host',revision:1,policy:null}};
function fixture(options={}) {
  const calls=[];let operation=null,inputs=0,selected=0;
  const payload=(data,images=[])=>({content:[{type:'text',text:JSON.stringify({success:true,data})},...images]});
  const frame=after=>({source:'remote_view_native',partialWindow:false,windowBounds:observation.window,capture:{desktopId:'desktop',generation:9,windowId:7,processIdentity:{pid:42,start_token:'owned'},region:observation.region,capture:{sha256:after?afterSha:sha,width:640,height:480}},challenge:{kind:after?'unknown':'image_puzzle',family:after?null:'hcaptcha',frameSha256:after?afterSha:sha,corpusVersion:'native-challenge-corpus.v1',completeWindow:true,automatedAttemptAuthorized:false,minimumOcrConfidence:70},nativeControlEpoch:4,observedAtMs:now,evidenceAgeMs:1,...(!after?{nativeImageObservation:{...structuredClone(observation),observedAtMs:options.nearExpiry?now-59999:now}}:{})});
  const client=createNativeImageChallengeClient({now:()=>now,callTool:async(name,args)=>{
    calls.push({name,args});
    if(name==='browser_command'&&args.action==='task_operation'){if(!operation)throw {code:'browser_task_operation_not_found'};return payload({operation});}
    if(name==='browser_command'&&args.action==='task_briefing')return payload(options.changedControl&&selected>0?{...binding,control:{owner:'agent',epoch:2}}:binding);
    if(name==='browser_command'&&args.action==='native_capture')return payload(frame(inputs>0),[{type:'image',mimeType:'image/png',data:(inputs>0||options.badImage?afterPng:beforePng).toString('base64')}]);
    if(name==='browser_command'&&args.action==='native_input'){
      inputs++;assert.deepEqual(args.params,{imageAttempt:{observationRequestId:'attempt:observation',selectedCandidateIds:[observation.candidates[0].candidateId,observation.candidates[2].candidateId]}});
      operation={...identity,requestId:'attempt',action:'native_input',state:options.lostReply?'uncertain':'owner_acknowledged',retryAllowed:false};
      if(options.lostReply)throw Error('Private provider canary /private/path SELECT password');
      return payload({source:'remote_view_native',delivery:'dispatched',taskOperation:operation});
    }
    if(name==='browser_continue')return payload({...identity,url:'https://owned.example.test/report'});
    throw Error('Unexpected protocol call '+name+'/'+args.action);
  },resolvePolicy:async()=>structuredClone(policy),selectCandidates:async context=>{
    selected++;assert.equal(context.observation.frameSha256,sha);assert.equal(context.images.length,1);
    return {...identity,observationRequestId:context.observationRequestId,frameSha256:sha,puzzleSha256:puzzle,selectedCandidateIds:[observation.candidates[0].candidateId,observation.candidates[2].candidateId]};
  },verifyClearance:async context=>({state:'verified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'independent-clearance'}),verifyAccess:async context=>({state:'verified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'independent-site-access'}),...options});
  return {client,calls,get inputs(){return inputs},get selected(){return selected}};
}
const f=fixture();const result=await f.client.resolve({taskId:'task',requestId:'attempt'});
assert.equal(result.status,'verified_access_after_native_image');assert.equal(f.inputs,1);assert.equal(f.selected,1);assert.equal(result.originalOperation.requestId,'attempt');assert.equal(result.retryAllowed,false);
console.log('Native image client public protocol PASS (provider-free test double; not visual/model qualification)');
for(const change of ['wrong_frame','unknown_id','duplicate','too_many','empty']) {
  const selectedFixture=fixture({selectCandidates:async context=>({taskId:'task',observationRequestId:context.observationRequestId,frameSha256:change==='wrong_frame'?'e'.repeat(64):sha,puzzleSha256:puzzle,selectedCandidateIds:change==='unknown_id'?['foreign']:change==='duplicate'?[observation.candidates[0].candidateId,observation.candidates[0].candidateId]:change==='too_many'?observation.candidates.slice(0,5).map(c=>c.candidateId):change==='empty'?[]:[observation.candidates[0].candidateId]})});
  const refused=await selectedFixture.client.resolve({taskId:'task',requestId:'attempt'});assert.notEqual(refused.status,'verified_access_after_native_image',change);assert.equal(selectedFixture.inputs,0,change);
}
const wrongPolicy=structuredClone(policy);wrongPolicy.provider.configRef='agent-browser:remote-view-native:v1';
const checkboxOnly=fixture({resolvePolicy:async()=>wrongPolicy});await checkboxOnly.client.resolve({taskId:'task',requestId:'attempt'});assert.equal(checkboxOnly.inputs,0);assert.equal(checkboxOnly.selected,0);
for(const membership of [[],['foreign-provider']]){const deniedPolicy=structuredClone(policy);deniedPolicy.sitePolicy.allowedChallengeProviders=membership;const denied=fixture({resolvePolicy:async()=>deniedPolicy});await denied.client.resolve({taskId:'task',requestId:'attempt'});assert.equal(denied.inputs,0);assert.equal(denied.selected,0);}
for(const options of [{badImage:true},{changedControl:true}]){const denied=fixture(options);await denied.client.resolve({taskId:'task',requestId:'attempt'});assert.equal(denied.inputs,0);}
const access=fixture({verifyAccess:async context=>({state:'unverified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'denied-site'})});
assert.notEqual((await access.client.resolve({taskId:'task',requestId:'attempt'})).status,'verified_access_after_native_image');assert.equal(access.inputs,1);assert(!access.calls.some(call=>call.name==='browser_continue'));
const lost=fixture({lostReply:true});const unknown=await lost.client.resolve({taskId:'task',requestId:'attempt'});assert.equal(lost.inputs,1);assert.equal(unknown.originalOperation.state,'uncertain');assert(!unknown.reason.includes('canary'));assert.equal(unknown.retryAllowed,false);
await lost.client.resolve({taskId:'task',requestId:'attempt'});assert.equal(lost.inputs,1);assert.equal(lost.selected,1);assert(!lost.calls.some(call=>call.name==='browser_continue'));
console.log('Native image client image binding, policy, selection, access and uncertain no-replay negatives PASS');
const timeout=fixture({nearExpiry:true,selectCandidates:async()=>new Promise(()=>{})});const timed=await timeout.client.resolve({taskId:'task',requestId:'attempt'});assert.equal(timed.reason,'native_image_selection_timeout');assert.equal(timeout.inputs,0);
console.log('Native image selection port deadline refuses without input PASS');
