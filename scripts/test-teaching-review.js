import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtemp,readFile,writeFile,rm,mkdir,copyFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createServer} from 'node:http';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {writeTeachingDraftCandidate} from '../packages/client/src/teaching-draft.js';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
import {createTeachingReviewClient} from '../packages/client/src/teaching-review.js';
import {createTeachingGrillingClient} from '../packages/client/src/teaching-grilling.js';
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const original=JSON.parse(await readFile(new URL('./fixtures/teaching-report/draft.json',import.meta.url),'utf8'));
const directory=await mkdtemp(join(tmpdir(),'teaching-review-public-'));
const decisions=new Map(),answers=new Map();let operatorPosts=0;
const server=createServer(async(req,res)=>{
 res.setHeader('content-type','application/json');
 const url=new URL(req.url,'http://fixture');
 if(req.method==='POST'&&url.pathname==='/decisions') {
  if(req.headers.authorization!=='Bearer synthetic-human'||req.headers.origin!=='https://operator.test'){res.writeHead(403);res.end('{}');return;}
  let raw='';for await(const chunk of req)raw+=chunk;
  const value=JSON.parse(raw);
  if(value.intent!=='review_exact_candidate'||!['approved','rejected'].includes(value.decision)){res.writeHead(400);res.end('{}');return;}
  operatorPosts++;
  const record={...value,decisionId:'decision-'+operatorPosts,actorRef:'authenticated-human-1',sourceRef:'owner-review-'+operatorPosts,provenance:'authenticated_operator',sourceUnavailableAcknowledged:value.sourceUnavailableAcknowledged===true};
  decisions.set(record.decisionId,record);res.end(JSON.stringify(record));return;
 }
 if(req.method==='GET'&&url.pathname.startsWith('/decisions/')) {
  if(req.headers.authorization!=='Bearer synthetic-receiver'){res.writeHead(403);res.end('{}');return;}
  const value=decisions.get(url.pathname.split('/').at(-1));res.writeHead(value?200:404);res.end(JSON.stringify(value??{}));return;
 }
 if(req.method==='POST'&&url.pathname==='/answers') {
  if(req.headers.authorization!=='Bearer synthetic-human'||req.headers.origin!=='https://operator.test'){res.writeHead(403);res.end('{}');return;}
  let raw='';for await(const chunk of req)raw+=chunk;const value=JSON.parse(raw);
  const record={...value,answerRef:'answer-'+(answers.size+1),actorRef:'authenticated-human-1',sourceRef:'owner-answer-'+(answers.size+1),provenance:'authenticated_operator'};
  answers.set(record.answerRef,record);res.end(JSON.stringify(record));return;
 }
 if(req.method==='GET'&&url.pathname.startsWith('/answers/')) {
  if(req.headers.authorization!=='Bearer synthetic-receiver'){res.writeHead(403);res.end('{}');return;}
  const value=answers.get(url.pathname.split('/').at(-1));res.writeHead(value?200:404);res.end(JSON.stringify(value??{}));return;
 }
 res.writeHead(404);res.end('{}');
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const origin='http://127.0.0.1:'+server.address().port;
try {
 await writeTeachingDraftCandidate({candidateDirectory:directory,candidateOnly:true,guide:original});
 const evidenceDirectory=join(directory,'derived-evidence');await mkdir(evidenceDirectory,{mode:0o700});
 for(const reference of original.teaching.evidence)await copyFile(new URL('./fixtures/teaching-report/'+reference.artifact,import.meta.url),join(evidenceDirectory,reference.artifact));
 const accepted=JSON.parse(await readFile(new URL('../config/siteguides/bill-session-v1.json',import.meta.url),'utf8'));
 accepted.guideId='retained-accepted-v0';
 const acceptedBytes=JSON.stringify(accepted);await writeFile(join(directory,accepted.guideId+'.json'),acceptedBytes,{mode:0o600});
 const peer=Buffer.from('unrelated candidate bytes');await writeFile(join(directory,'peer.txt'),peer);
 const browser=createPlaybookClient(async(name,args)=>({content:[{type:'text',text:JSON.stringify({success:true,data:args.params.operation==='validate_teaching_candidate'?{valid:true,guideId:args.params.guide.guideId,taskId:args.taskId,executionAllowed:false,operatorReviewQualified:false}:{authenticatedSelector:null,account:null,organization:null,controls:null,retrieval:null,...JSON.parse(await readFile(join(directory,args.params.guideId+'.json'),'utf8'))}})}]}));
 let sourceAvailable=true,sourceReason='source_deleted';
 const options={candidateDirectory:directory,browser,acceptedVersion:{guideId:accepted.guideId,sha256:hash(acceptedBytes)},sourceStatus:async request=>({...request,state:sourceAvailable?'available':'unavailable',reason:sourceAvailable?'scoped_read':sourceReason,gaps:[{tMs:6000,scope:'audio',reason:'synthetic_owner_gap'}],evidence:await Promise.all((request.evidence??original.teaching.evidence).map(async reference=>{
   try {const bytes=await readFile(join(evidenceDirectory,reference.artifact));return {id:reference.id,artifact:reference.artifact,sha256:hash(bytes),state:'available'};}
   catch {return {id:reference.id,artifact:reference.artifact,sha256:reference.sha256,state:'unavailable',reason:'source_deleted'};}
  }))}),readOperatorDecision:async request=>{
  const response=await fetch(origin+'/decisions/'+encodeURIComponent(request.decisionRef),{headers:{authorization:'Bearer synthetic-receiver'},redirect:'error'});
  if(!response.ok)throw Error('operator decision unavailable');return response.json();
 }};
 const client=createTeachingReviewClient(options);
 const view=await client.open({taskId:'task',guideId:original.guideId});
 assert.equal(view.review.state,'not_reviewed');assert.equal(view.ordinaryReuseAllowed,false);assert.equal(operatorPosts,0);
 assert.equal(view.steps[0].explanation,original.teaching.routines[0].steps[0].description);
 assert.equal(view.steps[0].expectedResult.scope,'routine');assert.equal(view.steps[0].evidence[0].id,'choose');
 assert.equal(view.steps[0].sourceGaps[0].reason,'synthetic_owner_gap');assert.equal(view.proposals[0].adopted,false);
 const forged=structuredClone(view.guide);forged.teaching.review={decision:'approved'};
 await assert.rejects(writeTeachingDraftCandidate({candidateDirectory:directory,candidateOnly:true,guide:forged}),/candidate_invalid/);
 await assert.rejects(client.save({taskId:'task',guideId:original.guideId,expectedSha256:view.sha256,operationId:'forged-review',guide:forged}),/metadata_not_editable/);
 const edited=structuredClone(view.guide);
 edited.teaching.routines[1].steps[0].description='Use the caller startDate; never silently reuse the May example.';
 edited.teaching.routines[1].steps[0].expectedResult='The requested startDate is visible in its control.';
 edited.teaching.routines[0].steps.splice(0,1);
 edited.teaching.parameters.push({id:'currencyCode',kind:'text',example:'USD',meaning:'Explicit supported report currency; USD was observed in this demonstration.',evidenceRefs:['verify']});
 const saved=await client.save({taskId:'task',guideId:original.guideId,expectedSha256:view.sha256,operationId:'edit-1',guide:edited});
 assert.equal(saved.status,'saved');assert.equal(saved.revision,2);
 const reopened=createTeachingReviewClient(options);
 const current=await reopened.open({taskId:'task',guideId:saved.guideId});
 assert.equal(current.guide.teaching.routines[0].steps.length,2);
 assert.match(current.guide.teaching.routines[1].steps[0].description,/caller startDate/);
 assert.equal(current.guide.teaching.parameters.at(-1).id,'currencyCode');
 assert.equal(current.review.state,'not_reviewed');assert.equal(current.ordinaryReuseAllowed,false);
 assert.equal((await reopened.save({taskId:'task',guideId:original.guideId,expectedSha256:view.sha256,operationId:'edit-1',guide:edited})).guideId,saved.guideId,'same operation reopens original save');
 const stale=await reopened.save({taskId:'task',guideId:original.guideId,expectedSha256:view.sha256,operationId:'stale-edit',guide:edited});
 assert.equal(stale.status,'conflict');
 const unconfigured=createTeachingReviewClient({...options,readOperatorDecision:undefined});
 await assert.rejects(unconfigured.recordDecision({taskId:'task',guideId:saved.guideId,expectedSha256:current.sha256,operationId:'missing-authority',decisionRef:'proposed-approval',approved:true}),/authority_unconfigured/);
 const request={intent:'review_exact_candidate',decision:'approved',taskId:'task',rootGuideId:current.rootGuideId,reviewedGuideId:saved.guideId,reviewedSha256:current.sha256,contentSha256:current.contentSha256};
 assert.equal((await fetch(origin+'/decisions',{method:'POST',headers:{authorization:'Bearer synthetic-receiver',origin:'https://operator.test'},body:JSON.stringify({...request,actorRef:'human'})})).status,403,'receiver/model label cannot perform operator decision');
 const operator=await fetch(origin+'/decisions',{method:'POST',headers:{authorization:'Bearer synthetic-human',origin:'https://operator.test'},body:JSON.stringify(request)});
 assert.equal(operator.status,200);const decision=await operator.json();
 const reviewed=await reopened.recordDecision({taskId:'task',guideId:saved.guideId,expectedSha256:current.sha256,operationId:'review-1',decisionRef:decision.decisionId});
 const recovered=await createTeachingReviewClient(options).recordDecision({taskId:'task',guideId:saved.guideId,expectedSha256:current.sha256,operationId:'review-1',decisionRef:decision.decisionId});
 assert.equal(recovered.guideId,reviewed.guideId);assert.equal(recovered.recoveredOriginal,true);
 const final=await reopened.open({taskId:'task',guideId:reviewed.guideId});
 assert.equal(final.review.state,'approved');assert.equal(final.ordinaryReuseAllowed,false);assert.equal(operatorPosts,1);
 assert.equal(final.blockers.includes('fresh_repeat_validation_required'),true);assert.equal(final.proposals[0].adopted,false);
 const repeatRoot=structuredClone(original);repeatRoot.guideId='daily-sales-repeat-v1';repeatRoot.teaching.unresolved=[];
 await writeTeachingDraftCandidate({candidateDirectory:directory,candidateOnly:true,guide:repeatRoot});
 let trustedRepeat;
 const repeatOptions={...options,readFreshRepeat:async request=>{if(request.validationRef!=='verified-repeat-1'||!trustedRepeat)throw Error('repeat receipt unavailable');return structuredClone(trustedRepeat);}};
 const repeatClient=createTeachingReviewClient(repeatOptions);
 const repeatView=await repeatClient.open({taskId:'review-task',guideId:repeatRoot.guideId});
 const repeatDecision=await (await fetch(origin+'/decisions',{method:'POST',headers:{authorization:'Bearer synthetic-human',origin:'https://operator.test'},body:JSON.stringify({intent:'review_exact_candidate',decision:'approved',taskId:'review-task',rootGuideId:repeatView.rootGuideId,reviewedGuideId:repeatView.guideId,reviewedSha256:repeatView.sha256,contentSha256:repeatView.contentSha256})})).json();
 const repeatReviewed=await repeatClient.recordDecision({taskId:'review-task',guideId:repeatView.guideId,expectedSha256:repeatView.sha256,operationId:'review-for-repeat',decisionRef:repeatDecision.decisionId});
 const reviewedRepeat=await repeatClient.open({taskId:'review-task',guideId:repeatReviewed.guideId});
 const actualCsv='report,date,net_usd,transactions\nDaily sales,2026-06-01,310.00,11\nDaily sales,2026-06-30,420.00,14\n';
 await writeFile(join(directory,'repeat.csv'),actualCsv,{mode:0o600});
 assert.equal(await readFile(join(directory,'repeat.csv'),'utf8'),actualCsv);
 trustedRepeat={schemaVersion:'agent-browser.teaching-fresh-repeat.v1',validationId:'verified-repeat-1',provenance:'receiving_application_verifier',taskId:'fresh-report-task',sourceTaskId:repeatRoot.teaching.sourceTaskId,rootGuideId:repeatRoot.guideId,executedGuideId:repeatReviewed.guideId,executedSha256:reviewedRepeat.sha256,contentSha256:reviewedRepeat.contentSha256,reviewDecisionId:repeatDecision.decisionId,playbookId:repeatRoot.teaching.playbook.id,requestedStartDate:'2026-06-01',requestedEndDate:'2026-06-30',reportId:'daily-sales',artifactId:'verified-csv-1',artifactSha256:hash(await readFile(join(directory,'repeat.csv'))),artifactMediaType:'text/csv',originalOperationId:'report-submit-1',verification:'report_identity_requested_dates_and_content'};
 const repeatInput={taskId:'fresh-report-task',guideId:repeatReviewed.guideId,expectedSha256:reviewedRepeat.sha256,operationId:'record-repeat-1',validationRef:'verified-repeat-1',requestedStartDate:'2026-06-01',requestedEndDate:'2026-06-30',reportId:'daily-sales'};
 const repeated=await repeatClient.recordFreshRepeat(repeatInput);
 assert.equal(repeated.status,'saved','verified fresh report evidence must be retained in the existing candidate lineage');
 const repeatedView=await createTeachingReviewClient(repeatOptions).open({taskId:'fresh-report-task',guideId:repeated.guideId});
 assert.equal(repeatedView.validation.state,'verified');assert.equal(repeatedView.ordinaryReuseAllowed,false);
 assert.equal(await readFile(join(directory,accepted.guideId+'.json'),'utf8'),acceptedBytes,'repeat evidence alone never replaces accepted bytes');
 const promoted=await repeatClient.promote({taskId:'fresh-report-task',guideId:repeated.guideId,expectedSha256:repeatedView.sha256,operationId:'promote-repeat-1'});
 assert.equal(promoted.status,'promoted');assert.equal(promoted.ordinaryReuseAllowed,true);
 const promotedView=await createTeachingReviewClient(repeatOptions).open({taskId:'fresh-report-task',guideId:promoted.guideId});
 assert.equal(promotedView.acceptedVersion.guideId,repeatReviewed.guideId);
 assert.equal(promotedView.acceptedVersion.sha256,reviewedRepeat.sha256);
 assert.equal(promotedView.ordinaryReuseAllowed,true);assert.deepEqual(promotedView.blockers,[]);
 assert.equal((await repeatClient.recordFreshRepeat(repeatInput)).guideId,repeated.guideId,'a repeated metadata operation returns its immutable original revision');
 assert.equal((await repeatClient.promote({taskId:'fresh-report-task',guideId:repeated.guideId,expectedSha256:repeatedView.sha256,operationId:'promote-repeat-1'})).guideId,promoted.guideId);
 const withoutRepeatAuthority=createTeachingReviewClient({...repeatOptions,readFreshRepeat:undefined});
 await assert.rejects(withoutRepeatAuthority.recordFreshRepeat({...repeatInput,operationId:'model-forged-repeat',passed:true}),/teaching_repeat_authority_unconfigured/);
 await assert.rejects(withoutRepeatAuthority.promote({taskId:'fresh-report-task',guideId:promoted.guideId,expectedSha256:promotedView.sha256,operationId:'model-forged-promotion',passed:true}),/teaching_repeat_authority_unconfigured/);
 const authorityMissing=await withoutRepeatAuthority.open({taskId:'fresh-report-task',guideId:promoted.guideId});
 assert.equal(authorityMissing.ordinaryReuseAllowed,false);assert.equal(authorityMissing.acceptedVersion.guideId,accepted.guideId);
 const ownerProof=structuredClone(trustedRepeat);
 for(const changedProof of [{...ownerProof,requestedStartDate:'2026-05-01'},{...ownerProof,reportId:'inventory-valuation'},{...ownerProof,artifactSha256:'a'.repeat(64)},{...ownerProof,executedSha256:'b'.repeat(64)}]) {
  trustedRepeat=changedProof;
  const refused=await repeatClient.open({taskId:'fresh-report-task',guideId:promoted.guideId});
  assert.equal(refused.validation.state,'unverified');assert.equal(refused.ordinaryReuseAllowed,false);
  assert.equal(refused.acceptedVersion.guideId,accepted.guideId,'changed trusted receipt cannot replace the prior accepted version');
 }
 trustedRepeat=ownerProof;
 const operatorUnavailable=await createTeachingReviewClient({...repeatOptions,readOperatorDecision:undefined}).open({taskId:'fresh-report-task',guideId:promoted.guideId});
 assert.equal(operatorUnavailable.ordinaryReuseAllowed,false);assert.equal(operatorUnavailable.acceptedVersion.guideId,accepted.guideId);
 await assert.rejects(repeatClient.recordFreshRepeat({...repeatInput,guideId:promoted.guideId,expectedSha256:promotedView.sha256,operationId:'record-training-period',requestedStartDate:'2026-05-01',requestedEndDate:'2026-05-31'}),/teaching_fresh_dates_required/);
 await assert.rejects(repeatClient.recordFreshRepeat({...repeatInput,guideId:promoted.guideId,expectedSha256:promotedView.sha256,operationId:'record-same-review-task',taskId:'review-task'}),/teaching_fresh_task_required/);
 await assert.rejects(repeatClient.promote({taskId:'x'.repeat(4001),guideId:promoted.guideId,expectedSha256:promotedView.sha256,operationId:'reject-oversized-task'}),/teaching_review_request_invalid/);
 await assert.rejects(repeatClient.promote({taskId:'fresh-report-task',guideId:reviewed.guideId,expectedSha256:final.sha256,operationId:'promote-unanswered-candidate'}),/teaching_open_questions_present/);
 assert.equal(await readFile(join(directory,accepted.guideId+'.json'),'utf8'),acceptedBytes);
 const clarifiedRoot=structuredClone(original);clarifiedRoot.guideId='daily-sales-clarification-repeat-v1';clarifiedRoot.teaching.unresolved=[clarifiedRoot.teaching.unresolved[0]];
 await writeTeachingDraftCandidate({candidateDirectory:directory,candidateOnly:true,guide:clarifiedRoot});
 let clarificationProof;
 const readOperatorAnswer=async request=>{const response=await fetch(origin+'/answers/'+encodeURIComponent(request.answerRef),{headers:{authorization:'Bearer synthetic-receiver'},redirect:'error'});if(!response.ok)throw Error('operator answer unavailable');return response.json();};
 const clarificationOptions={...options,readOperatorAnswer,readFreshRepeat:async()=>structuredClone(clarificationProof)};
 const clarifiedReview=createTeachingReviewClient(clarificationOptions),grilling=createTeachingGrillingClient({review:clarifiedReview,readOperatorAnswer});
 const clarificationInitial=await clarifiedReview.open({taskId:'task',guideId:clarifiedRoot.guideId});
 const preparedQuestion=await grilling.prepare({taskId:'task',guideId:clarificationInitial.guideId,expectedSha256:clarificationInitial.sha256,operationId:'prepare-refunds-question',questions:[{id:'refunds-policy',category:'variable_input',stepRefs:['set-report-period:2'],dependsOn:[]}]});
 const questionView=await grilling.open({taskId:'task',guideId:preparedQuestion.guideId}),question=questionView.round[0];
 const answer=await (await fetch(origin+'/answers',{method:'POST',headers:{authorization:'Bearer synthetic-human',origin:'https://operator.test'},body:JSON.stringify({questionId:question.id,questionSha256:question.questionSha256,answer:'Keep refunds as the explicit caller choice for each report.',taskId:'task',rootGuideId:questionView.rootGuideId,answeredGuideId:questionView.guideId,answeredSha256:questionView.sha256,contentSha256:questionView.contentSha256})})).json();
 const revised=structuredClone(questionView.guide);revised.teaching.parameters.find(parameter=>parameter.id==='includeRefunds').meaning='Use the explicit caller refunds choice confirmed by the operator, rather than adopting the example as a business default.';
 const answered=await grilling.answer({taskId:'task',guideId:questionView.guideId,expectedSha256:questionView.sha256,operationId:'apply-authenticated-answer',questionId:question.id,answerRef:answer.answerRef,revisedGuide:revised});
 const answeredView=await clarifiedReview.open({taskId:'review-task',guideId:answered.guideId});
 const clarificationDecision=await (await fetch(origin+'/decisions',{method:'POST',headers:{authorization:'Bearer synthetic-human',origin:'https://operator.test'},body:JSON.stringify({intent:'review_exact_candidate',decision:'approved',taskId:'review-task',rootGuideId:answeredView.rootGuideId,reviewedGuideId:answeredView.guideId,reviewedSha256:answeredView.sha256,contentSha256:answeredView.contentSha256})})).json();
 const clarificationReviewed=await clarifiedReview.recordDecision({taskId:'review-task',guideId:answeredView.guideId,expectedSha256:answeredView.sha256,operationId:'review-answered-procedure',decisionRef:clarificationDecision.decisionId});
 const clarified=await clarifiedReview.open({taskId:'review-task',guideId:clarificationReviewed.guideId});
 clarificationProof={...ownerProof,validationId:'verified-clarified-repeat',taskId:'fresh-clarified-task',rootGuideId:clarified.rootGuideId,executedGuideId:clarified.guideId,executedSha256:clarified.sha256,contentSha256:clarified.contentSha256,reviewDecisionId:clarificationDecision.decisionId,originalOperationId:'clarified-report-submit'};
 const clarifiedInput={taskId:'fresh-clarified-task',guideId:clarified.guideId,expectedSha256:clarified.sha256,operationId:'record-clarified-repeat',validationRef:clarificationProof.validationId,requestedStartDate:'2026-06-01',requestedEndDate:'2026-06-30',reportId:'daily-sales'};
 await assert.rejects(createTeachingReviewClient({...clarificationOptions,readOperatorAnswer:undefined}).recordFreshRepeat(clarifiedInput),/clarification_authority_unconfigured/);
 await assert.rejects(createTeachingReviewClient({...clarificationOptions,readOperatorAnswer:async request=>({...request})}).recordFreshRepeat(clarifiedInput),/clarification_answers_unverified/,'mirrored bindings do not authenticate an answer');
 const clarifiedRepeat=await clarifiedReview.recordFreshRepeat(clarifiedInput),clarifiedRepeatView=await clarifiedReview.open({taskId:'fresh-clarified-task',guideId:clarifiedRepeat.guideId});
 assert.equal((await clarifiedReview.promote({taskId:'fresh-clarified-task',guideId:clarifiedRepeat.guideId,expectedSha256:clarifiedRepeatView.sha256,operationId:'promote-clarified-repeat'})).status,'promoted');
 const reopenedWithoutAcceptedConfig=await createTeachingReviewClient({...options,acceptedVersion:undefined}).open({taskId:'task',guideId:reviewed.guideId});assert.equal(reopenedWithoutAcceptedConfig.acceptedVersion.available,true);
 sourceAvailable=false;
 const missing=await reopened.open({taskId:'task',guideId:reviewed.guideId});
 assert.equal(missing.source.state,'unavailable');assert.equal(missing.source.replayable,false);
 assert.equal(missing.acceptedVersion.available,true);
 sourceReason='source_expired';assert.equal((await reopened.open({taskId:'task',guideId:reviewed.guideId})).source.reason,'source_expired');
 const wrongSource=await createTeachingReviewClient({...options,sourceStatus:async()=>({state:'available',recordingId:'peer',videoSha256:'a'.repeat(64)})}).open({taskId:'task',guideId:reviewed.guideId});assert.equal(wrongSource.source.reason,'source_binding_changed');
 const peerTaskSource=await createTeachingReviewClient({...options,sourceStatus:async request=>({...request,state:'available',sourceTaskId:'peer-task'})}).open({taskId:'task',guideId:reviewed.guideId});assert.equal(peerTaskSource.source.reason,'source_binding_changed');
 const malformedGaps=await createTeachingReviewClient({...options,sourceStatus:async request=>({...request,state:'available',gaps:[{tMs:4,scope:'audio'}]})}).open({taskId:'task',guideId:reviewed.guideId});assert.equal(malformedGaps.source.reason,'source_gap_metadata_unverified');assert.equal(malformedGaps.source.replayable,false);
 assert.equal(await readFile(join(directory,accepted.guideId+'.json'),'utf8'),acceptedBytes);
 assert.deepEqual(await readFile(join(directory,'peer.txt')),peer);
 assert.deepEqual(JSON.parse(await readFile(join(directory,original.guideId+'.json'),'utf8')),original);
 // Stale approval is not reusable on a changed content/version binding.
 const staleApproval={...request,reviewedSha256:'a'.repeat(64)};
 const staleDecision=await (await fetch(origin+'/decisions',{method:'POST',headers:{authorization:'Bearer synthetic-human',origin:'https://operator.test'},body:JSON.stringify(staleApproval)})).json();
 await assert.rejects(reopened.recordDecision({taskId:'task',guideId:reviewed.guideId,expectedSha256:final.sha256,operationId:'stale-review',decisionRef:staleDecision.decisionId}),/operator_decision_invalid/);
 const noAuthority=await createTeachingReviewClient({...options,readOperatorDecision:undefined}).open({taskId:'task',guideId:reviewed.guideId});
 assert.equal(noAuthority.review.state,'unverified');assert.equal(noAuthority.ordinaryReuseAllowed,false);
 const a=structuredClone(final.guide),b=structuredClone(final.guide);
 a.teaching.routines[1].steps[0].description+=' Caller A.';b.teaching.routines[1].steps[0].description+=' Caller B.';
 const race=await Promise.all([reopened.save({taskId:'task',guideId:final.guideId,expectedSha256:final.sha256,operationId:'race-a',guide:a}),reopened.save({taskId:'task',guideId:final.guideId,expectedSha256:final.sha256,operationId:'race-b',guide:b})]);
 assert.equal(race.filter(item=>item.status==='saved').length,1);assert.equal(race.filter(item=>item.status==='conflict').length,1);
 const winner=race.find(item=>item.status==='saved');
 const changed=await reopened.open({taskId:'task',guideId:winner.guideId});assert.equal(changed.review.state,'not_reviewed');
 async function interrupt(mode,input,marker) {
  const inputFile=join(directory,'caller-'+mode+'.json');await writeFile(inputFile,JSON.stringify(input),{mode:0o600});
  const child=spawn(process.execPath,[fileURLToPath(new URL('./fixtures/teaching-review-save-child.mjs',import.meta.url)),directory,mode,inputFile],{stdio:['ignore','pipe','pipe']});
  let stderr='';child.stderr.on('data',chunk=>stderr+=chunk);
  try {await new Promise((resolve,reject)=>{
   const timer=setTimeout(()=>reject(Error('owned child checkpoint timeout '+stderr)),10000);
   child.stdout.on('data',chunk=>{if(String(chunk).includes(marker)){clearTimeout(timer);resolve();}});
   child.once('error',reject);child.once('exit',code=>{clearTimeout(timer);reject(Error('owned child exited '+code+' '+stderr));});
  });} finally {
   if(child.exitCode===null&&child.signalCode===null){const exited=new Promise(resolve=>child.once('exit',resolve));child.kill('SIGTERM');await exited;}
  }
  assert.ok(child.exitCode!==null||child.signalCode!==null,'independent child process settlement');
 }
 const interruptedGuide=structuredClone(changed.guide);interruptedGuide.teaching.routines[1].steps[0].description+=' Explicit interrupted caller.';
 const interruptedInput={taskId:'task',guideId:changed.guideId,expectedSha256:changed.sha256,operationId:'interrupted-before',guide:interruptedGuide};
 await interrupt('before',interruptedInput,'validation-wait');
 const absent=await reopened.lookupOperation({rootGuideId:changed.rootGuideId,operationId:'interrupted-before'});
 assert.equal(absent.status,'not_observed');assert.equal(absent.retryAuthority,'not_inferred');
 await interrupt('after',{...interruptedInput,operationId:'interrupted-after'},'committed-no-reply');
 const recoveredSave=await createTeachingReviewClient(options).lookupOperation({rootGuideId:changed.rootGuideId,operationId:'interrupted-after'});
 assert.equal(recoveredSave.status,'saved');assert.equal(recoveredSave.recoveredOriginal,true);
 const originalAgain=await reopened.save({...interruptedInput,operationId:'interrupted-after'});assert.equal(originalAgain.guideId,recoveredSave.guideId);
 assert.equal(await readFile(join(directory,accepted.guideId+'.json'),'utf8'),acceptedBytes);assert.deepEqual(await readFile(join(directory,'peer.txt')),peer);
 assert.deepEqual(JSON.parse(await readFile(join(directory,original.guideId+'.json'),'utf8')),original);
 // Independent evidence retention: video access cannot certify a deleted
 // derived frame (or Speech result) as replayable.
 await rm(join(evidenceDirectory,'dates.png'));
 const componentClient=createTeachingReviewClient({...options,sourceStatus:async request=>({...await options.sourceStatus(request),state:'available',reason:'scoped_read'})});
 const componentView=await componentClient.open({taskId:'task',guideId:reviewed.guideId});
 assert.equal(componentView.source.replayable,true);
 const missingReference=componentView.steps.flatMap(step=>step.evidence).find(reference=>reference.id==='dates');
 assert.equal(missingReference.replayable,false);
 assert.equal(missingReference.availabilityState,'unavailable');assert.equal(missingReference.unavailabilityReason,'source_deleted');
 // Rust serializes absent optional SiteGuide fields as null. A review of an
 // unchanged root must bind the same semantic content as its stored source.
 const nullable=structuredClone(original);nullable.guideId='daily-sales-nullable-root';
 await writeTeachingDraftCandidate({candidateDirectory:directory,candidateOnly:true,guide:nullable});
 const nullableView=await reopened.open({taskId:'task',guideId:nullable.guideId});
 const nullableRequest={intent:'review_exact_candidate',decision:'approved',taskId:'task',rootGuideId:nullableView.rootGuideId,reviewedGuideId:nullable.guideId,reviewedSha256:nullableView.sha256,contentSha256:nullableView.contentSha256,sourceUnavailableAcknowledged:true};
 const nullableDecision=await (await fetch(origin+'/decisions',{method:'POST',headers:{authorization:'Bearer synthetic-human',origin:'https://operator.test'},body:JSON.stringify(nullableRequest)})).json();
 const nullableReviewed=await reopened.recordDecision({taskId:'task',guideId:nullable.guideId,expectedSha256:nullableView.sha256,operationId:'nullable-review',decisionRef:nullableDecision.decisionId});
 assert.equal(nullableReviewed.status,'saved');
 console.log('Teaching review public client: edit/reopen/version lineage, explicit review and accepted/peer/source preservation pass');
} finally {await new Promise(resolve=>server.close(resolve));await rm(directory,{recursive:true,force:true});}
