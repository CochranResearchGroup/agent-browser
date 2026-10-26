import assert from 'node:assert/strict';
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {createServer} from 'node:http';
import {writeTeachingDraftCandidate} from '../packages/client/src/teaching-draft.js';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
import {createTeachingReviewClient} from '../packages/client/src/teaching-review.js';
import {createTeachingGrillingClient} from '../packages/client/src/teaching-grilling.js';
const directory=await mkdtemp(join(tmpdir(),'teaching-grilling-'));
const original=JSON.parse(await readFile(new URL('./fixtures/teaching-report/draft.json',import.meta.url),'utf8'));
original.teaching.unresolved.push({id:'refunds-exception',question:'When the requested refunds option is unavailable, should the run stop or use another report?',interpretation:'The recording shows only the normal checked option. Recommend stopping rather than changing the report meaning silently.',evidenceRefs:['dates','verify']});
original.teaching.unresolved.push(
 {id:'export-verification',question:'Which CSV columns and saved-file checks establish delivery of the requested report?',interpretation:'CSV prepared is only a screen marker. Recommend verifying the actual saved CSV dates, identity and content before completion.',evidenceRefs:['verify','export']},
 {id:'export-recovery',question:'If the expected CSV artifact is absent or mismatched, should the run stop for assistance before another export?',interpretation:'No recovery is shown. Recommend preserving the original operation and inspecting its outcome rather than blindly repeating export.',evidenceRefs:['export']},
);
const answers=new Map();let posts=0;
const server=createServer(async(req,res)=>{
 res.setHeader('content-type','application/json');
 if(req.method==='POST'&&req.url==='/answers'){
  if(req.headers.authorization!=='Bearer synthetic-human'||req.headers.origin!=='https://operator.test'){res.writeHead(403);res.end('{}');return;}
  let raw='';for await(const chunk of req)raw+=chunk;const input=JSON.parse(raw);
  if(input.intent!=='answer_exact_question'||typeof input.answer!=='string'){res.writeHead(400);res.end('{}');return;}
  const value={...input,answerRef:'answer-'+(++posts),actorRef:'synthetic-operator',sourceRef:'operator-event-'+posts,provenance:'authenticated_operator'};
  answers.set(value.answerRef,value);res.end(JSON.stringify(value));return;
 }
 if(req.method==='GET'&&req.url.startsWith('/answers/')){
  if(req.headers.authorization!=='Bearer synthetic-receiver'){res.writeHead(403);res.end('{}');return;}
  const value=answers.get(req.url.split('/').at(-1));res.writeHead(value?200:404);res.end(JSON.stringify(value??{}));return;
 }
 res.writeHead(404);res.end('{}');
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const origin='http://127.0.0.1:'+server.address().port;
try{
 await writeTeachingDraftCandidate({candidateDirectory:directory,candidateOnly:true,guide:original});
 const browser=createPlaybookClient(async(name,args)=>({content:[{type:'text',text:JSON.stringify({success:true,data:args.params.operation==='validate_teaching_candidate'?{valid:true,guideId:args.params.guide.guideId,taskId:args.taskId,executionAllowed:false,operatorReviewQualified:false}:JSON.parse(await readFile(join(directory,args.params.guideId+'.json'),'utf8'))})}]}));
 const retained=JSON.parse(await readFile(new URL('../config/siteguides/bill-session-v1.json',import.meta.url),'utf8'));retained.guideId='accepted-procedure';
 const retainedBytes=JSON.stringify(retained);await writeFile(join(directory,'accepted-procedure.json'),retainedBytes,{mode:0o600});
 await writeFile(join(directory,'peer.fixture'),'untouched peer',{mode:0o600});
 const review=createTeachingReviewClient({candidateDirectory:directory,browser,acceptedVersion:{guideId:'accepted-procedure',sha256:createHash('sha256').update(retainedBytes).digest('hex')}});
 const readOperatorAnswer=async request=>{const response=await fetch(origin+'/answers/'+request.answerRef,{headers:{authorization:'Bearer synthetic-receiver'},redirect:'error'});if(!response.ok)throw Error('missing');return response.json();};
 const client=createTeachingGrillingClient({review,readOperatorAnswer});
 await assert.rejects(client.open({taskId:'peer-task',guideId:original.guideId}),/grilling_source_task_mismatch/);
 const first=await review.open({taskId:'task',guideId:original.guideId});
 assert.equal(first.guide.teaching.routines[1].steps[0].parameter,'startDate','useful variable-date draft exists before questions');
 assert.equal(first.guide.teaching.parameters.find(p=>p.id==='startDate').example,'2026-05-01');
 assert.equal(first.guide.teaching.parameters.find(p=>p.id==='endDate').example,'2026-05-31');
 assert.match(first.guide.teaching.evidence.find(e=>e.id==='verify').summary,/USD/);
 assert.match(first.guide.teaching.claims.find(c=>c.id==='dates-variable').statement,/newly requested dates/);
 await assert.rejects(client.prepare({taskId:'task',guideId:first.guideId,expectedSha256:first.sha256,operationId:'unrelated-question',questions:[{id:'refunds-policy',category:'missing_intent',stepRefs:['choose-report:1'],dependsOn:[]}]}),/question_source_invalid/);
 await assert.rejects(client.prepare({taskId:'task',guideId:first.guideId,expectedSha256:first.sha256,operationId:'known-fact',questions:[{id:'report-kind',category:'ambiguity',stepRefs:['choose-report:1'],dependsOn:[]}]}),/question_source_invalid/);
 await assert.rejects(client.prepare({taskId:'task',guideId:first.guideId,expectedSha256:first.sha256,operationId:'cycle',questions:[{id:'refunds-policy',category:'missing_intent',stepRefs:['set-report-period:2'],dependsOn:['refunds-policy']}]}),/question_source_invalid/);
 const prepared=await client.prepare({taskId:'task',guideId:first.guideId,expectedSha256:first.sha256,operationId:'questions-1',questions:[
  {id:'refunds-policy',category:'missing_intent',stepRefs:['set-report-period:2'],dependsOn:[]},
  {id:'date-boundary-policy',category:'variable_input',stepRefs:['set-report-period:1'],dependsOn:[]},
  {id:'refunds-exception',category:'exception',stepRefs:['set-report-period:2'],dependsOn:['refunds-policy']},
  {id:'export-verification',category:'outcome',stepRefs:['export-and-verify:0'],dependsOn:['date-boundary-policy']},
  {id:'export-recovery',category:'recovery',stepRefs:['export-and-verify:0'],dependsOn:['export-verification']},
 ]});
 assert.equal(prepared.status,'saved');
 const round=await client.open({taskId:'task',guideId:prepared.guideId});
 assert.deepEqual(round.round.map(q=>q.id),['refunds-policy','date-boundary-policy']);
 assert.match(round.round[0].recommendation,/exposing includeRefunds/);
 assert.equal(round.round[0].recommendationAdopted,false);assert.equal(posts,0);
 assert.equal(round.round[0].sources[0].startMs,3000);assert.equal(round.round[0].sources[1].startMs,9000);
 assert.equal(round.round.some(q=>/which report type|what were the dates|which currency/i.test(q.question)),false);
 const question=round.round[0];
 const request={intent:'answer_exact_question',taskId:'task',rootGuideId:round.rootGuideId,answeredGuideId:round.guideId,answeredSha256:round.sha256,contentSha256:round.contentSha256,questionId:question.id,questionSha256:question.questionSha256,answer:'The caller chooses includeRefunds separately for each report; never reuse the demonstration setting.'};
 assert.equal((await fetch(origin+'/answers',{method:'POST',headers:{authorization:'Bearer synthetic-receiver',origin:'https://operator.test'},body:JSON.stringify({...request,actorRef:'human'})})).status,403);
 const response=await fetch(origin+'/answers',{method:'POST',headers:{authorization:'Bearer synthetic-human',origin:'https://operator.test'},body:JSON.stringify(request)});
 const answer=await response.json();
 const revised=structuredClone(round.guide);
 revised.teaching.parameters.find(p=>p.id==='includeRefunds').meaning='Caller must explicitly choose includeRefunds for every report; no remembered default.';
 revised.teaching.routines[1].preconditions[2]='Caller explicitly supplied includeRefunds for this report.';
 revised.teaching.claims.find(c=>c.id==='refunds-default').statement='Operator clarification requires a separate caller refunds choice for each report; the recording itself establishes only its checked example.';
 const childInput={taskId:'task',guideId:round.guideId,expectedSha256:round.sha256,operationId:'answer-1',questionId:question.id,answerRef:answer.answerRef,revisedGuide:revised};
 const inputPath=join(directory,'answer-request.fixture');await writeFile(inputPath,JSON.stringify({candidateDirectory:directory,origin,input:childInput}),{mode:0o600});
 const child=spawn(process.execPath,[new URL('./fixtures/teaching-grilling-answer-child.mjs',import.meta.url).pathname,inputPath],{stdio:['ignore','pipe','pipe']});
 let stderr='';child.stderr.on('data',chunk=>stderr+=chunk);
 try{
  await new Promise((resolve,reject)=>{
   const timer=setTimeout(()=>reject(Error('child_checkpoint_timeout '+stderr)),15000);
   child.stdout.on('data',chunk=>{if(chunk.toString().includes('published-without-reply')){clearTimeout(timer);resolve();}});
   child.once('exit',code=>{clearTimeout(timer);reject(Error('child_early_exit '+code+' '+stderr));});
  });
 }finally{child.kill('SIGTERM');await new Promise(resolve=>child.once('exit',resolve));}
 const answered=await client.lookupOperation({rootGuideId:round.rootGuideId,operationId:'answer-1'});
 assert.equal(answered.status,'saved');
 const reopened=await createTeachingGrillingClient({review,readOperatorAnswer}).open({taskId:'task',guideId:answered.guideId});
 assert.deepEqual(reopened.round.map(q=>q.id),['date-boundary-policy','refunds-exception']);
 assert.equal(reopened.acceptedVersion.available,true);
 assert.equal(await readFile(join(directory,'accepted-procedure.json'),'utf8'),retainedBytes);
 assert.equal(await readFile(join(directory,'peer.fixture'),'utf8'),'untouched peer');
 assert.equal(reopened.answers[0].actorRef,'synthetic-operator');assert.equal(reopened.answers[0].verification,'verified');
 assert.equal(reopened.answers[0].answer,request.answer);assert.equal(reopened.ordinaryReuseAllowed,false);
 assert.match(reopened.guide.teaching.parameters[2].meaning,/every report/);
 assert.equal(reopened.guide.teaching.unresolved.some(q=>q.id==='refunds-policy'),false);
 assert.equal(reopened.guide.teaching.claims.find(c=>c.id==='refunds-shown').attribution,'observed','historical observed evidence is not rewritten');
 const retry=await client.answer({taskId:'task',guideId:round.guideId,expectedSha256:round.sha256,operationId:'answer-1',questionId:question.id,answerRef:answer.answerRef,revisedGuide:revised});
 assert.equal(retry.guideId,answered.guideId);assert.equal(retry.recoveredOriginal,true);assert.equal(posts,1);
 const mirrored=await createTeachingGrillingClient({review,readOperatorAnswer:async request=>request}).open({taskId:'task',guideId:answered.guideId});
 assert.equal(mirrored.answers[0].verification,'unverified','stored actor/provenance labels are not an authority response');
 const noReader=await createTeachingGrillingClient({review}).open({taskId:'task',guideId:answered.guideId});
 assert.equal(noReader.answers[0].verification,'unverified');assert.equal(noReader.round.length,0);assert.equal(noReader.answers[0].answer,request.answer,'authority outage preserves answer without reasking or assuming consent');
 await assert.rejects(createTeachingGrillingClient({review}).answer({taskId:'task',guideId:round.guideId,expectedSha256:round.sha256,operationId:'forged',questionId:question.id,answerRef:answer.answerRef,revisedGuide:revised}),/authority_unconfigured/);
 const wrong=await createTeachingGrillingClient({review,readOperatorAnswer:async()=>({...answer,answeredSha256:'a'.repeat(64)})});
 await assert.rejects(wrong.answer({taskId:'task',guideId:round.guideId,expectedSha256:round.sha256,operationId:'stale-answer',questionId:question.id,answerRef:answer.answerRef,revisedGuide:revised}),/answer_invalid/);
 assert.equal((await client.lookupOperation({rootGuideId:round.rootGuideId,operationId:'answer-1'})).guideId,answered.guideId);
 assert.deepEqual(JSON.parse(await readFile(join(directory,original.guideId+'.json'),'utf8')),original);
 const changedQuestion=structuredClone(reopened.guide);
 changedQuestion.teaching.clarification.questions[0].question='Should all future runs exclude refunds?';
 const changed=await review.save({taskId:'task',guideId:reopened.guideId,expectedSha256:reopened.sha256,operationId:'changed-question',guide:changedQuestion});
 const inconsistent=await client.open({taskId:'task',guideId:changed.guideId});
 assert.equal(inconsistent.answers[0].verification,'unverified','an old answer cannot settle a changed question');
 assert.equal(inconsistent.round.length,0);
 console.log('Teaching grilling: useful draft precedes focused evidence-linked questions; recommendations are not consent');
}finally{await new Promise(resolve=>server.close(resolve));await rm(directory,{recursive:true,force:true});}
