import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
import {mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {resumeOwnerLoginInFreshProcess} from './fixtures/owner-chrome-process.mjs';
import {chromium} from 'playwright';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';
import {createOwnerLoginPlaybook} from '../packages/client/src/login-playbook.js';

const source=process.env.AUTH_RECEIPTS_SOURCE;if(!source)throw Error('Set reviewed AUTH_RECEIPTS_SOURCE');
const load=file=>import(pathToFileURL(join(source,file)));
const [{withBrowserAcceptanceFixture},{createGenericBrowserReceiver},{openPrivateTransactions}]=await Promise.all([
 load('test/helpers/browser-acceptance-fixture.mjs'),load('src/generic-browser-receiver.mjs'),load('src/private-transactions.mjs')]);
const artifacts=await mkdtemp(join(tmpdir(),'owner-chrome-evidence-'));
try{await withBrowserAcceptanceFixture({chromium,artifacts},async chrome=>{
 const {page,origin,counts,targetId,generation}=chrome,urls=[];page.on('request',r=>urls.push(r.url()));
 const fields='<input name="username" autocomplete="username"><input type="password" name="password" autocomplete="current-password">';
 for(const kind of ['click','submit','native','lost-submit','lost-fill','unhandled','wrong-account','wrong-organization','changed-target']){
  await page.goto(origin);
  const send=`const r=await fetch('/password',{method:'POST',body:new URLSearchParams(new FormData(document.querySelector('form')))});if(r.ok)document.body.innerHTML='<p>Signed in</p>';`;
  const event=kind==='unhandled'?'document.querySelector("button").addEventListener("click",e=>e.preventDefault());':kind==='submit'?'document.querySelector("form").addEventListener("submit",async e=>{e.preventDefault();'+send+'});':kind==='lost-fill'?'document.querySelector("input[type=password]").addEventListener("input",async()=>{'+send+'});':'document.querySelector("button").addEventListener("click",async e=>{e.preventDefault();'+send+'});';
  await page.setContent(kind==='native'?`<!doctype html><form method="post" action="/password">${fields}<button>Sign in</button></form>`:`<!doctype html><form>${fields}<button>Sign in</button></form><script>${event}</script>`);
  let held=false,receiver,receiverConfig,lost=false;
  let currentTarget=targetId,currentGeneration=generation;
  const originalPage={url:()=>page.url(),evaluate:async(fn,args)=>{const result=await page.evaluate(fn,args);if(kind==='lost-submit'&&fn.name==='submitForm'&&!lost){lost=true;throw Error('Synthetic lost submit acknowledgement');}return result;},locator(selector){const locator=page.locator(selector);if(kind!=='lost-fill'||selector!=='input')return locator;return {nth(index){const field=locator.nth(index);return {async fill(value){await field.fill(value);if(index===1&&!lost){lost=true;throw Error('Synthetic lost fill acknowledgement');}}};}};}};
  const identity=async()=>{
   if(await page.locator('input[type=password],input[autocomplete=one-time-code]').count())return null;
   const principal=await page.evaluate(async()=>await(await fetch('/whoami')).json());
   return principal.username==='synthetic-user'?{origin,username:principal.username,organization:principal.organization}:null;
  };
  const f=await createOwnerAuthFixture({configure:async({root,state,receiving,sourcePort})=>{
   Object.assign(receiving,{tabId:targetId,generation});
   sourcePort.discover=async()=>[{recordId:'synthetic:billing',origin,name:'Synthetic Chrome',account:'synthetic-user',modifiedAt:1}];
   sourcePort.read=async(_id,{authorize})=>{await authorize();state.reads++;return {username:'synthetic-user',password:'SYNTHETIC-ONLY-PASSWORD'};};
   receiverConfig={store:await openPrivateTransactions({directory:root,name:'chrome-receiving'}),identityKey:randomBytes(32),resolveTarget:async()=>({page:originalPage,resourceId:currentTarget,generation:currentGeneration}),privacy:{begin:async()=>{held=true;return true;},readCaptureSafety:async()=>held,finish:async()=>{if(!await identity())return false;held=false;return true;}},readIdentity:identity,actionEvidence:true};
   receiver=await createGenericBrowserReceiver(receiverConfig);return {receiver:{validate:(...args)=>receiver.validate(...args),use:(...args)=>receiver.use(...args),reconcile:(...args)=>receiver.reconcile(...args)}};
  }});
  try{
   const before=counts.password;
   Object.assign(f.options,{origin,query:origin,accountHint:'synthetic-user',idempotencyKey:'chrome-'+kind});
   f.browser.verifyLogin=async()=>{const actual=await identity();const matches=actual&&actual.username===(kind==='wrong-account'?'other-intended-user':'synthetic-user')&&actual.organization===(kind==='wrong-organization'?'other-intended-company':'fixture-org');return {...f.options,tabId:currentTarget,generation:currentGeneration,status:matches?'completed':'waiting',reasonCode:matches?'authenticated':actual?'identity_mismatch':'password_required',effectState:'no_effect'};};
   let playbook=createOwnerLoginPlaybook(f);
   const pending=await playbook.advance(f.options);assert.equal(pending.reasonCode,'approval_required');assert.equal(f.state.reads,0);
   f.respond();await playbook.advance(f.options);
   for(let i=0;i<30&&counts.password===before;i++)await page.waitForTimeout(50);
   assert.equal(counts.password,before+(kind==='unhandled'?0:1),kind+': actual server authentication operation required');
   if(kind!=='native'&&kind!=='unhandled')await page.waitForSelector('p');
   if(kind==='wrong-account'||kind==='wrong-organization')assert.deepEqual(await identity(),{origin,username:'synthetic-user',organization:'fixture-org'},'Independent server identifies the account and company that differ from intended identity');
   if(kind==='changed-target'){const session=await page.context().newCDPSession(chrome.peer);currentTarget=(await session.send('Target.getTargetInfo')).targetInfo.targetId;assert.notEqual(currentTarget,targetId);currentGeneration=generation+':replacement';await session.detach();}
   await f.restart();receiver=await createGenericBrowserReceiver(receiverConfig);
   playbook=createOwnerLoginPlaybook(f);
   // Lost acknowledgement recovery reconnects from another OS process, never the current SDK instance.
   const resumed=kind.startsWith('lost-')?await resumeOwnerLoginInFreshProcess(f):await playbook.advance(f.options);assert.equal(resumed.requestId,pending.requestId,kind+': retain original request');await playbook.advance(f.options);
   const original=await f.auth.helper('inspect_credential_request',{requestId:resumed.requestId});
   assert.equal(original.browserActionEvidence?.contractVersion,1,kind+': versioned input/submission evidence');
   assert.equal(resumed.browserActionEvidence?.contractVersion,1,kind+': consumer exposes bounded action evidence');
   assert.equal(original.browserActionEvidence.generation,generation);
   assert.equal(original.browserActionEvidence.submission,kind==='lost-submit'?'unknown':kind==='lost-fill'?'not_attempted':'acknowledged');
   assert.equal(counts.password,before+(kind==='unhandled'?0:1),kind+': restart cannot repeat submission');assert.equal(f.state.reads,1,kind+': no replacement credential read');
   if(['native','unhandled','wrong-account','wrong-organization','changed-target'].includes(kind))assert.notEqual(resumed.status,'completed',kind+': incomplete, mismatched or changed actual browser cannot complete');
   else assert.equal(resumed.status,'completed',kind+': independent identity reconciles the original attempt');
   assert.equal(urls.some(url=>url.includes('SYNTHETIC')||url.includes('synthetic-user')),false,'No protected fields in native GET URLs');
  }finally{await f.close();}
 }
 console.log('Actual Chrome: native POST, JavaScript submit/click, fresh OS-process lost fill/submit recovery and held unhandled/account/company/target negatives retain original requests without repeated effects');
});}finally{await rm(artifacts,{recursive:true,force:true});}
