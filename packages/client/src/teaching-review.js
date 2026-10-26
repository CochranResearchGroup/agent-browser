// @ts-check
import {readFile,lstat,open,link,unlink} from 'node:fs/promises';
import {join,isAbsolute} from 'node:path';
import {createHash,randomUUID} from 'node:crypto';

const sha=value=>createHash('sha256').update(value).digest('hex');
const clone=value=>structuredClone(value);
// Validated optional SiteGuide null fields and absent fields have the same
// meaning after Rust readback; raw file SHA remains the exact CAS identity.
const canonical=value=>Array.isArray(value)?value.map(canonical):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).filter(key=>value[key]!==null&&value[key]!==undefined).sort().map(key=>[key,canonical(value[key])])):value;
const digest=value=>sha(JSON.stringify(canonical(value)));
const id=value=>typeof value==='string'&&/^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$/.test(value);
const guideId=value=>typeof value==='string'&&/^[a-z0-9][a-z0-9-]{0,119}$/.test(value);
const hash=value=>typeof value==='string'&&/^[a-f0-9]{64}$/.test(value);
const task=value=>typeof value==='string'&&value.trim().length>0&&value.length<=4000&&!/\p{Cc}/u.test(value);
/** @param {string} code @returns {never} */
const error=code=>{throw Error(code);};
function content(guide) {
  const value=clone(guide);delete value.guideId;
  delete value.teaching.lineage;delete value.teaching.review;delete value.teaching.validation;
  return value;
}
function versionId(root,revision) {return `${root.length<=96?root:'teaching-'+sha(root).slice(0,24)}-r${revision}`;}

/**
 * Review/version the existing SiteGuide JSON candidate directory. The trusted
 * receiving application supplies the operator decision reader; model-supplied
 * approval booleans and merely opening this client establish no approval.
 * @param {import('./teaching-review.d.ts').TeachingReviewOptions} options
 * @returns {import('./teaching-review.d.ts').TeachingReviewClient}
 */
export function createTeachingReviewClient({candidateDirectory,browser,sourceStatus,readOperatorDecision,readFreshRepeat,readOperatorAnswer,acceptedVersion}) {
  const retainedConfiguration=acceptedVersion===undefined?undefined:clone(acceptedVersion);
  if(retainedConfiguration&&(!guideId(retainedConfiguration.guideId)||!hash(retainedConfiguration.sha256)))error('teaching_accepted_version_invalid');
  if(!isAbsolute(candidateDirectory??'') || !browser?.inspectTeachingDraft || !browser?.validateTeachingCandidate)error('teaching_review_configuration_required');
  async function directory() {
    const stat=await lstat(candidateDirectory);
    if(!stat.isDirectory()||stat.isSymbolicLink()||process.getuid&&stat.uid!==process.getuid()||process.platform!=='win32'&&(stat.mode&0o077))error('teaching_review_directory_invalid');
  }
  async function read(name) {
    if(!guideId(name))error('teaching_review_guide_id_invalid');
    await directory();
    const filename=join(candidateDirectory,name+'.json');
    const stat=await lstat(filename);
    if(!stat.isFile()||stat.isSymbolicLink()||stat.size>65536)error('teaching_review_file_invalid');
    const bytes=await readFile(filename);if(bytes.length>65536)error('teaching_review_file_invalid');
    const guide=JSON.parse(bytes.toString('utf8'));
    if(guide.guideId!==name)error('teaching_review_identity_changed');
    return {guide,sha256:sha(bytes)};
  }
  const candidate=record=>{if(record.guide.teaching?.schemaVersion!=='agent-browser.teaching-draft.v1'||record.guide.teaching.state!=='draft')error('teaching_draft_required');return record;};
  const lineage=record=>record.guide.teaching?.lineage??{rootGuideId:record.guide.guideId,revision:1};
  async function chain(root) {
    const records=[candidate(await read(root))];
    if(records[0].guide.teaching.lineage)error('teaching_review_root_invalid');
    for(let revision=2;revision<=256;revision++) {
      let next;
      try {next=await read(versionId(root,revision));} catch(err) {if(err.code==='ENOENT')return records;throw err;}
      const previous=records.at(-1),meta=next.guide.teaching?.lineage;
      if(meta?.rootGuideId!==root||meta.revision!==revision||meta.parentGuideId!==previous.guide.guideId||meta.parentSha256!==previous.sha256)error('teaching_review_lineage_conflict');
      records.push(next);
    }
    return records;
  }
  /** @template {string} S
   * @param {any} record @param {S} status
   * @returns {import('./teaching-review.d.ts').TeachingVersionReference & {status:S,ordinaryReuseAllowed:false}}
   */
  function receipt(record,status) {
    const meta=lineage(record);
    return {status,guideId:record.guide.guideId,sha256:record.sha256,rootGuideId:meta.rootGuideId,revision:meta.revision,operationId:meta.operationId,ordinaryReuseAllowed:false};
  }
  function prior(records,operationId,intentSha256) {
    const record=records.find(item=>item.guide.teaching?.lineage?.operationId===operationId);
    if(!record)return;
    if(record.guide.teaching.lineage.intentSha256!==intentSha256)return {status:'conflict',reason:'operation_intent_changed',...receipt(record,'conflict')};
    return {...receipt(record,'saved'),recoveredOriginal:true};
  }
  async function publish(guide) {
    await directory();
    const bytes=Buffer.from(JSON.stringify(guide,null,2)+'\n');
    if(bytes.length>65536)error('teaching_candidate_too_large');
    const temporary=join(candidateDirectory,`.teaching-review-${randomUUID()}.tmp`);
    let file;
    try {
      file=await open(temporary,'wx',0o600);await file.writeFile(bytes);await file.sync();await file.close();file=null;
      await link(temporary,join(candidateDirectory,guide.guideId+'.json'));
      if(process.platform!=='win32'){const parent=await open(candidateDirectory,'r');try{await parent.sync();}finally{await parent.close();}}
      return {guide,sha256:sha(bytes)};
    } finally {await file?.close();await unlink(temporary).catch(err=>{if(err.code!=='ENOENT')throw err;});bytes.fill(0);}
  }
  /** @returns {Promise<import('./teaching-review.js').TeachingReviewView['source']>} */
  async function source(guide) {
    const scope={recordingId:guide.teaching.recordingId,videoSha256:guide.teaching.videoSha256,sourceTaskId:guide.teaching.sourceTaskId};
    const request={...scope,evidence:clone(guide.teaching.evidence)};
    if(typeof sourceStatus!=='function')return {...scope,state:'unchecked',replayable:false,reason:'source_not_checked'};
    try {
      const observed=await sourceStatus(request);
      if(observed.recordingId!==request.recordingId||observed.videoSha256!==request.videoSha256||observed.sourceTaskId!==request.sourceTaskId)return {...scope,state:'unavailable',replayable:false,reason:'source_binding_changed'};
      if(observed.state==='unchecked')return {...scope,state:'unchecked',replayable:false,reason:'source_not_checked'};
      if(!['available','unavailable'].includes(observed.state))return {...scope,state:'unchecked',replayable:false,reason:'source_state_unverified'};
      const available=observed.state==='available';
      if(observed.gaps!==undefined&&(!Array.isArray(observed.gaps)||observed.gaps.length>256||observed.gaps.some(gap=>!Number.isFinite(gap.tMs)||gap.tMs<0||!id(gap.reason)||!id(gap.scope))))return {...scope,state:'unchecked',replayable:false,reason:'source_gap_metadata_unverified'};
      const gaps=(observed.gaps??[]).map(gap=>({tMs:gap.tMs,reason:gap.reason,scope:gap.scope,timeDomain:/** @type {const} */('recording_ms')}));
      const evidenceAvailability=guide.teaching.evidence.map(reference=>{
        const matches=Array.isArray(observed.evidence)?observed.evidence.filter(item=>item.id===reference.id):[];
        const row=matches.length===1?matches[0]:undefined;
        const bound=row&&row.artifact===reference.artifact&&row.sha256===reference.sha256;
        const state=!available?'unavailable':matches.length>1?'unchecked':!row?'unchecked':!bound?'unavailable':['available','unavailable','unchecked'].includes(row.state)?row.state:'unchecked';
        const reason=state==='available'?'scoped_read':row&&!bound?'source_binding_changed':!row||matches.length>1?'evidence_not_checked':['source_deleted','source_expired','source_revoked'].includes(row?.reason)?row.reason:'source_unavailable';
        return {id:reference.id,artifact:reference.artifact,sha256:reference.sha256,state,reason,replayable:state==='available'};
      });
      return {...scope,gaps,evidenceAvailability,state:available?'available':'unavailable',replayable:available,reason:available?'scoped_read': ['source_deleted','source_expired','source_revoked'].includes(observed.reason)?observed.reason:'source_unavailable'};
    } catch {return {...scope,state:'unavailable',replayable:false,reason:'source_unavailable'};}
  }
  function acceptedReference(record) {
    const retained=record.guide.teaching.lineage?.acceptedVersion;
    if(retainedConfiguration&&retained&&digest(retainedConfiguration)!==digest(retained))error('teaching_accepted_version_conflict');
    return retained??retainedConfiguration;
  }
  /** @returns {Promise<import('./teaching-review.d.ts').TeachingAcceptedVersion>} */
  async function accepted(record) {
    const configured=acceptedReference(record);
    /** @type {import('./teaching-review.d.ts').TeachingAcceptedVersion} */
    let retained={available:false,state:'none_configured'};
    if(configured) {
      try {const value=await read(configured.guideId);retained={...configured,available:value.sha256===configured.sha256,state:value.sha256===configured.sha256?'retained':'changed',readiness:'not_requalified'};}
      catch {retained={...configured,available:false,state:'unavailable',readiness:'not_requalified'};}
    }
    for(const published of (await chain(lineage(record).rootGuideId)).reverse()) {
      if(published.guide.teaching.lineage?.operationKind!=='promote')continue;
      const proof=await validation(published);
      if(proof.state==='verified')return {guideId:proof.executedGuideId,sha256:proof.executedSha256,available:true,state:'accepted',readiness:'qualified_fresh_repeat',validationId:proof.validationId,previousAcceptedVersion:retained};
    }
    return retained;
  }
  /** @returns {import('./teaching-draft.js').TeachingOperatorReview} */
  function checkedDecision(value,request) {
    if(!value||(request.decisionRef&&value.decisionId!==request.decisionRef)||!['approved','rejected'].includes(value.decision)||value.provenance!=='authenticated_operator'||
       !id(value.decisionId)||!id(value.actorRef)||!id(value.sourceRef)||
       ['taskId','rootGuideId','reviewedGuideId','reviewedSha256','contentSha256'].some(key=>value[key]!==request[key]))error('teaching_operator_decision_invalid');
    return {decisionId:value.decisionId,decision:value.decision,actorRef:value.actorRef,sourceRef:value.sourceRef,provenance:'authenticated_operator',
      taskId:value.taskId,rootGuideId:value.rootGuideId,reviewedGuideId:value.reviewedGuideId,reviewedSha256:value.reviewedSha256,contentSha256:value.contentSha256,
      sourceUnavailableAcknowledged:value.sourceUnavailableAcknowledged===true};
  }
  async function operatorDecision(request) {
    try {return await readOperatorDecision(request);} catch {error('teaching_operator_decision_unavailable');}
  }
  /** @returns {Promise<import('./teaching-review.d.ts').TeachingReviewView['review']>} */
  async function review(record) {
    const saved=record.guide.teaching.review;
    if(!saved)return {state:'not_reviewed'};
    if(saved.contentSha256!==digest(content(record.guide)))return {state:'unverified',reason:'reviewed_content_changed',decisionId:saved.decisionId};
    if(typeof readOperatorDecision!=='function')return {state:'unverified',reason:'operator_authority_unconfigured',decisionId:saved.decisionId};
    try {
      const request={decisionRef:saved.decisionId,taskId:saved.taskId,rootGuideId:saved.rootGuideId,reviewedGuideId:saved.reviewedGuideId,reviewedSha256:saved.reviewedSha256,contentSha256:saved.contentSha256};
      const value=checkedDecision(await operatorDecision(request),request);
      if(digest(value)!==digest(saved))error('teaching_operator_decision_changed');
      return {state:value.decision,...value};
    } catch {return {state:'unverified',reason:'operator_decision_unavailable',decisionId:saved.decisionId};}
  }
  const date=value=>typeof value==='string'&&/^\d{4}-\d{2}-\d{2}$/.test(value)&&Number.isFinite(Date.parse(value))&&new Date(value).toISOString().slice(0,10)===value;
  /** @returns {import('./teaching-draft.js').TeachingValidation} */
  function checkedRepeat(value,request) {
    const bindings=['taskId','sourceTaskId','rootGuideId','executedGuideId','executedSha256','contentSha256','reviewDecisionId','playbookId','requestedStartDate','requestedEndDate','reportId'];
    if(!value||value.schemaVersion!=='agent-browser.teaching-fresh-repeat.v1'||value.provenance!=='receiving_application_verifier'||value.verification!=='report_identity_requested_dates_and_content'||value.artifactMediaType!=='text/csv'||
       value.validationId!==request.validationRef||!id(value.validationId)||!id(value.artifactId)||!id(value.originalOperationId)||!hash(value.artifactSha256)||bindings.some(key=>value[key]!==request[key]))error('teaching_repeat_receipt_invalid');
    return {schemaVersion:'agent-browser.teaching-fresh-repeat.v1',validationId:value.validationId,provenance:'receiving_application_verifier',
      taskId:value.taskId,sourceTaskId:value.sourceTaskId,rootGuideId:value.rootGuideId,executedGuideId:value.executedGuideId,executedSha256:value.executedSha256,
      contentSha256:value.contentSha256,reviewDecisionId:value.reviewDecisionId,playbookId:value.playbookId,
      requestedStartDate:value.requestedStartDate,requestedEndDate:value.requestedEndDate,reportId:value.reportId,
      artifactId:value.artifactId,artifactSha256:value.artifactSha256,artifactMediaType:'text/csv',originalOperationId:value.originalOperationId,
      verification:'report_identity_requested_dates_and_content'};
  }
  async function repeatReceipt(request) {
    if(typeof readFreshRepeat!=='function')error('teaching_repeat_authority_unconfigured');
    let value;try {value=await readFreshRepeat(request);} catch {error('teaching_repeat_receipt_unavailable');}
    return checkedRepeat(value,request);
  }
  async function settled(record) {
    if(record.guide.teaching.unresolved.length)error('teaching_open_questions_present');
    if(record.guide.teaching.clarification?.questions?.length) {
      if(typeof readOperatorAnswer!=='function')error('teaching_clarification_authority_unconfigured');
      const {verifyTeachingClarificationAnswers}=await import('./teaching-grilling.js');
      const answers=await verifyTeachingClarificationAnswers({view:{guide:record.guide,rootGuideId:lineage(record).rootGuideId},readOperatorAnswer,readCandidate:async({guideId:name})=>{const value=await read(name);return {...value,rootGuideId:lineage(value).rootGuideId,contentSha256:digest(content(value.guide))};}});
      if(record.guide.teaching.clarification.questions.some(question=>answers.filter(answer=>answer.questionId===question.id&&answer.verification==='verified').length!==1))error('teaching_clarification_answers_unverified');
    }
  }
  function repeatRequest(record,taskId,validationRef,start,end,reportId) {
    const teaching=record.guide.teaching,saved=teaching.review;
    const exampleStart=teaching.parameters.find(parameter=>parameter.id==='startDate'&&parameter.kind==='date');
    const exampleEnd=teaching.parameters.find(parameter=>parameter.id==='endDate'&&parameter.kind==='date');
    if(!date(start)||!date(end)||start>end||!id(reportId)||!id(validationRef)||!exampleStart||!exampleEnd||start===exampleStart.example||end===exampleEnd.example)error('teaching_fresh_dates_required');
    if(!task(taskId)||taskId===teaching.sourceTaskId||taskId===saved?.taskId)error('teaching_fresh_task_required');
    return {validationRef,taskId,sourceTaskId:teaching.sourceTaskId,rootGuideId:lineage(record).rootGuideId,executedGuideId:record.guide.guideId,executedSha256:record.sha256,contentSha256:digest(content(record.guide)),reviewDecisionId:saved?.decisionId,playbookId:teaching.playbook.id,requestedStartDate:start,requestedEndDate:end,reportId};
  }
  /** @returns {Promise<import('./teaching-review.d.ts').TeachingValidationState>} */
  async function validation(record) {
    const saved=record.guide.teaching.validation;
    if(!saved)return {state:'not_validated'};
    try {
      await settled(record);
      const decision=await review(record);if(decision.state!=='approved')error('teaching_operator_review_required');
      const executed=await read(saved.executedGuideId);
      if(executed.sha256!==saved.executedSha256||digest(content(executed.guide))!==digest(content(record.guide)))error('teaching_repeat_content_changed');
      const request=repeatRequest(executed,saved.taskId,saved.validationId,saved.requestedStartDate,saved.requestedEndDate,saved.reportId);
      const value=await repeatReceipt(request);if(digest(value)!==digest(saved))error('teaching_repeat_receipt_changed');
      return {state:'verified',...value};
    } catch(err) {return {state:'unverified',reason:err.message,validationId:saved.validationId};}
  }
  /** @returns {Promise<import('./teaching-review.d.ts').TeachingSaveResult>} */
  async function mutate({taskId,guideId:name,expectedSha256,operationId},kind,intentData,transform) {
    if(!task(taskId)||!hash(expectedSha256)||!id(operationId))error('teaching_review_request_invalid');
    const base=candidate(await read(name)),meta=lineage(base),records=await chain(meta.rootGuideId);
    const intentSha256=digest({taskId,guideId:name,expectedSha256,operationId,kind,data:intentData});
    const recovered=prior(records,operationId,intentSha256);if(recovered)return recovered;
    const head=records.at(-1);
    if(base.sha256!==expectedSha256||head.guide.guideId!==name||head.sha256!==expectedSha256)return {status:'conflict',reason:'stale_revision',current:receipt(head,'saved'),ordinaryReuseAllowed:false};
    if(meta.revision>=256)error('teaching_review_lineage_capacity');
    const retained=acceptedReference(base);
    const guide=await transform(clone(base.guide),base);
    const revision=meta.revision+1;
    guide.guideId=versionId(meta.rootGuideId,revision);
    guide.teaching.lineage={rootGuideId:meta.rootGuideId,revision,parentGuideId:name,parentSha256:base.sha256,operationId,operationKind:kind,intentSha256,
      ...(retained?{acceptedVersion:clone(retained)}:{})};
    const validated=await browser.validateTeachingCandidate({taskId,guide});
    if(validated.valid!==true||validated.guideId!==guide.guideId||validated.taskId!==taskId||validated.executionAllowed!==false||validated.operatorReviewQualified!==false)error('teaching_candidate_validation_failed');
    try {return receipt(await publish(guide),'saved');}
    catch(err) {
      if(err.code!=='EEXIST')return {status:'unknown',reason:'publication_result_unconfirmed',rootGuideId:meta.rootGuideId,operationId,ordinaryReuseAllowed:false};
      const observed=await chain(meta.rootGuideId),result=prior(observed,operationId,intentSha256);
      return result??{status:'conflict',reason:'concurrent_revision',current:receipt(observed.at(-1),'saved'),ordinaryReuseAllowed:false};
    }
  }
  return {
    async open({taskId,guideId:name}) {
      const record=await read(name),guide=await browser.inspectTeachingDraft(taskId,name);
      if(digest(guide.teaching)!==digest(record.guide.teaching))error('teaching_review_readback_changed');
      const [availability,decision,retained,repeat]=await Promise.all([source(guide),review(record),accepted(record),validation(record)]);
      const evidence=new Map(guide.teaching.evidence.map(item=>[item.id,item]));
      const evidenceAvailability=new Map((availability.evidenceAvailability??[]).map(item=>[item.id,item]));
      const referenceView=id=>{const status=evidenceAvailability.get(id);return {...evidence.get(id),replayable:status?.replayable===true,availabilityState:status?.state??'unchecked',unavailabilityReason:status?.reason??availability.reason};};
      /** @type {import('./teaching-review.d.ts').TeachingReviewView['steps']} */
      const steps=guide.teaching.routines.flatMap(routine=>routine.steps.map((step,index)=>({stepRef:`${routine.id}:${index}`,routineId:routine.id,index,action:step.action,target:step.target,explanation:step.description,
        expectedResult:step.expectedResult?{scope:'step',items:[step.expectedResult]}:{scope:'routine',items:routine.outcomes.map(item=>item.verification)},
        evidence:step.evidenceRefs.map(referenceView),
        sourceGaps:availability.gaps??[],uncertainty:[...guide.teaching.claims.filter(item=>item.attribution==='unknown'&&item.evidenceRefs.some(id=>step.evidenceRefs.includes(id))).map(item=>item.statement),...guide.teaching.unresolved.filter(item=>item.evidenceRefs.some(id=>step.evidenceRefs.includes(id))).map(item=>item.question)]})));
      const reusable=record.guide.teaching.lineage?.operationKind==='promote'&&repeat.state==='verified'&&decision.state==='approved'&&retained.guideId===repeat.executedGuideId&&retained.sha256===repeat.executedSha256;
      return {...receipt(record,'opened'),guide,contentSha256:digest(content(guide)),steps,source:availability,review:decision,validation:repeat,acceptedVersion:retained,
        warnings:clone(guide.teaching.limitations),proposals:guide.teaching.unresolved.map(item=>({...item,adopted:false})),blockers:reusable?[]:[...(decision.state==='approved'?[]:['operator_review_required']),...(guide.teaching.unresolved.length?['open_questions_present']:[]),repeat.state==='verified'?'promotion_required':'fresh_repeat_validation_required'],ordinaryReuseAllowed:reusable};
    },
    async save(input) {
      const prepared=clone(input.guide);
      if(prepared?.teaching?.state!=='draft')error('teaching_draft_required');
      return mutate(input,'edit',content(prepared),(base)=>{
        for(const key of ['lineage','review','validation'])if(digest(prepared.teaching[key]??null)!==digest(base.teaching[key]??null))error('teaching_review_metadata_not_editable');
        if(['sourceTaskId','recordingId','videoSha256'].some(key=>prepared.teaching[key]!==base.teaching[key])||digest(prepared.teaching.evidence)!==digest(base.teaching.evidence))error('teaching_original_source_immutable');
        delete prepared.teaching.lineage;delete prepared.teaching.review;delete prepared.teaching.validation;
        return prepared;
      });
    },
    async recordDecision(supplied) {
      const input=clone(supplied);
      if(!id(input.decisionRef))error('teaching_operator_decision_reference_required');
      return mutate(input,'review',{decisionRef:input.decisionRef},async(base,record)=>{
        if(typeof readOperatorDecision!=='function')error('teaching_operator_authority_unconfigured');
        const request={decisionRef:input.decisionRef,taskId:input.taskId,rootGuideId:lineage(record).rootGuideId,reviewedGuideId:input.guideId,reviewedSha256:input.expectedSha256,contentSha256:digest(content(base))};
        const decision=checkedDecision(await operatorDecision(request),request);
        const availability=await source(base);
        if((!availability.replayable||(availability.evidenceAvailability??[]).some(item=>!item.replayable))&&!decision.sourceUnavailableAcknowledged)error('teaching_source_unavailable_not_acknowledged');
        base.teaching.review=decision;delete base.teaching.validation;return base;
      });
    },
    async recordFreshRepeat(supplied) {
      const input=clone(supplied);
      if(typeof readFreshRepeat!=='function')error('teaching_repeat_authority_unconfigured');
      return mutate(input,'repeat',{validationRef:input.validationRef,requestedStartDate:input.requestedStartDate,requestedEndDate:input.requestedEndDate,reportId:input.reportId},async(base,record)=>{
        await settled(record);
        if((await review(record)).state!=='approved')error('teaching_operator_review_required');
        const request=repeatRequest(record,input.taskId,input.validationRef,input.requestedStartDate,input.requestedEndDate,input.reportId);
        base.teaching.validation=await repeatReceipt(request);return base;
      });
    },
    async promote(supplied) {
      const input=clone(supplied);
      if(typeof readFreshRepeat!=='function')error('teaching_repeat_authority_unconfigured');
      const result=await mutate(input,'promote',{},async(base,record)=>{
        await settled(record);
        if((await review(record)).state!=='approved')error('teaching_operator_review_required');
        if((await validation(record)).state!=='verified')error('teaching_fresh_repeat_required');
        return base;
      });
      if(result.status!=='saved')return result;
      const proof=await validation(await read(result.guideId));
      if(proof.state!=='verified')error('teaching_fresh_repeat_required');
      return {...result,status:'promoted',ordinaryReuseAllowed:true,acceptedVersion:{guideId:proof.executedGuideId,sha256:proof.executedSha256}};
    },
    async lookupOperation({rootGuideId,operationId}) {
      if(!id(operationId))error('teaching_review_request_invalid');
      const records=await chain(rootGuideId),found=records.find(record=>record.guide.teaching?.lineage?.operationId===operationId);
      return found?{...receipt(found,'saved'),recoveredOriginal:true}:{status:'not_observed',operationId,retryAuthority:'not_inferred'};
    },
  };
}
