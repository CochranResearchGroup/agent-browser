import assert from 'node:assert/strict';
import {createChallengeAssistanceClient} from '../packages/client/src/challenge-assistance.js';
const binding={taskId:'task',profileId:'browser-profile',browserId:'browser',tabId:'tab',targetId:'target'};
const families=['turnstile-checkbox-p169-v1','hcaptcha-checkbox-p181-v2'];
const payload=data=>({content:[{type:'text',text:JSON.stringify({success:true,data})}]});
async function scenario(condition,family=families[1]) {
  let clock=1000,epoch=2,captures=0,inputs=0,policyReads=0,accessReads=0,clearanceReads=0;
  const originals=new Map(),calls=[];
  // Receiving-app source records, not a caller-provided allow/authorized label.
  const sitePolicy={id:'owned-site',originPattern:'https://owned.example.test',origins:[],
    challengePolicy:condition==='manual'?'manual_only':condition==='passive'?'avoid_first':'provider_allowed',allowedChallengeProviders:[condition==='provider_denied'?'other-provider':'native-fixture-provider']};
  const provider={id:'native-fixture-provider',kind:'captcha',enabled:condition!=='provider_disabled',configRef:condition==='wrong_backend'?'vendor-config':'agent-browser:remote-view-native:v1',capabilities:condition==='missing_capability'?[]:['captcha_solve']};
  const profile={id:binding.profileId,sitePolicyIds:['owned-site'],manualLoginPreferred:condition==='profile_manual'};
  sitePolicy.requiresCdpFree=condition==='requires_cdp_free';sitePolicy.interactionMode=condition==='manual_mode'?'manual':'cdp_direct';sitePolicy.manualLoginPreferred=condition==='manual_login';
  const taskPolicy={version:1,deniedActions:condition==='policy_denied'?['native_input']:[],artifactDestinations:{}};
  async function tool(name,args) {
    calls.push({name,args});
    if(name==='browser_policy')return payload({taskId:'task',hostId:'owned-app',revision:condition==='policy_changed'&&policyReads++>0?2:1,policy:taskPolicy});
    if(name==='browser_command'&&args.action==='task_briefing')return payload({...binding,control:{owner:'agent',epoch}});
    if(name==='browser_command'&&args.action==='task_operation') {
      const original=originals.get(args.params.requestId);
      if(!original&&condition==='rpc_not_found')throw Object.assign(Error('Invalid params'),{data:{message:'browser_task_operation_not_found'}});
      return original?payload({operation:original}):{isError:true,content:[{type:'text',text:'{"success":false,"error":{"code":"browser_task_operation_not_found"}}'}]};
    }
    if(name==='browser_command'&&args.action==='native_capture') {
      captures++;
      const kind=condition==='image'?'image_puzzle':condition==='passive'?(captures===1?'passive_check':'unknown'):(inputs?'unknown':'checkbox');
      const frame={source:'remote_view_native',nativeControlEpoch:4,capture:{desktopId:'desktop',generation:9,windowId:7,processIdentity:{pid:42,start_token:'original'},capture:{sha256:'a'.repeat(64)}},
        challenge:{corpusVersion:'native-challenge-corpus.v1',kind,family:family.startsWith('turnstile')?'turnstile':'hcaptcha',registeredProfileId:family,frameSha256:'a'.repeat(64),automatedAttemptAuthorized:false},evidenceAgeMs:condition==='stale'?10001:10,observedAtMs:clock};
      if(args.params.registeredActionId) {
        assert.equal(args.params.classifyChallenges,true);
        assert(args.requestId,'Capture original must be retained with stable ID');
        frame.nativeObservation={registeredActionId:family,desktopId:'desktop',generation:9,windowId:7,frameSha256:'a'.repeat(64),nativeControlEpoch:4,observedAtMs:clock};
        if(condition==='control_changed')epoch++;
      }
      return payload(frame);
    }
    if(name==='browser_command'&&args.action==='native_input') {
      inputs++;assert.equal(inputs,1,'No blind retry');
      assert.equal(args.params.actions,undefined,'Caller supplies no guessed point');
      assert.equal(args.params.nativeAttempt.registeredActionId,family);
      assert.equal(args.params.nativeAttempt.observationRequestId,args.requestId+':observation');
      const original={...binding,requestId:args.requestId,action:'native_input',state:condition==='uncertain'?'uncertain':'owner_acknowledged',retryAllowed:false,evidenceReferences:{nativeDelivery:'dispatched',nativeSitePolicy:JSON.stringify({providerId:provider.id,sitePolicyId:sitePolicy.id,registeredActionId:family,family:family.startsWith('turnstile')?'turnstile':'hcaptcha',digest:'b'.repeat(64)})}};
      originals.set(args.requestId,original);
      if(condition==='uncertain')return {isError:true,content:[{type:'text',text:'{"success":false,"error":{"code":"browser_task_operation_uncertain"}}'}]};
      return payload({source:'remote_view_native',delivery:'dispatched',taskCompleted:false,taskOperation:original});
    }
    if(name==='browser_handoff')return payload({taskId:'task',handoffId:'same',handoffUrl:'https://native.example.test/remote-view/same',operatorVisible:{state:'ready'}});
    if(name==='browser_continue')return payload(binding);
    throw Error('Unexpected tool '+name+' '+args.action);
  }
  const client=createChallengeAssistanceClient({callTool:tool,now:()=>clock,wait:async ms=>{clock+=ms;},
    resolvePolicy:async context=>({taskId:context.taskId,binding,origin:'https://owned.example.test',sitePolicy,provider,profile,
      taskPolicy:(JSON.parse((await tool('browser_policy',{taskId:'task',operation:'status'})).content[0].text)).data,sourceRef:'owned-app-current-policy'}),
    verifyClearance:async context=>{clearanceReads++;return {state:condition==='clearance_denied'?'unverified':'verified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'https-server-clearance'};},
    verifyAccess:async context=>{accessReads++;return {state:condition==='access_denied'?'unverified':'verified',taskId:context.taskId,frameSha256:context.frameSha256,evidenceRef:'https-server-protected-report'};},
  });
  const result=await client.resolve({taskId:'task',requestId:'original-attempt'});
  if(['success','passive','manual_login','profile_manual','rpc_not_found'].includes(condition)) {
    assert.equal(result.status,condition==='passive'?'verified_access_after_passive_wait':'verified_access_after_native');
    assert.equal(result.taskCompleted,false);assert(clearanceReads>0&&accessReads>0);
    assert.equal(inputs,condition==='passive'?0:1);
  } else {
    assert(['awaiting_operator','intervention'].includes(result.status),JSON.stringify(result));
    assert.equal(result.retryAllowed,false);
    assert.equal(inputs,['access_denied','clearance_denied','uncertain'].includes(condition)?1:0);
  }
  if(condition==='uncertain') {
    const repeated=await client.resolve({taskId:'task',requestId:'original-attempt'});
    assert.equal(inputs,1);assert.equal(repeated.retryAllowed,false);
  }
  return {condition,family,inputs,captures,clearanceReads,accessReads};
}
const receipts=[];
for(const family of families)receipts.push(await scenario('success',family));
for(const condition of ['rpc_not_found','passive','image','manual','requires_cdp_free','manual_mode','manual_login','profile_manual','provider_denied','provider_disabled','wrong_backend','missing_capability','policy_denied','policy_changed','stale','control_changed','access_denied','clearance_denied','uncertain'])receipts.push(await scenario(condition));
console.log(JSON.stringify({passed:true,publicClient:true,receipts}));
