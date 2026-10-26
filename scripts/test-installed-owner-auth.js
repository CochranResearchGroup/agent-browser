import assert from 'node:assert/strict';
import {randomBytes,createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {mkdtemp,mkdir,cp,readFile,writeFile,rm,readdir,symlink} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,dirname,basename,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {chromium} from 'playwright';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';

const source=process.env.AUTH_RECEIPTS_SOURCE,parentSource=process.env.RECEIPTS_SOURCE;
assert.ok(source&&parentSource,'Set reviewed AUTH_RECEIPTS_SOURCE and RECEIPTS_SOURCE');
const root=await mkdtemp(join(tmpdir(),'installed-owner-auth-'));
const envKeys=['HOME','XDG_DATA_HOME','CODEX_THREAD_ID','AUTH_RECEIPTS_DISCOVERY_FILE','CODEX_AUTH_DISCOVERY_FILE'];
const saved=Object.fromEntries(envKeys.map(k=>[k,process.env[k]]));
const load=(dir,file)=>import(pathToFileURL(join(dir,file)).href);
const revision=dir=>execFileSync('git',['-C',dir,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
async function manifest(dir){const rows=[];async function walk(at){for(const item of (await readdir(join(dir,at),{withFileTypes:true})).sort((a,b)=>a.name.localeCompare(b.name))){const name=join(at,item.name);if(item.isDirectory())await walk(name);else if(item.isFile())rows.push([name,createHash('sha256').update(await readFile(join(dir,name))).digest('hex')]);}}await walk('');return {files:rows.length,sha256:createHash('sha256').update(JSON.stringify(rows)).digest('hex')};}
let evidence;const captureDiagnostics=[];
try{
 const service=join(root,'candidates/auth'),parent=join(root,'candidates/parent'),client=join(root,'candidates/browser-client'),harness=join(root,'synthetic-harness'),home=join(root,'home'),data=join(home,'.local/share');
 for(const dir of [service,parent,client,harness,home])await mkdir(dir,{recursive:true,mode:0o700});
 await cp(join(source,'src'),join(service,'src'),{recursive:true});await cp(join(source,'package.json'),join(service,'package.json'));await cp(join(source,'node_modules'),join(service,'node_modules'),{recursive:true,dereference:true});
 await cp(join(parentSource,'apps/receipts-interoperation/src'),join(parent,'apps/receipts-interoperation/src'),{recursive:true});await cp(join(parentSource,'package.json'),join(parent,'package.json'));
 await cp(resolve('packages/client/src'),join(client,'src'),{recursive:true});await cp(resolve('packages/client/package.json'),join(client,'package.json'));
 // Synthetic HTTPS server only: its public RFC test seed generates 287082 at 45s.
 await cp(join(source,'test/helpers/browser-acceptance-fixture.mjs'),join(harness,'browser-acceptance-fixture.mjs'));
 await writeFile(join(harness,'browser-acceptance-server.mjs'),(await readFile(join(source,'test/helpers/browser-acceptance-server.mjs'),'utf8')).replace("==='123456'","==='287082'"));
 Object.assign(process.env,{HOME:home,XDG_DATA_HOME:data,CODEX_THREAD_ID:'synthetic-caller'});delete process.env.AUTH_RECEIPTS_DISCOVERY_FILE;delete process.env.CODEX_AUTH_DISCOVERY_FILE;
 const [{withBrowserAcceptanceFixture},{createAgentBrowserRuntime,inspectAgentBrowserTarget},{openEnrolledBrowserReceiver},{connectHostedAuthHelper},{createOwnerLoginPlaybook},{createOwnerTotpContinuation}]=await Promise.all([
 load(harness,'browser-acceptance-fixture.mjs'),load(service,'src/agent-browser-runtime.mjs'),load(service,'src/enrolled-browser-receiver.mjs'),load(client,'src/hosted-auth-helper.js'),load(client,'src/login-playbook.js'),load(client,'src/owner-totp-continuation.js')]);
 assert.deepEqual(await manifest(join(service,'src')),await manifest(join(source,'src')));assert.deepEqual(await manifest(join(parent,'apps/receipts-interoperation/src')),await manifest(join(parentSource,'apps/receipts-interoperation/src')));assert.deepEqual(await manifest(join(client,'src')),await manifest(resolve('packages/client/src')));
 const artifacts=join(root,'evidence');
 await withBrowserAcceptanceFixture({chromium,artifacts},async chrome=>{
  const {page,peer,origin,targetId,counts}=chrome,profile=join(chrome.root,'chrome'),principal={id:'client',taskId:'synthetic-caller'};
  let enrolled,f,fillChecks=0,proof;const receiverRefusals=[];
  const runtime=createAgentBrowserRuntime({profileDirectories:{'isolated-client-profile':profile}});
  const route=await runtime.enroll({clientId:'client',taskId:'synthetic-caller',targetId,profile:'isolated-client-profile',mode:'client'});
  proof=await inspectAgentBrowserTarget({profile:route.profile,targetId},{profileDirectory:profile});
  assert.equal(proof.pid,chrome.child.pid);assert.equal(route.generation,proof.generation);assert.ok(proof.generation);
  await assert.rejects(runtime.resolve({...route,generation:'forged'}),{code:'BROWSER_RUNTIME_CHANGED'});
  const realResolve=runtime.resolve.bind(runtime);
  // Call-through observation: the actual enrolled durable gate must reject capture
  // immediately before every protected CDP input. No privacy authority is replaced.
  runtime.resolve=async r=>{const binding=await realResolve(r),privatePage=binding.page;return {...binding,page:{...privatePage,async screenshot(){try{return await privatePage.screenshot();}catch(error){captureDiagnostics.push({code:error.code,operation:error.operation,reason:error.reason});throw error;}},locator(selector){const fields=privatePage.locator(selector);return {nth(index){const field=fields.nth(index);return {async fill(value){await assert.rejects(enrolled.capture(principal,targetId),{code:'BROWSER_CAPTURE_HELD'});fillChecks++;return field.fill(value);}};}};}}};};
  const identity=async()=>{if(await page.locator('input[type=password],input[autocomplete=one-time-code]').count())return null;const who=await page.evaluate(async()=>await(await fetch('/whoami')).json());return who.factor===true?{origin,username:who.username,organization:who.organization}:null;};
  const openReceiver=()=>openEnrolledBrowserReceiver({root:join(root,'receiving'),identityKey:Buffer.alloc(32,7),routes:[route],runtime,readIdentity:identity,readAuthenticationContext:async()=>{const who=await page.evaluate(async()=>await(await fetch('/whoami')).json());return {kind:await page.locator('input[autocomplete=one-time-code]').count()?'totp':'none',username:who.username,observationId:'fixture-server-factor-form'};}});
  try{
   await page.bringToFront();await mkdir(join(root,'receiving'),{mode:0o700});enrolled=await openReceiver();assert.ok((await enrolled.capture(principal,targetId)).length);await assert.rejects(enrolled.capture({...principal,taskId:'foreign'},targetId),{code:'BROWSER_ENROLLMENT_DENIED'});
   assert.equal(typeof enrolled.receiver.observeChallenge,'undefined','Number-matching observer is not supported by this installed enrolled route');
   f=await createOwnerAuthFixture({source:service,parentSource:parent,sdkSource:source,configure({state,receiving,sourcePort}){
    state.time=45000;Object.assign(receiving,{tabId:targetId,generation:route.generation});
    sourcePort.discover=async()=>[{recordId:'synthetic:billing',origin,name:'Installed synthetic Chrome',account:'synthetic-user',modifiedAt:1}];sourcePort.read=async(_id,{authorize})=>{await authorize();state.reads++;return {username:'synthetic-user',password:'SYNTHETIC-ONLY-PASSWORD'};};
    const receiver={};for(const method of ['observeAuthentication','validate','use','reconcile','cancel','observeAttempt'])receiver[method]=async(...args)=>{try{return await enrolled.receiver[method](...args);}catch(error){receiverRefusals.push({method,code:error.code});throw error;}};
    return {receiver,materials:{key:Buffer.alloc(32,3),readClock:async()=>({synchronized:true,timeMs:state.time}),minimumRemainingMs:0,verifyEnrollment:async()=>true}};
   }});
   // The default discovery location is task-owned. Both metadata files remain
   // regular private files; only their parent directory has an isolated locator.
   let actualDiscovery;
   async function locate(dir){for(const ent of await readdir(dir,{withFileTypes:true})){const path=join(dir,ent.name);if(ent.isDirectory())await locate(path);else if(ent.name==='external-discovery.json')actualDiscovery=path;}}
   await locate(join(f.root,'ipc'));assert.ok(actualDiscovery);
   const defaultDirectory=join(home,'.local/state/codex-bots/auth');await mkdir(dirname(defaultDirectory),{recursive:true,mode:0o700});await symlink(dirname(actualDiscovery),defaultDirectory);
   async function bindDefaultDiscovery(){await locate(join(f.root,'ipc'));const metadata=JSON.parse(await readFile(actualDiscovery,'utf8'));metadata.connectionFile=join(defaultDirectory,basename(metadata.connectionFile));await writeFile(actualDiscovery,JSON.stringify(metadata),{mode:0o600});}
   await bindDefaultDiscovery();const selector=join(data,'auth-receipts/client/current');await mkdir(dirname(selector),{recursive:true});await cp(join(f.root,'sdk'),join(root,'candidates/sdk'),{recursive:true});await symlink(join(root,'candidates/sdk'),selector);
   const auth=await connectHostedAuthHelper();assert.equal(auth.installation.product,'auth-receipts');
   Object.assign(f.options,{origin,query:origin,accountHint:'synthetic-user',targetId,idempotencyKey:'installed-password'});
   f.browser.verifyLogin=async()=>({...f.options,status:await identity()?'completed':'waiting',reasonCode:await identity()?'authenticated':await page.locator('input[autocomplete=one-time-code]').count()?'mfa_required':'password_required',effectState:'no_effect'});
   const describe=()=>page.evaluate(()=>({origin:location.origin,controls:[...document.querySelectorAll('input')].map(e=>({name:e.name,type:e.type,autocomplete:e.autocomplete})),forms:[...document.forms].map(e=>({action:e.action,method:e.method}))}));
   f.browser.prepareLogin=async()=>{const current=await inspectAgentBrowserTarget({profile:route.profile,targetId},{profileDirectory:profile});return {...f.options,generation:current.generation,status:'completed',plan:await describe()};};
   f.browser.validatePlan=async options=>{const current=await inspectAgentBrowserTarget({profile:route.profile,targetId},{profileDirectory:profile});return {...f.options,generation:current.generation,status:JSON.stringify(options.plan)===JSON.stringify(await describe())?'completed':'adaptation_required',reasonCode:'plan_current'};};
   const dependencies={auth,browser:f.browser,checkpoints:f.checkpoints};let playbook=createOwnerLoginPlaybook(dependencies);
   const pending=await playbook.advance(f.options);assert.equal(pending.reasonCode,'approval_required');assert.equal(f.state.reads,0);f.respond();await playbook.advance(f.options);
   await page.waitForSelector('input[autocomplete=one-time-code]');assert.equal(counts.password,1);assert.equal(f.state.reads,1);assert.equal(fillChecks,2);await assert.rejects(enrolled.capture(principal,targetId),{code:'BROWSER_CAPTURE_HELD'});
   await f.restart();await bindDefaultDiscovery();enrolled.close();enrolled=await openReceiver();playbook=createOwnerLoginPlaybook(dependencies);assert.ok((await auth.helper('auth_capabilities',{})).sharedOwnerAuthority);const resumed=await playbook.advance(f.options);assert.equal(resumed.requestId,pending.requestId);assert.notEqual(resumed.status,'completed');assert.equal(counts.password,1);assert.equal(f.state.reads,1);
   const staged=await f.ownerCall('stage_mfa',{sourceId:'synthetic',recordId:'synthetic:billing',origin,input:{kind:'key',value:'GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ'},options:{account:'synthetic-user',issuer:'Synthetic'}});await f.ownerCall('validate_mfa',{enrollmentId:staged.id});
   for(const [id,operations,approval]of [['discover',['discover'],'standing'],['login',['login'],'per-use']])await f.ownerCall('put_credential_rule',{rule:{id,effect:'allow',clients:['client'],sources:['mfa'],credentialTypes:['otp'],operations,origins:[origin],modes:['private'],approval,notificationGate:'delivered'}});
   const factorOptions={...f.options,guideId:'factor',idempotencyKey:'installed-totp',originalRequestId:pending.requestId};let factor=createOwnerTotpContinuation(dependencies);const factorPending=await factor.advance(factorOptions);assert.equal(factorPending.reasonCode,'approval_required');f.respond();const factorUse=await factor.advance(factorOptions);assert.notEqual(factorUse.reasonCode,'auth_unavailable',JSON.stringify({reason:'same-target factor dispatch must transfer the original password reservation',receiverRefusals,password:counts.password,factor:counts.factor,protectedInputs:fillChecks}));await page.waitForSelector('p');
   const complete=await factor.advance(factorOptions);assert.equal(complete.status,'completed');assert.equal(counts.factor,1);assert.equal(fillChecks,3);assert.ok(await identity());assert.ok((await enrolled.capture(principal,targetId)).length);
   await f.restart();await bindDefaultDiscovery();enrolled.close();enrolled=await openReceiver();factor=createOwnerTotpContinuation(dependencies);assert.equal((await factor.advance(factorOptions)).requestId,factorPending.requestId);assert.equal(counts.factor,1);assert.equal(counts.password,1);assert.equal(f.state.reads,1);assert.equal(fillChecks,3);assert.equal((await playbook.advance(f.options)).status,'completed','Original password workflow independently verifies the final intended account/company');
   assert.equal(await peer.locator('input[type=password]').count(),1,'Unrelated target remains on its initial form');
   evidence={contractVersion:1,qualification:'isolated-installed-browser-client',sources:{browser:revision(resolve('.')),auth:revision(source),parent:revision(parentSource)},candidates:{service:await manifest(service),parent:await manifest(parent),browserClient:await manifest(client),sdk:await manifest(join(root,'candidates/sdk'))},installation:auth.installation,physical:{pid:chrome.child.pid,targetId,generation:proof.generation,chromeVersion:chrome.browser.version(),pidMatches:true,generationMatches:true,forgedGenerationRefused:true},privacy:{captureBefore:true,preInputCaptureRefusals:fillChecks,heldAcrossMfa:true,captureAfter:true},operations:{password:counts.password,totp:counts.factor,protectedSourceReads:f.state.reads},recovery:{authorityRestart:true,receiverReopen:true,originalRequests:true,noRepeat:true},captureDiagnostics,unsupported:['native-at-spi','enrolled-number-matching-observer'],unrelatedTargetPreserved:true};
  }finally{enrolled?.close();await f?.close();}
 });
 evidence.cleanup=JSON.parse(await readFile(join(artifacts,'cleanup.json'),'utf8'));assert.equal(evidence.cleanup.chromeExited,true);assert.equal(evidence.cleanup.remainingOwnedProcesses,0);const remainingHomeProcesses=execFileSync('ps',['-eo','pid=,ppid=,args='],{encoding:'utf8'}).split('\n').filter(line=>line.includes(root));evidence.cleanup.remainingTaskHomeProcesses=remainingHomeProcesses.length;assert.equal(remainingHomeProcesses.length,0,'No process retaining the task-owned HOME/candidate paths');
 if(process.env.INSTALLED_OWNER_AUTH_EVIDENCE)await writeFile(process.env.INSTALLED_OWNER_AUTH_EVIDENCE,JSON.stringify(evidence,null,2)+'\n',{mode:0o600});
 console.log(JSON.stringify(evidence,null,2));
}finally{if(!evidence){try{console.log('Failed-run OS cleanup:',await readFile(join(root,'evidence/cleanup.json'),'utf8'));}catch{}}for(const key of envKeys){if(saved[key]===undefined)delete process.env[key];else process.env[key]=saved[key];}await rm(root,{recursive:true,force:true});}
