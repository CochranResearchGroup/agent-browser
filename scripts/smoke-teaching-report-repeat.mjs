#!/usr/bin/env node
/** Owned HTTPS extension of the existing teaching report page; no private portal or speech. */
import assert from 'node:assert/strict';
import {createHash,randomUUID} from 'node:crypto';
import {execFileSync,spawn} from 'node:child_process';
import {createServer as httpsServer,get as httpsGet} from 'node:https';
import {createServer as socketServer,connect} from 'node:net';
import {readFileSync,writeFileSync,mkdirSync,readdirSync,existsSync,statSync,renameSync,unlinkSync,chmodSync,appendFileSync} from 'node:fs';
import {readFile,writeFile,copyFile} from 'node:fs/promises';
import {join,isAbsolute} from 'node:path';
import {createSmokeContext,createMcpStdioClient,runCli} from './smoke-utils.js';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
import {writeTeachingDraftCandidate} from '../packages/client/src/teaching-draft.js';
import {createTeachingReviewClient} from '../packages/client/src/teaching-review.js';

const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const candidate=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD,browser=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(candidate&&isAbsolute(candidate)&&browser&&isAbsolute(browser));
const candidateSha256=hash(readFileSync(candidate)),browserSha256=hash(readFileSync(browser));
assert.equal(candidateSha256,process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_SHA256,'Pin exact immutable combined candidate');
assert.equal(browserSha256,process.env.AGENT_BROWSER_SMOKE_BROWSER_SHA256,'Pin reviewed Chrome');
assert.equal(execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim(),'','Pin a clean consumer tree before runtime effects');
const consumerCommit=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
assert.match(process.env.AGENT_BROWSER_SMOKE_BINARY_SOURCE??'',/^[a-f0-9]{40}$/,'Bind the immutable binary source commit');
const sourceManifest=JSON.parse(readFileSync(new URL('./fixtures/teaching-report/source.json',import.meta.url),'utf8'));
const baseGuide=JSON.parse(readFileSync(new URL('./fixtures/teaching-report/draft.json',import.meta.url),'utf8'));
const evidenceSource=new Map(baseGuide.teaching.evidence.map(reference=>[reference.id,reference]));
const templatePath=new URL('./fixtures/teaching-report/demo.html',import.meta.url),template=readFileSync(templatePath,'utf8'),templateSha256=hash(template);
const context=createSmokeContext({prefix:'ab-report477-',sessionPrefix:'report477'});
const env=Object.fromEntries(Object.entries(context.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')&&!/^(HTTP|HTTPS|ALL|NO)_PROXY$/i.test(key)));
const candidateDirectory=join(context.tempHome,'candidates'),downloads=join(context.tempHome,'downloads');
for(const directory of [candidateDirectory,downloads,join(context.tempHome,'runtime')])mkdirSync(directory,{mode:0o700});
const config=join(context.tempHome,'config.json'),envFile=join(context.tempHome,'fixture.env');writeFileSync(config,'{}');writeFileSync(envFile,'');
Object.assign(env,{AGENT_BROWSER_HOME:context.agentHome,AGENT_BROWSER_SOCKET_DIR:context.socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_EXECUTABLE_PATH:browser,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:candidate,AGENT_BROWSER_SITEGUIDE_DIRECTORY:candidateDirectory,AGENT_BROWSER_CONFIG:config,AGENT_BROWSER_ENV_FILE:envFile,AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS:'0',XDG_RUNTIME_DIR:join(context.tempHome,'runtime'),XDG_CONFIG_HOME:join(context.tempHome,'config'),XDG_DATA_HOME:join(context.tempHome,'data')});context.env=env;
const certPath=join(context.tempHome,'cert.pem'),keyPath=join(context.tempHome,'key.pem');
execFileSync('openssl',['req','-x509','-newkey','rsa:2048','-nodes','-keyout',keyPath,'-out',certPath,'-days','1','-subj','/CN=localhost','-addext','subjectAltName=IP:127.0.0.1,DNS:localhost'],{stdio:'ignore'});chmodSync(keyPath,0o600);
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const expectedCsv='report,date,net_usd,transactions\nDaily sales,2026-06-01,310.00,11\nDaily sales,2026-06-30,420.00,14\n';
const records=[['2026-05-01','125.00',5],['2026-05-31','240.00',8],['2026-06-01','310.00',11],['2026-06-30','420.00',14]];
const reports=new Map(),decisions=new Map(),proofs=new Map(),sessions=new Set(),counts={reports:{},exports:{}};
const operatorSecret=randomUUID(),receiverSecret=randomUUID();let origin;
function runtimePage(){
 const params=new URLSearchParams(location.search),actor=params.get('actor'),variant=params.get('variant');
 const selection=document.querySelector('#select'),configuration=document.querySelector('#configure'),result=document.querySelector('#result'),caption=document.querySelector('#caption');
 const note=document.createElement('textarea');note.id='peer-draft';note.setAttribute('aria-label','Private scratch draft');document.querySelector('main').append(note);
 let current;
 document.querySelector('#open').onclick=()=>{selection.classList.add('hidden');configuration.classList.remove('hidden');};
 document.querySelector('#run').onclick=async()=>{
  const response=await fetch('/report',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({actor,report:document.querySelector('select').value,startDate:document.querySelector('[aria-label="Start date"]').value,endDate:document.querySelector('[aria-label="End date"]').value,includeRefunds:document.querySelector('[aria-label="Include refunds"]').checked})});
  if(!response.ok){caption.textContent='Report request refused';return;}
  current=await response.json();configuration.classList.add('hidden');result.classList.remove('hidden');result.dataset.reportId=current.id;result.dataset.artifactId=current.artifactId;
  result.querySelector('h1').textContent=current.reportName;document.querySelector('#range').textContent=current.startDate+' to '+current.endDate+' · '+current.currency+' · '+(current.includeRefunds?'Includes refunds':'Excludes refunds');
  result.querySelector('tbody').replaceChildren(...current.rows.map(row=>{const tr=document.createElement('tr');for(const value of row){const td=document.createElement('td');td.textContent=value;tr.append(td);}return tr;}));
  if(variant==='changed-export')document.querySelector('#export').textContent='Download workbook';
 };
 document.querySelector('#export').onclick=()=>{if(current)location.href='/csv/'+current.id;};
}
function csv(report){return ['report,date,net_usd,transactions',...report.rows.map(row=>[report.reportName,...row].join(','))].join('\n')+'\n';}
const application=httpsServer({key:readFileSync(keyPath),cert:readFileSync(certPath)},async(req,res)=>{
 try {
  const url=new URL(req.url,'https://fixture'),cookie=/ab_report_session=([^;]+)/.exec(req.headers.cookie??'')?.[1],authenticated=sessions.has(cookie);
  const json=value=>{if(!res.headersSent)res.setHeader('content-type','application/json');res.end(JSON.stringify(value));};
  if(url.pathname==='/portal') {
   if(!authenticated){const session=randomUUID();sessions.add(session);res.setHeader('set-cookie','ab_report_session='+session+'; Secure; HttpOnly; SameSite=Strict; Path=/');}
   res.setHeader('content-type','text/html');res.end(template.replace('<body>','<body data-authenticated-context="synthetic">').replace(/<script>[\s\S]*?<\/script>/,'<script>('+runtimePage.toString()+')()</script>'));return;
  }
  if(url.pathname==='/operator/decisions'&&req.method==='POST') {
   if(req.headers.authorization!=='Bearer '+operatorSecret||req.headers.origin!==origin){res.writeHead(403);json({});return;}
   let text='';for await(const chunk of req)text+=chunk;const value=JSON.parse(text);
   if(value.intent!=='review_exact_candidate'||value.decision!=='approved'){res.writeHead(400);json({});return;}
   const decision={decisionId:'operator-decision-'+(decisions.size+1),decision:'approved',actorRef:'authenticated-fixture-operator',sourceRef:'owned-review-event',provenance:'authenticated_operator',taskId:value.taskId,rootGuideId:value.rootGuideId,reviewedGuideId:value.reviewedGuideId,reviewedSha256:value.reviewedSha256,contentSha256:value.contentSha256,sourceUnavailableAcknowledged:false};decisions.set(decision.decisionId,decision);json(decision);return;
  }
  if(url.pathname.startsWith('/receiver/')) {
   if(req.headers.authorization!=='Bearer '+receiverSecret){res.writeHead(403);json({});return;}
   const [kind,id]=url.pathname.slice('/receiver/'.length).split('/');const value=(kind==='decisions'?decisions:proofs).get(id);res.writeHead(value?200:404);json(value??{});return;
  }
  if(!authenticated){res.writeHead(401);json({authenticated:false});return;}
  if(url.pathname==='/whoami'){json({authenticated:true,account:'training@example.invalid',organization:'north-warehouse'});return;}
  if(url.pathname==='/report'&&req.method==='POST') {
   assert.equal(req.headers.origin,origin);let body='';for await(const chunk of req)body+=chunk;const input=JSON.parse(body);
   assert.equal(input.report,'Daily sales');assert.match(input.startDate,/^2026-\d\d-\d\d$/);assert.match(input.endDate,/^2026-\d\d-\d\d$/);assert.equal(input.includeRefunds,true);
   const report={id:'report-'+(reports.size+1),actor:input.actor,reportId:'daily-sales',reportName:'Daily sales',startDate:input.startDate,endDate:input.endDate,includeRefunds:input.includeRefunds,currency:'USD',rows:records.filter(row=>row[0]>=input.startDate&&row[0]<=input.endDate)};report.artifactId='csv-'+report.id;reports.set(report.id,report);counts.reports[input.actor]=(counts.reports[input.actor]??0)+1;json(report);return;
  }
  if(url.pathname.startsWith('/csv/')) {
   const report=reports.get(url.pathname.slice(5));assert(report);counts.exports[report.actor]=(counts.exports[report.actor]??0)+1;
   res.setHeader('content-type','text/csv');res.setHeader('content-disposition','attachment; filename="daily-sales-'+report.startDate+'-'+report.endDate+'.csv"');const exported=report.actor==='stale'?{...report,rows:records.filter(row=>row[0]>='2026-05-01'&&row[0]<='2026-05-31')}:report;res.end(csv(exported));return;
  }
  res.writeHead(404);json({});
 }catch{if(res.headersSent){if(!res.writableEnded)res.destroy();}else{res.writeHead(400);res.end('fixture_request_rejected');}}
});
await new Promise(resolve=>application.listen(0,'127.0.0.1',resolve));origin='https://127.0.0.1:'+application.address().port;
async function receiverRead(kind,id){return new Promise((resolve,reject)=>httpsGet(origin+'/receiver/'+kind+'/'+encodeURIComponent(id),{ca:readFileSync(certPath),headers:{authorization:'Bearer '+receiverSecret}},res=>{let body='';res.on('data',chunk=>body+=chunk);res.on('end',()=>res.statusCode===200?resolve(JSON.parse(body)):reject(Error('trusted_receipt_unavailable')));}).on('error',reject));}
function ownedProcesses(){return readdirSync('/proc').filter(id=>/^\d+$/.test(id)&&Number(id)!==process.pid).filter(id=>{
 try{if(statSync('/proc/'+id).uid!==process.getuid())return false;}catch{return false;}
 try{if(readFileSync('/proc/'+id+'/environ').toString().split('\0').includes('AGENT_BROWSER_HOME='+context.agentHome))return true;}catch{}
 try{return readFileSync('/proc/'+id+'/cmdline').toString().split('\0').some(argument=>argument.startsWith('--user-data-dir='+context.tempHome+'/'));}catch{return false;}
}).map(Number);}
const receipt={schemaVersion:'agent-browser.report-repeat-acceptance.v1',consumerCommit,binarySourceCommit:process.env.AGENT_BROWSER_SMOKE_BINARY_SOURCE,candidateSha256,browserSha256,teachingFixtureSha256:templateSha256,profileMode:'existing_owned_fixture_browser',tls:'exception_confined_to_owned_browser',synthetic:true,teachingSource:'existing_captioned_reference_tone_fixture_not_narrated',downloadPolicy:'owned_initial_allow_normal_HTTP_csv',beforeOwnedProcesses:ownedProcesses(),checks:[]};
let client,chrome,proxy;const tasks=[];
function phase(value){receipt.phase=value;appendFileSync(join(context.tempHome,'fixture-phases.jsonl'),JSON.stringify({at:new Date().toISOString(),phase:value,effectOutcome:'not_inferred_from_phase',counts})+'\n',{mode:0o600});}
async function raw(name,args){let timer;try{return await Promise.race([client.send('tools/call',{name,arguments:args}),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('bounded_fixture_request_timeout')),65000);})]);}finally{clearTimeout(timer);}}
async function call(name,args){const result=await raw(name,args);assert.notEqual(result.isError,true,JSON.stringify(result));const envelope=JSON.parse(result.content.find(item=>item.type==='text').text);assert.equal(envelope.success,true,JSON.stringify(envelope));return envelope.data;}
async function evaluate(taskId,script){const value=await call('browser_command',{taskId,action:'evaluate',params:{script}});return value.result??value;}
async function waitFor(read,accept,label){const deadline=Date.now()+10000;let value;do{value=await read();if(accept(value))return value;await pause(50);}while(Date.now()<deadline);throw Error('fixture_observation_timeout:'+label);}
async function replyLoss(){
 const path=join(context.socketDir,'runtime-host.sock'),original=path+'.original',inode=statSync(path).ino;renameSync(path,original);let drops=0,requests=0,armed=false,lastDroppedSuccess=null;const sockets=new Set();
 const server=socketServer(down=>{const up=connect(original);for(const socket of [down,up]){sockets.add(socket);socket.on('close',()=>sockets.delete(socket));socket.on('error',()=>{});}let request='',response='',drop=false,parsed=false;
  down.on('data',chunk=>{if(!parsed){request+=chunk;if(request.includes('\n')){parsed=true;let command;try{command=JSON.parse(request.slice(0,request.indexOf('\n')));}catch{down.destroy();up.destroy();return;}requests++;if(armed&&command.action==='click'){armed=false;drop=true;}}}up.write(chunk);});
  up.on('data',chunk=>{if(!drop){down.write(chunk);return;}response+=chunk;if(response.includes('\n')){try{lastDroppedSuccess=JSON.parse(response.slice(0,response.indexOf('\n'))).success===true;}catch{lastDroppedSuccess=false;}drops++;down.destroy();up.destroy();}});up.on('end',()=>down.end());down.on('end',()=>up.end());
 });await new Promise(resolve=>server.listen(path,resolve));const proxyInode=statSync(path).ino;
 return {arm(){armed=true;},get drops(){return drops;},get requests(){return requests;},get lastDroppedSuccess(){return lastDroppedSuccess;},async restore(){for(const socket of sockets)socket.destroy();await new Promise(resolve=>server.close(resolve));assert.equal(statSync(path).ino,proxyInode);unlinkSync(path);assert.equal(statSync(original).ino,inode);renameSync(original,path);}};
}
async function configureDownloads(profile){
 const port=readFileSync(join(profile,'DevToolsActivePort'),'utf8').trim().split('\n');
 const ws=new WebSocket('ws://127.0.0.1:'+port[0]+port[1]);
 await new Promise((resolve,reject)=>{ws.addEventListener('open',resolve,{once:true});ws.addEventListener('error',reject,{once:true});});
 try{await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('owned_download_policy_timeout')),5000);ws.addEventListener('message',event=>{const response=JSON.parse(event.data);if(response.id===1){clearTimeout(timer);response.error?reject(Error('owned_download_policy_refused')):resolve();}});ws.send(JSON.stringify({id:1,method:'Browser.setDownloadBehavior',params:{behavior:'allow',downloadPath:downloads,eventsEnabled:true}}));});}finally{ws.close();}
}
const selectors={'report':'select[aria-label="Report"]','open-report':'#open','start-date':'[aria-label="Start date"]','end-date':'[aria-label="End date"]','include-refunds':'[aria-label="Include refunds"]','run-report':'#run','export-csv':'#export'};
async function executeReviewed(view,task,actor,{loseReply=false}={}){
 assert.equal(view.review.state,'approved');assert.equal(view.guide.teaching.unresolved.length,0);
 const identity=await evaluate(task.taskId,"fetch('/whoami').then(response=>response.json())");assert.deepEqual(identity,{authenticated:true,account:'training@example.invalid',organization:'north-warehouse'});
 const parameters={startDate:'2026-06-01',endDate:'2026-06-30',includeRefunds:true},operationId=actor+'-report-submit';let artifact;
 for(const routineId of view.guide.teaching.playbook.routineIds){const routine=view.guide.teaching.routines.find(value=>value.id===routineId);assert(routine);
  for(const step of routine.steps){
   if(step.action==='observe'){await call('browser_snapshot',{taskId:task.taskId});continue;}
   if(step.action==='verify'){
    if(step.target==='downloaded-artifact'){assert(artifact);if(readFileSync(artifact.path,'utf8')!==expectedCsv)return {status:'verification_failed',reason:'downloaded_report_content_mismatch',taskId:task.taskId,operationId,artifactSha256:hash(readFileSync(artifact.path)),nextAction:'Retain the original export and ask the operator to inspect the report cache/date filters; do not silently rerun the report.'};}
    else {await waitFor(()=>evaluate(task.taskId,"({visible:!document.querySelector('#result').classList.contains('hidden'),title:document.querySelector('#result h1').textContent,range:document.querySelector('#range').textContent,reportId:document.querySelector('#result').dataset.reportId})"),value=>value.visible&&value.title==='Daily sales'&&value.range.includes(parameters.startDate)&&value.range.includes(parameters.endDate),'exact_report_and_period');}
    continue;
   }
   const selector=selectors[step.target];assert(selector,'Only existing reviewed controls are supported');
   const control=view.guide.teaching.controls.find(value=>value.id===step.target);assert(control);
   const observedName=await evaluate(task.taskId,'document.querySelector('+JSON.stringify(selector)+').getAttribute("aria-label") || document.querySelector('+JSON.stringify(selector)+').textContent.trim()');
   if(observedName!==control.name)return {status:'assistance_required',reason:'reviewed_control_changed',taskId:task.taskId,controlId:control.id,expectedName:control.name,observedName,nextAction:'Ask the operator to inspect the renamed export control and supported file format before any export.'};
   if(step.action==='select'){await call('browser_command',{taskId:task.taskId,action:'select',params:{selector,values:['Daily sales']},requestId:actor+'-select-report'});continue;}
   if(step.action==='fill'){await call('browser_fill',{taskId:task.taskId,selector,value:parameters[step.parameter],requestId:actor+'-'+step.target});continue;}
   if(step.action==='set_checked'){await call('browser_command',{taskId:task.taskId,action:parameters[step.parameter]?'check':'uncheck',params:{selector},requestId:actor+'-refunds'});continue;}
   assert.equal(step.action,'activate');
   if(step.target==='export-csv'){artifact=await call('browser_command',{taskId:task.taskId,action:'download',params:{selector,path:join(context.tempHome,actor+'.csv')},requestId:actor+'-export'});continue;}
   if(step.target==='run-report'&&loseReply){proxy.arm();let uncertain=false;try{uncertain=(await raw('browser_click',{taskId:task.taskId,selector,requestId:operationId})).isError===true;}catch{uncertain=true;}assert(uncertain);assert.equal(proxy.lastDroppedSuccess,true,'Reply loss follows a successful owner response');const original=await call('browser_command',{taskId:task.taskId,action:'task_operation',params:{requestId:operationId}});assert.equal(original.operation.state,'uncertain');const sent=proxy.requests;await raw('browser_click',{taskId:task.taskId,selector,requestId:operationId});assert.equal(proxy.requests,sent,'Original submission never repeats through transport');}
   else await call('browser_click',{taskId:task.taskId,selector,requestId:step.target==='run-report'?operationId:actor+'-'+step.target});
  }
 }
 return {status:'completed',artifact,operationId,parameters,observedReport:await evaluate(task.taskId,"({reportId:document.querySelector('#result').dataset.reportId,artifactId:document.querySelector('#result').dataset.artifactId})")};
}
try{
 phase('owned_profile');
 const profileName='report-repeat-owned-synthetic-portal-validation';
 const created=JSON.parse((await runCli(context,['--json','runtime','create',profileName])).stdout),profile=created.userDataDir??created.data?.userDataDir;assert(profile.startsWith(context.tempHome+'/'));mkdirSync(profile,{recursive:true});
 chrome=spawn(browser,['--headless=new','--no-sandbox','--disable-dev-shm-usage','--disable-background-networking','--password-store=basic','--no-first-run','--no-default-browser-check','--remote-debugging-port=0','--user-data-dir='+profile,'--ignore-certificate-errors','about:blank'],{env,stdio:'ignore'});
 await waitFor(async()=>existsSync(join(profile,'DevToolsActivePort')),Boolean,'owned_chrome_ready');await configureDownloads(profile);
 client=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{}});await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'report-repeat-fixture',version:'1'}});client.notify('notifications/initialized');
 const common={profile:profileName,profileMode:'existing',serviceName:'ReportRepeatFixture',agentName:'synthetic-agent',taskName:'Run reviewed report for June'};
 const reviewTask=await call('browser_open',{...common,taskKey:'review',url:origin+'/portal?actor=review'});tasks.push(reviewTask);
 const freshTask=await call('browser_open',{...common,taskKey:'repeat',url:origin+'/portal?actor=repeat'});tasks.push(freshTask);
 const peer=await call('browser_open',{...common,taskKey:'peer',url:origin+'/portal?actor=peer'});tasks.push(peer);
 assert.notEqual(freshTask.taskId,reviewTask.taskId);assert.equal(freshTask.browserId,peer.browserId);assert.notEqual(freshTask.tabId,peer.tabId);assert(freshTask.taskId.length>128);receipt.taskIdLengths={review:reviewTask.taskId.length,fresh:freshTask.taskId.length};
 await call('browser_fill',{taskId:peer.taskId,selector:'#peer-draft',value:'Preserve this unsaved peer note',requestId:'peer-note'});
 const guide=structuredClone(baseGuide);guide.guideId='owned-daily-sales-repeat-v1';guide.allowedOrigins=[origin];guide.teaching.unresolved=[];guide.teaching.limitations.push('Receiver-reviewed fixture policy uses UTC inclusive dates and explicit caller refunds choice; no real Auth or spoken narration qualification.');
 await writeTeachingDraftCandidate({candidateDirectory,candidateOnly:true,guide});
 const accepted=JSON.parse(readFileSync(new URL('../config/siteguides/bill-session-v1.json',import.meta.url),'utf8'));accepted.guideId='retained-accepted-v0';const acceptedBytes=JSON.stringify(accepted);writeFileSync(join(candidateDirectory,accepted.guideId+'.json'),acceptedBytes,{mode:0o600});
 const options={candidateDirectory,browser:createPlaybookClient(raw),acceptedVersion:{guideId:accepted.guideId,sha256:hash(acceptedBytes)},sourceStatus:async request=>{
   const bound=request.recordingId===sourceManifest.recordingId&&request.videoSha256===sourceManifest.sha256&&request.sourceTaskId===baseGuide.teaching.sourceTaskId;
   const videoAvailable=hash(await readFile(new URL('./fixtures/teaching-report/'+sourceManifest.artifact,import.meta.url)))===sourceManifest.sha256;
   return {recordingId:sourceManifest.recordingId,videoSha256:sourceManifest.sha256,sourceTaskId:baseGuide.teaching.sourceTaskId,state:bound&&videoAvailable?'available':'unavailable',evidence:await Promise.all(request.evidence.map(async reference=>{
    const known=evidenceSource.get(reference.id);if(!known||known.artifact!==reference.artifact||known.sha256!==reference.sha256)return {...reference,state:'unavailable',reason:'source_binding_changed'};
    return {id:known.id,artifact:known.artifact,sha256:hash(await readFile(new URL('./fixtures/teaching-report/'+known.artifact,import.meta.url))),state:'available'};
   }))};
  },readOperatorDecision:request=>receiverRead('decisions',request.decisionRef),readFreshRepeat:request=>receiverRead('proofs',request.validationRef)};
 const review=createTeachingReviewClient(options),initial=await review.open({taskId:reviewTask.taskId,guideId:guide.guideId});
 const decision=await new Promise((resolve,reject)=>{import('node:https').then(({request})=>{const req=request(origin+'/operator/decisions',{method:'POST',ca:readFileSync(certPath),headers:{authorization:'Bearer '+operatorSecret,origin,'content-type':'application/json'}},res=>{let body='';res.on('data',chunk=>body+=chunk);res.on('end',()=>res.statusCode===200?resolve(JSON.parse(body)):reject(Error('operator_review_refused')));});req.on('error',reject);req.end(JSON.stringify({intent:'review_exact_candidate',decision:'approved',taskId:reviewTask.taskId,rootGuideId:initial.rootGuideId,reviewedGuideId:initial.guideId,reviewedSha256:initial.sha256,contentSha256:initial.contentSha256}));});});
 const recorded=await review.recordDecision({taskId:reviewTask.taskId,guideId:initial.guideId,expectedSha256:initial.sha256,operationId:'explicit-operator-review',decisionRef:decision.decisionId});
 const reviewed=await review.open({taskId:reviewTask.taskId,guideId:recorded.guideId});assert.equal(reviewed.review.state,'approved');assert.equal(reviewed.ordinaryReuseAllowed,false);
 await assert.rejects(review.promote({taskId:freshTask.taskId,guideId:reviewed.guideId,expectedSha256:reviewed.sha256,operationId:'premature-promote',passed:true}),/fresh_repeat_required/);
 assert.equal(readFileSync(join(candidateDirectory,accepted.guideId+'.json'),'utf8'),acceptedBytes);
 phase('wrong_csv_refusal');
 const staleTask=await call('browser_open',{...common,taskKey:'stale',url:origin+'/portal?actor=stale'});tasks.push(staleTask);
 const staleRun=await executeReviewed(reviewed,staleTask,'stale');assert.equal(staleRun.status,'verification_failed');assert.equal(counts.reports.stale,1);assert.equal(counts.exports.stale,1);
 await assert.rejects(review.recordFreshRepeat({taskId:staleTask.taskId,guideId:reviewed.guideId,expectedSha256:reviewed.sha256,operationId:'reject-wrong-csv',validationRef:'download-event-with-wrong-period',requestedStartDate:'2026-06-01',requestedEndDate:'2026-06-30',reportId:'daily-sales',passed:true}),/repeat_receipt_unavailable/);
 assert.equal((await review.open({taskId:reviewTask.taskId,guideId:reviewed.guideId})).acceptedVersion.guideId,accepted.guideId);
 receipt.checks.push({kind:'wrong_csv_refusal',downloadEventObserved:true,requestedJuneButCsvMay:true,independentOracleRejected:true,acceptedUnchanged:true,reportSubmissions:1,exports:1});
 phase('fresh_repeat');proxy=await replyLoss();const run=await executeReviewed(reviewed,freshTask,'repeat',{loseReply:true});assert.equal(run.status,'completed');assert.equal(proxy.drops,1);await proxy.restore();proxy=null;
 const actualReport=reports.get(run.observedReport.reportId);assert(actualReport);assert.equal(actualReport.reportId,'daily-sales');assert.equal(actualReport.startDate,'2026-06-01');assert.equal(actualReport.endDate,'2026-06-30');assert.equal(run.observedReport.artifactId,actualReport.artifactId);assert.equal(counts.reports.repeat,1);assert.equal(counts.exports.repeat,1);assert.equal(readFileSync(run.artifact.path,'utf8'),expectedCsv);
 const proof={schemaVersion:'agent-browser.teaching-fresh-repeat.v1',validationId:'independently-verified-june-report',provenance:'receiving_application_verifier',taskId:freshTask.taskId,sourceTaskId:guide.teaching.sourceTaskId,rootGuideId:reviewed.rootGuideId,executedGuideId:reviewed.guideId,executedSha256:reviewed.sha256,contentSha256:reviewed.contentSha256,reviewDecisionId:decision.decisionId,playbookId:guide.teaching.playbook.id,requestedStartDate:'2026-06-01',requestedEndDate:'2026-06-30',reportId:actualReport.reportId,artifactId:actualReport.artifactId,artifactSha256:hash(readFileSync(run.artifact.path)),artifactMediaType:'text/csv',originalOperationId:run.operationId,verification:'report_identity_requested_dates_and_content'};proofs.set(proof.validationId,proof);
 const repeated=await review.recordFreshRepeat({taskId:freshTask.taskId,guideId:reviewed.guideId,expectedSha256:reviewed.sha256,operationId:'retain-verified-repeat',validationRef:proof.validationId,requestedStartDate:'2026-06-01',requestedEndDate:'2026-06-30',reportId:'daily-sales'});
 const repeatedView=await review.open({taskId:freshTask.taskId,guideId:repeated.guideId});assert.equal(repeatedView.validation.state,'verified');assert.equal(repeatedView.ordinaryReuseAllowed,false);assert.equal(repeatedView.acceptedVersion.guideId,accepted.guideId);
 const promoted=await review.promote({taskId:freshTask.taskId,guideId:repeated.guideId,expectedSha256:repeatedView.sha256,operationId:'promote-verified-repeat'});assert.equal(promoted.status,'promoted');const promotedView=await createTeachingReviewClient(options).open({taskId:freshTask.taskId,guideId:promoted.guideId});assert.equal(promotedView.acceptedVersion.guideId,reviewed.guideId);assert.equal(promotedView.ordinaryReuseAllowed,true);
 const staticValidation=await options.browser.validateTeachingCandidate({taskId:freshTask.taskId,guide:promotedView.guide});assert.equal(staticValidation.executionAllowed,false);assert.equal(staticValidation.operatorReviewQualified,false);
 const untrusted=await createTeachingReviewClient({...options,readFreshRepeat:undefined}).open({taskId:freshTask.taskId,guideId:promoted.guideId});assert.equal(untrusted.ordinaryReuseAllowed,false);assert.equal(untrusted.acceptedVersion.guideId,accepted.guideId);
 const corrupt={...proof,requestedStartDate:'2026-05-01'};proofs.set(proof.validationId,corrupt);assert.equal((await review.open({taskId:freshTask.taskId,guideId:promoted.guideId})).ordinaryReuseAllowed,false);proofs.set(proof.validationId,proof);
 receipt.checks.push({kind:'fresh_repeat',newTask:true,differentDates:true,report:'daily-sales',period:['2026-06-01','2026-06-30'],csvSha256:proof.artifactSha256,independentContentVerified:true,reportSubmissions:1,exports:1,originalSubmissionUncertain:true,trustedReviewAndProofRequired:true,stockMcpRemainsInert:true});
 phase('changed_page');const changed=await call('browser_open',{...common,taskKey:'changed',url:origin+'/portal?actor=changed&variant=changed-export'});tasks.push(changed);const assistance=await executeReviewed(reviewed,changed,'changed');assert.equal(assistance.status,'assistance_required');assert.equal(assistance.controlId,'export-csv');assert.equal(counts.exports.changed??0,0);receipt.checks.push({kind:'changed_page',...assistance,exportEffects:0});
 assert(JSON.stringify(await call('browser_snapshot',{taskId:peer.taskId})).includes('Preserve this unsaved peer note'));assert.equal(counts.reports.repeat,1);assert.equal(readFileSync(join(candidateDirectory,accepted.guideId+'.json'),'utf8'),acceptedBytes);assert.equal(hash(readFileSync(templatePath)),templateSha256);
 receipt.checks.push({kind:'preservation',peerDraft:true,priorAcceptedBytes:true,originalTeachingPage:true});phase('complete');receipt.status='pass';
}catch(error){receipt.status='failed';receipt.error=error.message;receipt.counts=counts;throw error;}
finally{
 if(proxy)await proxy.restore();if(client){for(const task of tasks.reverse()){try{await Promise.race([call('browser_close',{taskId:task.taskId}),pause(3000).then(()=>{throw Error('cleanup_deadline');})]);}catch{}}client.close();}
 if(chrome&&chrome.exitCode===null)chrome.kill('SIGTERM');for(const pid of ownedProcesses()){try{if(ownedProcesses().includes(pid))process.kill(pid,'SIGTERM');}catch{}}
 for(let i=0;i<100&&ownedProcesses().length;i++)await pause(50);receipt.afterOwnedProcesses=ownedProcesses();await new Promise(resolve=>application.close(resolve));assert.equal(hash(readFileSync(candidate)),candidateSha256);
 const out=join('/tmp/agent-browser-spec466-implementation','report477-'+randomUUID()+'.json');
 if(receipt.afterOwnedProcesses.length){receipt.status='failed';receipt.cleanupFailure='owned_processes_remain';}
 else {try{context.cleanupTempHome();receipt.privateHomeRemoved=!existsSync(context.tempHome);}catch{receipt.status='failed';receipt.cleanupFailure='private_home_removal_failed';}}
 writeFileSync(out,JSON.stringify(receipt,null,2)+'\n',{mode:0o600,flag:'wx'});console.log(JSON.stringify({status:receipt.status,receiptPath:out,ownedResidue:receipt.afterOwnedProcesses.length,checks:receipt.checks}));assert.equal(receipt.afterOwnedProcesses.length,0);assert.equal(receipt.privateHomeRemoved,true);
}
