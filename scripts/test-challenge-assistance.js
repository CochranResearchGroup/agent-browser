import assert from 'node:assert/strict';
import {createChallengeAssistanceClient} from '../packages/client/src/challenge-assistance.js';
const binding={taskId:'task-a',profileId:'profile-a',browserId:'browser-a',tabId:'tab-a',targetId:'target-a'};
let owner='agent',epoch=2,handoffCalls=0,continues=0,allowAccess=false;
const calls=[];
const payload=data=>({content:[{type:'text',text:JSON.stringify({success:true,data})}]});
const frame={source:'remote_view_native',capture:{desktopId:'desktop-a',generation:9,windowId:7,processIdentity:{pid:42,start_token:'original'},capture:{sha256:'a'.repeat(64)}},
  challenge:{corpusVersion:'native-challenge-corpus.v1',kind:'image_puzzle',frameSha256:'a'.repeat(64),automatedAttemptAuthorized:false},evidenceAgeMs:10};
const client=createChallengeAssistanceClient({
  callTool:async(name,args)=>{
    calls.push({name,args});
    if(name==='browser_command'&&args.action==='task_briefing')return payload({...binding,control:{owner,epoch}});
    if(name==='browser_command'&&args.action==='native_capture')return payload(frame);
    if(name==='browser_handoff'){handoffCalls++;return payload({taskId:'task-a',handoffId:'opaque-a',handoffUrl:'https://native.example.test/remote-view/opaque-a',operatorVisible:{state:'ready'}});}
    if(name==='browser_control')return payload({taskId:'task-a',browserId:'browser-a',control:{owner,epoch}});
    if(name==='browser_continue'){continues++;return payload(binding);}
    throw Error('Unsupported public tool');
  },
  verifyClearance:async({taskId,frameSha256})=>({state:'verified',taskId,frameSha256,evidenceRef:'independent-clearance'}),
  verifyAccess:async({taskId,frameSha256})=>({state:allowAccess?'verified':'unverified',taskId,frameSha256,evidenceRef:'independent-protected-access'}),
});
const prepared=await client.prepare({taskId:'task-a'});
assert.equal(prepared.status,'awaiting_operator');
assert.equal(prepared.handoffUrl,'https://native.example.test/remote-view/opaque-a');
assert.equal(owner,'agent','Viewing is not takeover');
assert.equal(handoffCalls,1);
owner='human';epoch=3;
const human=await client.recordTakeover(prepared);
assert.equal(human.status,'operator_has_control');
assert.equal(continues,0);
owner='agent';epoch=4;
const denied=await client.continueAfterReturn(human);
assert.equal(denied.status,'intervention');
assert.equal(denied.reason,'site_access_unverified');
assert.equal(continues,0,'Challenge clearance cannot substitute for protected-site access');
allowAccess=true;
const continued=await client.continueAfterReturn(human);
assert.equal(continued.status,'verified_access_after_return');
assert.equal(continues,1);
assert.equal(handoffCalls,1,'Return never requests another viewer/handoff');
assert(calls.filter(c=>c.name==='browser_command'&&c.args.action==='native_capture').every(c=>c.args.params.classifyChallenges===true));
console.log('Challenge assistance: exact public handoff, explicit control and independent admission pass');
