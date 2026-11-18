import assert from 'node:assert/strict';
import {test} from 'node:test';
import {createLoginDiscovery} from '../packages/client/src/login-discovery.js';

const intent={origin:'https://accounts.example.test',accountHint:'operator@example.test',extensionState:'signed_out'};
const candidate={sourceId:'lastpass',recordId:'record-1',origin:intent.origin,account:intent.accountHint};
function fixture(discovery={state:'unique',sources:[{sourceId:'lastpass',capabilities:{discover:true,privateUse:true}}],candidates:[candidate],selected:candidate}) {
  const operations=[];
  return {operations,client:{async call(operation,args){
    operations.push({operation,args});
    if(operation==='auth_capabilities')return {contractVersion:1,credentialOperations:true,sharedOwnerAuthority:true,legacyCredentialEffects:false};
    if(operation==='discover_credentials')return discovery;
    throw Error('unexpected operation');
  }}};
}

test('signed-out extension still discovers the enrolled Auth source without target private use',async()=>{
  const f=fixture();
  const result=await createLoginDiscovery({client:f.client}).inspect(intent);
  assert.equal(result.category,'credential_ready');
  assert.equal(result.extensionState,'signed_out');
  assert.equal(result.sourceAvailability,'available');
  assert.equal(result.permission.state,'visibility_unavailable');
  assert.equal(result.waitingForUser,false);
  assert.deepEqual(result.selected,{sourceId:'lastpass',recordId:'record-1'});
  assert.deepEqual(f.operations.map(x=>x.operation),['auth_capabilities','discover_credentials']);
});

test('invalid input, mismatched request and malformed source response cannot produce readiness',async()=>{
  const invalid=fixture();
  await assert.rejects(createLoginDiscovery({client:invalid.client}).inspect({...intent,origin:'http://accounts.example.test'}),{code:'auth_invalid_request'});
  assert.equal(invalid.operations.length,0);
  const mismatched=fixture();const original=mismatched.client.call;
  mismatched.client.call=async(operation,args)=>operation==='inspect_credential_request'?{requestId:'other-request',state:'approved'}:original(operation,args);
  const mismatch=await createLoginDiscovery({client:mismatched.client}).inspect({...intent,requestId:'original-request'});
  assert.equal(mismatch.category,'request_binding_mismatch');
  assert.equal(mismatch.requestId,'original-request');
  const malformed=fixture(null);
  assert.equal((await createLoginDiscovery({client:malformed.client}).inspect(intent)).category,'capability_unavailable');
  const malformedCollections=fixture({state:'unique',sources:{},candidates:{},selected:candidate});
  assert.equal((await createLoginDiscovery({client:malformedCollections.client}).inspect(intent)).category,'capability_unavailable');
  const forged=fixture({state:'unique',sources:[],candidates:[{...candidate,sourceId:'SECRET RAW SOURCE'}],selected:{...candidate,sourceId:'SECRET RAW SOURCE'}});
  const held=await createLoginDiscovery({client:forged.client}).inspect(intent);
  assert.equal(held.selected,undefined);
  assert.equal(JSON.stringify(held).includes('SECRET RAW SOURCE'),false);
});

test('capability absence and real policy refusal remain distinct from a hidden permission inventory',async()=>{
  const absent=fixture();
  absent.client.call=async operation=>{absent.operations.push({operation});return {contractVersion:1,credentialOperations:false};};
  const unavailable=await createLoginDiscovery({client:absent.client}).inspect(intent);
  assert.equal(unavailable.category,'capability_unavailable');
  assert.deepEqual(absent.operations.map(x=>x.operation),['auth_capabilities']);
  const refused=fixture();
  const call=refused.client.call;
  refused.client.call=async(operation,args)=>{
    if(operation==='discover_credentials')throw Object.assign(Error('private detail'),{code:'auth_not_authorized'});
    return call(operation,args);
  };
  const denial=await createLoginDiscovery({client:refused.client}).inspect(intent);
  assert.equal(denial.category,'authority_denied');
  assert.equal(denial.waitingForUser,false);
  assert.equal(JSON.stringify(denial).includes('private detail'),false);
});

test('owner permission must be current and bound to the exact discovered identity',async()=>{
  for(const readPermission of [
    async()=>{throw Object.assign(Error('restricted'),{code:'auth_not_authorized'});},
    async scope=>({state:'authorized',current:true,kind:'standing',scope:{...scope,recordId:'other'},evidenceRef:'permission-1'}),
    async scope=>({state:'authorized',current:false,kind:'standing',scope,evidenceRef:'permission-1'}),
  ]) {
    const result=await createLoginDiscovery({client:fixture().client,readPermission}).inspect(intent);
    assert.equal(result.permission.state,'visibility_unavailable');
    assert.equal(result.waitingForUser,false);
  }
  const denied=await createLoginDiscovery({client:fixture().client,readPermission:async scope=>({state:'denied',current:true,scope,evidenceRef:'denial-1'})}).inspect(intent);
  assert.equal(denied.category,'authority_denied');
  assert.equal(denied.permission.state,'denied');
});

test('an owner-scoped standing permission survives an expired individual request without a new approval',async()=>{
  const f=fixture();
  const original=f.client.call;
  f.client.call=async(operation,args)=>{
    if(operation==='inspect_credential_request') {
      f.operations.push({operation,args});
      return {requestId:args.requestId,state:'expired'};
    }
    return original(operation,args);
  };
  const result=await createLoginDiscovery({client:f.client,readPermission:async scope=>({state:'authorized',kind:'standing',current:true,scope,evidenceRef:'owner-permission-1'})}).inspect({...intent,requestId:'expired-request'});
  assert.equal(result.category,'credential_ready');
  assert.equal(result.permission.state,'owner_authorized');
  assert.equal(result.permission.kind,'standing');
  assert.equal(result.requestId,'expired-request');
  assert.equal(result.waitingForUser,false);
  assert.deepEqual(f.operations.map(x=>x.operation),['auth_capabilities','inspect_credential_request','discover_credentials']);
});

test('missing recovery and ambiguous or incomplete metadata never select a credential',async()=>{
  for(const [discovery,category,availability] of [
    [{state:'partial',sources:[{sourceId:'lastpass',capabilities:{discover:true}}],candidates:[],unavailable:['lastpass']},'source_unavailable','unavailable'],
    [{state:'choice_required',sources:[],candidates:[candidate,{...candidate,recordId:'record-2'}]},'credential_choice_required','unknown'],
    [{state:'unique',sources:[],candidates:[candidate],selected:{...candidate,account:'other@example.test'}},'credential_choice_required','unknown'],
    [{state:'partial',sources:[],candidates:[candidate],selected:candidate},'discovery_incomplete','unknown'],
  ]) {
    const f=fixture(discovery);
    const result=await createLoginDiscovery({client:f.client}).inspect(intent);
    assert.equal(result.category,category);
    assert.equal(result.sourceAvailability,availability);
    assert.equal(result.selected,undefined);
    assert.equal(result.waitingForUser,false);
    assert.equal(JSON.stringify(result).includes('other@example.test'),false);
  }
});
