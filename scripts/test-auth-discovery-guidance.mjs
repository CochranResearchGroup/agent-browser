import assert from 'node:assert/strict';
import {createServer} from 'node:net';
import {mkdtemp,writeFile,rm,realpath,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,isAbsolute} from 'node:path';
import {connectHostedAuthHelper} from '../packages/client/src/hosted-auth-helper.js';
import {createOwnerLoginPlaybook} from '../packages/client/src/owner-login-playbook.js';

// Execute the real verified SDK and Agent Browser helper/factory. Only the
// external broker and receiving-browser ports are synthetic. No live discovery,
// enrolled source, credential store, provider or browser is contacted.
const installationRoot=process.env.AUTH_RECEIPTS_SDK_INSTALLATION;
assert.ok(installationRoot&&isAbsolute(installationRoot),'Set reviewed AUTH_RECEIPTS_SDK_INSTALLATION');
await realpath(installationRoot);
const root=await mkdtemp(join(tmpdir(),'auth-guidance-'));
const previous=process.env.CODEX_THREAD_ID;process.env.CODEX_THREAD_ID='guidance-caller';
const binding={taskId:'receiving-task',profileId:'fixture-profile',browserId:'fixture-browser',tabId:'fixture-tab',generation:'fixture-generation'};
const options={...binding,guideId:'bill-fixture',accountRef:'company-login',organizationRef:'fixture-company',query:'billing.example',accountHint:'company@example.test',origin:'https://billing.example',targetId:'fixture-target',purpose:'Synthetic documentation qualification',expiresAt:1800000060000,idempotencyKey:'fixture-login'};
const company={sourceId:'fixture-vault',recordId:'fixture-vault:company',origin:options.origin,account:options.accountHint,name:'Company BILL'};
const personal={...company,recordId:'fixture-vault:personal',account:'personal@example.test',name:'Personal BILL'};
const sources=[{sourceId:'fixture-vault',capabilities:{kind:'custom',credentialTypes:['password']}}];
const cases=[
 ['not_found',{state:'not_found',candidates:[],sources},'credential_not_found'],
 ['unavailable',{state:'partial',candidates:[],sources,unavailable:['fixture-vault']},'credential_source_unavailable'],
 ['partial_with_candidate',{state:'partial',candidates:[company],sources,unavailable:['other-source']},'credential_source_unavailable'],
 ['truncated',{state:'partial',candidates:[company],sources,truncated:true},'credential_discovery_incomplete'],
 ['account_choice',{state:'choice_required',candidates:[personal,company],sources},'account_ambiguous'],
 ['discovery_denied',{error:'auth_not_authorized'},'auth_not_authorized'],
 ['unverified',{state:'unknown',candidates:[],sources},'credential_unavailable'],
 ['unique',{state:'unique',candidates:[company],sources,selected:company},'authenticated'],
];
const documentation=await readFile(new URL('../skills/agent-browser/references/playbooks.md',import.meta.url),'utf8');
const example=documentation.split('<!-- owner-login-example -->')[1]?.split('```js\n')[1]?.split('```')[0];
assert.ok(example,'Current owner login documentation supplies an executable example');
const AsyncFunction=Object.getPrototypeOf(async function(){}).constructor;
const executeExample=new AsyncFunction('connectHostedAuthHelper','createOwnerLoginPlaybook','playbooks','durableMetadataStore',...Object.keys(options),example.replace(/^import .*;\n/gm,'')+'\nreturn result;');
let scenario,request,authenticated=false;
let capabilities={contractVersion:1,credentialOperations:true,sharedOwnerAuthority:true,legacyCredentialEffects:false};
const operations=[];
const server=createServer(socket=>{
 let bytes='';socket.on('data',data=>{
  bytes+=data;if(!bytes.includes('\n'))return;
  let envelope;try{envelope=JSON.parse(bytes.slice(0,bytes.indexOf('\n')));}catch{socket.destroy();return;}
  const {operation,args,context}=envelope;
  try{
   assert.equal(envelope.token,'synthetic-fixture-token');assert.equal(context.threadId,'guidance-caller');
   operations.push(operation);let result;
   if(operation==='auth_capabilities')result=capabilities;
   else if(operation==='list_accounts')result={accounts:[]};
   else if(operation==='discover_credentials'){
    assert.deepEqual(args,{query:options.query,accountHint:options.accountHint});
    if(scenario.error)throw Object.assign(Error('Synthetic discovery refusal'),{code:scenario.error});
    result=scenario;
   }else if(operation==='request_credential'){
    assert.equal(args.sourceId,company.sourceId);assert.equal(args.recordId,company.recordId);
    assert.equal(args.operation,'login');assert.equal(args.mode,'private');
    assert.equal(args.origin,options.origin);assert.equal(args.targetId,options.targetId);
    request={...args,requestId:'fixture-request',taskId:'guidance-caller',state:'pending'};result=request;
   }else if(operation==='inspect_credential_request'){
    assert.equal(args.requestId,request.requestId);result=request;
   }else if(operation==='receive_credential_approval'){
    assert.equal(args.requestId,request.requestId);request={...request,state:'approved'};result=request;
   }else if(operation==='use_credential'){
    assert.equal(request.state,'approved');assert.equal(args.requestId,request.requestId);
    authenticated=true;request={...request,state:'unknown',attemptId:'fixture-attempt',authentication:'verified'};result=request;
   }else throw Error('Unexpected operation '+operation);
   socket.end(JSON.stringify({result})+'\n');
  }catch(error){socket.end(JSON.stringify({error:{code:error.code??'fixture_assertion_failed',message:error.message}})+'\n');}
 });
});
try{
 const socketPath=join(root,'broker.sock');await new Promise(resolve=>server.listen(socketPath,resolve));
 const connectionFile=join(root,'connection.json'),discoveryFile=join(root,'discovery.json');
 await writeFile(connectionFile,JSON.stringify({socketPath,token:'synthetic-fixture-token'}),{mode:0o600});
 await writeFile(discoveryFile,JSON.stringify({contractVersion:2,transport:'codex-bots-auth',receivingProvider:'agent-browser',connectionFile}),{mode:0o600});
 const auth=await connectHostedAuthHelper({installationRoot,discoveryFile});
 assert.equal(auth.installation.product,'auth-receipts');
 console.log('SDK source: '+auth.installation.sourceRevision);
 assert.equal((await auth.helper('auth_capabilities',{})).sharedOwnerAuthority,true);
 assert.deepEqual(await auth.helper('list_accounts',{}),{accounts:[]});
 const browser={
  async verifyLogin(){return {...binding,status:authenticated?'completed':'waiting',reasonCode:authenticated?'authenticated':'password_required'};},
  async prepareLogin(){return {...binding,status:'completed',plan:{guideDigest:'fixture-guide',controlFingerprint:'fixture-controls'}};},
  async validatePlan(){return {...binding,status:'completed',reasonCode:'plan_current'};},
 };
 for(const [name,discovery,reason] of cases)for(const seam of ['factory','documentation']){
  scenario=discovery;request=undefined;authenticated=false;operations.length=0;
  const saved=new Map();const checkpoints={async load(key){return saved.get(key)??null;},async save(key,value){saved.set(key,structuredClone(value));}};
  const result=seam==='factory'
   ?await createOwnerLoginPlaybook({browser,auth,checkpoints}).advance(options)
   :await executeExample(()=>connectHostedAuthHelper({installationRoot,discoveryFile}),createOwnerLoginPlaybook,browser,checkpoints,...Object.values(options));
  assert.equal(result.reasonCode,reason,name);
  assert.ok(!operations.includes('list_accounts'),'Current login never uses legacy scopes for discovery');
  if(name==='unique'){
   assert.equal(result.status,'completed');assert.equal(result.requestId,'fixture-request');
   assert.equal(operations.filter(x=>x==='request_credential').length,1);
   assert.equal(operations.filter(x=>x==='use_credential').length,1);
  }else{
   assert.equal(result.status,'waiting');assert.equal(result.effectState,'no_effect');
   assert.equal(saved.size,0,'Unqualified discovery cannot create a selected-account checkpoint');
   assert.ok(!operations.some(x=>['request_credential','use_credential'].includes(x)),name);
  }
  console.log('passed: '+seam+'/'+name);
 }
 capabilities={...capabilities,sharedOwnerAuthority:false};operations.length=0;
 await assert.rejects(executeExample(()=>connectHostedAuthHelper({installationRoot,discoveryFile}),createOwnerLoginPlaybook,browser,{load:async()=>null,save:async()=>{}},...Object.values(options)),{code:'auth_authority_migration_required'});
 assert.deepEqual(operations,['auth_capabilities'],'Incompatible owner authority cannot discover or request credentials');
 console.log('passed: documentation/incompatible-capabilities');
 console.log('Actual verified Auth SDK/helper and public owner login distinguish discovery outcomes; synthetic private use requires owner approval and exact receiving identity');
}finally{
 await new Promise(resolve=>server.close(resolve));
 if(previous===undefined)delete process.env.CODEX_THREAD_ID;else process.env.CODEX_THREAD_ID=previous;
 await rm(root,{recursive:true,force:true});
}
