// Qualification only. This factory deliberately refuses every real account.
import assert from 'node:assert/strict';
import {readFile,writeFile,lstat,rename} from 'node:fs/promises';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
const hash=value=>createHash('sha256').update(value).digest('hex');
const ocr=bytes=>new Promise((resolve,reject)=>{
 const child=spawn('/usr/bin/tesseract',['stdin','stdout','--psm','11'],{stdio:['pipe','pipe','pipe']});let text='';
 const timer=setTimeout(()=>{child.kill('SIGTERM');reject(Error('Native fixture OCR timeout'));},10000);
 child.stdout.on('data',data=>{text+=data;if(text.length>65536){child.kill('SIGTERM');reject(Error('Native fixture OCR limit'));}});child.stderr.resume();child.stdin.on('error',reject);child.on('error',reject);child.on('close',code=>{clearTimeout(timer);code===0?resolve(text):reject(Error('Native fixture OCR failed'));});child.stdin.end(bytes);
});
// This file is the fixture's durable execution audit, including across the
// producer restart exercised by qualification. Never replace it with zeros.
export async function loadNativeFixtureAudit(root){
 let audit={privateSubmissions:0,nativeOperationDispatches:0,nativeTextDispatches:0};
 try{audit=JSON.parse(await readFile(join(root,'native-input-audit.json'),'utf8'));}catch(error){if(error.code!=='ENOENT')throw error;}
 assert.deepEqual(Object.keys(audit).sort(),['nativeOperationDispatches','nativeTextDispatches','privateSubmissions']);
 assert(Object.values(audit).every(value=>Number.isSafeInteger(value)&&value>=0));return audit;
}
export async function loadProtectedInputAudit(root){
 const audit=JSON.parse(await readFile(join(root,'protected-input-audit.json'),'utf8'));
 assert.deepEqual(Object.keys(audit).sort(),['mfaFills','passwordFills','version']);assert.equal(audit.version,1);
 assert([audit.passwordFills,audit.mfaFills].every(value=>Number.isSafeInteger(value)&&value>=0));return audit;
}
export async function createAuthConfig(context){
 const file=process.env.CODEX_AUTH_EXTERNAL_FIXTURE_CONFIG,info=await lstat(file);assert(info.isFile()&&info.uid===process.getuid()&&!(info.mode&0o077));
 const config=JSON.parse(await readFile(file,'utf8'));assert(config.nativeStaged===true&&config.password.startsWith('dummy-')&&new URL(config.origin).hostname==='127.0.0.1');
 for(const module of Object.values(config.nativeModules)){assert(module.path.startsWith(join(config.root,'native-modules')+'/'));assert.equal(hash(await readFile(module.path)),module.sha256);}
 const {createAuthConfig:original}=await import(pathToFileURL(join(config.installedRoot,'scripts/auth-external-fixture-host.mjs')));
 const base=await original(context);
 const {createHostedLoginReceiver}=await import(pathToFileURL(config.nativeModules.receiver.path));
 const {createNativeReceivingBrowser}=await import(pathToFileURL(config.nativeModules.browser.path));
 const {connectRemoteView}=await import(pathToFileURL(join(config.installedRoot,'src/remote-view.mjs')));
 const {createMcpStdioClient}=await import(config.smokeUtilities);
 const remote=await connectRemoteView({expectedInstallationId:config.installationId});
 const client=createMcpStdioClient({context:{env:config.browserEnv},args:['mcp','serve'],onFatal:()=>{}});
 await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'native-staged-private-receiver',version:'1'}});client.notify('notifications/initialized');
 process.once('SIGTERM',()=>{client.close();remote.close();});
 const call=async(name,args)=>{const reply=await client.send('tools/call',{name,arguments:args});const envelope=JSON.parse(reply.content.find(c=>c.type==='text').text);assert.equal(envelope.success,true,'Native fixture private browser operation failed');return envelope.data;};
 const binding=structuredClone(config.receivingTarget),scope=structuredClone(base.accounts[0]);delete scope.callers;delete scope.approvers;
 const audit=await loadNativeFixtureAudit(config.root),protectedAudit=await loadProtectedInputAudit(config.root);
 const save=()=>writeFile(join(config.root,'native-input-audit.json'),JSON.stringify(audit)+'\n',{mode:0o600});await save();
 const observed=await remote.observation(config.desktop),windows=observed.windows.windows.filter(w=>w.active&&w.processIdentity?.pid===config.processIdentity.pid&&w.processIdentity.start_token===config.processIdentity.startToken);assert.equal(windows.length,1);
 const w=windows[0],guards={windowId:w.id,expectedProcessIdentity:w.processIdentity,expectedWidth:w.width,expectedHeight:w.height,expectedWindow:{x:w.x,y:w.y,width:w.width,height:w.height}};
 const native={observe:b=>remote.observe(b),observation:b=>remote.observation(b),captureRegion:(b,args)=>remote.captureRegion(b,args),async input(b,args){audit.nativeOperationDispatches++;audit.nativeTextDispatches+=args.actions.filter(a=>a.kind==='text').length;await save();return remote.input(b,args);}};
 const observeScript=`(()=>{const specs={identifier:'input[name=username]',password:'input[name=password]',identifierSubmit:'#identifier-submit',submit:'#password-submit'};
  const controls=Object.fromEntries(Object.entries(specs).map(([name,selector])=>{const es=document.querySelectorAll(selector),e=es.length===1?es[0]:null,r=e?.getBoundingClientRect(),visible=!!(r&&r.width&&r.height&&getComputedStyle(e).visibility!=='hidden'),hit=r?document.elementFromPoint(r.x+r.width/2,r.y+r.height/2):null;return [name,{selector,count:es.length,tag:e?.tagName??null,type:e?.type??null,visible,enabled:!!e&&!e.disabled,hitTarget:!!e&&(hit===e||e.contains(hit)),rect:r?{x:r.x,y:r.y,width:r.width,height:r.height}:null}];}));
  const authenticated=!!document.querySelector('#application'),state=authenticated?'authenticated':controls.identifier.visible?'identifier_required':controls.password.visible?'password_required':'unknown';
  const accountMatched=authenticated?document.querySelector('#application')?.getAttribute('data-account')==='dummy':state==='password_required'?document.querySelector('#account')?.textContent===${JSON.stringify(config.username)}:document.querySelector('input[name=username]')?.value===${JSON.stringify(config.username)};
  return {origin:location.origin,state,accountMatched,visiblePassword:controls.password.visible,maskedPassword:!controls.password.count||controls.password.type==='password',controls,viewport:{width:innerWidth,height:innerHeight,devicePixelRatio,focused:document.hasFocus(),visible:document.visibilityState==='visible'}};})()`;
 const browser=createNativeReceivingBrowser({remote:native,desktop:config.desktop,guards,
  async readObservation(){const current=await call('browser_continue',{taskId:binding.taskId});for(const key of ['taskId','profileId','browserId','tabId'])assert.equal(current[key],binding[key]);return {...(await call('browser_command',{taskId:binding.taskId,action:'evaluate',params:{script:observeScript}})).result,binding:{...binding}};},
  async inputMatches({selector,value,guard}){await guard();return (await call('browser_command',{taskId:binding.taskId,action:'evaluate',params:{script:`(()=>{const es=document.querySelectorAll(${JSON.stringify(selector)});return es.length===1&&es[0].value===${JSON.stringify(value)};})()`}})).result===true;},
 });
 const fill=browser.fill,activate=browser.activate;
 browser.fill=async input=>{
  if(['password','mfa'].includes(input.control)){
   protectedAudit[input.control==='password'?'passwordFills':'mfaFills']++;
   await writeFile(join(config.root,'protected-input-audit.json.next'),JSON.stringify(protectedAudit),{mode:0o600});await rename(join(config.root,'protected-input-audit.json.next'),join(config.root,'protected-input-audit.json'));
  }
  return fill(input);
 };
 browser.activate=async input=>{if(config.untouchedLogin&&input.control==='identifierSubmit')throw Error('Owned pre-protected-input submit obstruction');return activate(input);};
 return {...base,external:{...base.external,async createReceiver(){
  const readSafety=async({credential})=>{
    const current=await browser.observe(binding);assert.equal(current.state,'authenticated');
    const clear=(await call('browser_command',{taskId:binding.taskId,action:'evaluate',params:{script:`(()=>{const secret=${JSON.stringify(credential.password)};return !document.body.innerText.includes(secret)&&![...document.querySelectorAll('input,textarea')].some(e=>e.value.includes(secret));})()`}})).result;
    const captured=await remote.capture(config.desktop),bytes=Buffer.from(captured.image.data,'base64'),text=await ocr(bytes),normal=text.toLowerCase().replace(/[^a-z0-9]/g,'');
    const safe=clear===true&&normal.includes('owneduser')&&normal.includes('managedauthenticationverified')&&!text.includes(credential.password);await browser.observe(binding);
    await writeFile(join(config.root,'native-safety.json'),JSON.stringify({safe,bytes:bytes.length,sha256:hash(bytes),exactOwner:true,credentialAbsent:!text.includes(credential.password)})+'\n',{mode:0o600});return {safe};
  };
  const receiver=createHostedLoginReceiver({binding,scope,expectedUsername:config.username,browser,
   async readPrincipal(){const principal=(await call('browser_command',{taskId:binding.taskId,action:'evaluate',params:{script:"fetch('/whoami').then(r=>r.ok?r.json():null)"}})).result;assert.equal(principal?.username,config.username);assert.equal(principal.origin,config.origin);return {...principal,authenticated:true};},
   readCaptureSafety:readSafety,
   async readReconciliationSafety({authenticated,guard}){
    if(authenticated)return readSafety({credential:{username:config.username,password:config.password}});
    if(!config.untouchedLogin||protectedAudit.passwordFills!==0||protectedAudit.mfaFills!==0)return {safe:false};
    await guard();const before=await browser.observe(binding);if(before.state!=='identifier_required'||!before.accountMatched)return {safe:false};
    const clear=async()=>(await call('browser_command',{taskId:binding.taskId,action:'evaluate',params:{script:`(()=>{const secret=${JSON.stringify(config.password)};return !document.body.innerText.includes(secret)&&![...document.querySelectorAll('input,textarea')].some(e=>e.value.includes(secret))&&!document.querySelector('input[type=password]')?.value;})()`}})).result===true;
    if(!await clear())return {safe:false};
    const captured=await remote.capture(config.desktop),bytes=Buffer.from(captured.image.data,'base64'),text=await ocr(bytes),normal=text.toLowerCase().replace(/[^a-z0-9]/g,'');
    await guard();const after=await browser.observe(binding),currentAudit=await loadProtectedInputAudit(config.root);
    const checks={sameObservation:after.fingerprint===before.fingerprint,domClear:await clear(),nativeLoginHeader:normal.includes('ownednativeidentifierlogin'),credentialAbsent:!text.includes(config.password),untouchedPassword:currentAudit.passwordFills===0,untouchedFactor:currentAudit.mfaFills===0};
    const safe=Object.values(checks).every(Boolean);
    await writeFile(join(config.root,'native-safety.json'),JSON.stringify({safe,bytes:bytes.length,sha256:hash(bytes),exactOwner:true,credentialAbsent:!text.includes(config.password),knownLoginVerified:safe,protectedInputNotAttempted:safe,checks,smallTextDiagnostics:{nativeUser:normal.includes('owneduser'),nativeContinue:normal.includes('continue')}})+'\n',{mode:0o600});
    return {safe,protectedInputNotAttempted:safe,receivingLoginVerified:safe};
   },
  });
  if(config.reconcileHeld)receiver.verify=async()=>{throw Error('Owned fixture lost final verification acknowledgement');};
  const apply=receiver.apply;receiver.apply=async(...args)=>{audit.privateSubmissions++;await save();return apply(...args);};return receiver;
 }}};
}
