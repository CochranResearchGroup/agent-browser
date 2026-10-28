import {createHash} from 'node:crypto';
const clone=value=>structuredClone(value);
const canonical=value=>Array.isArray(value)?value.map(canonical):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).sort().filter(key=>value[key]!==undefined&&value[key]!==null).map(key=>[key,canonical(value[key])])):value;
const digest=value=>createHash('sha256').update(JSON.stringify(canonical(value))).digest('hex');
const fail=message=>{throw Error(message);};
const categories=['ambiguity','contradiction','missing_intent','variable_input','outcome','exception','recovery'];
 const answerFields=['answerRef','questionId','questionSha256','answer','actorRef','sourceRef','provenance','taskId','rootGuideId','answeredGuideId','answeredSha256','contentSha256'];
 const checkedAnswer=(value,request)=>{
  if(!value||value.provenance!=='authenticated_operator'||['answerRef','questionId','questionSha256','taskId','rootGuideId','answeredGuideId','answeredSha256','contentSha256'].some(key=>value[key]!==request[key])||['actorRef','sourceRef','answerRef'].some(key=>typeof value[key]!=='string'||!/^[A-Za-z0-9_.:-]{1,128}$/.test(value[key]))||['answer'].some(key=>typeof value[key]!=='string'||!value[key].trim()||value[key].length>4000||/[\u0000-\u001f]/.test(value[key])))fail('teaching_operator_answer_invalid');
  return Object.fromEntries(answerFields.map(key=>[key,value[key]]));
 };
/** Recheck inert answer claims using original candidate bytes and receiver authority.
 * @param {import('./teaching-grilling.js').TeachingAnswerVerificationOptions} options
 * @returns {Promise<Array<import('./teaching-draft.js').TeachingClarificationAnswer & {verification:'verified'|'unverified'}>>}
 */
export async function verifyTeachingClarificationAnswers({view,readOperatorAnswer,readCandidate}){
  return Promise.all((view.guide.teaching.clarification?.answers??[]).map(async saved=>{
   try{
    if(typeof readOperatorAnswer!=='function')throw Error('unconfigured');
    const question=view.guide.teaching.clarification.questions.find(question=>question.id===saved.questionId);
    const original=await readCandidate({taskId:saved.taskId,guideId:saved.answeredGuideId});
    const originalQuestion=original.guide.teaching.clarification?.questions.find(question=>question.id===saved.questionId);
    if(!question||!originalQuestion||digest(originalQuestion)!==saved.questionSha256||digest(question)!==saved.questionSha256||saved.taskId!==view.guide.teaching.sourceTaskId||saved.rootGuideId!==view.rootGuideId||original.rootGuideId!==view.rootGuideId||original.sha256!==saved.answeredSha256||original.contentSha256!==saved.contentSha256)throw Error('answer_binding_changed');
    const {answerRef,questionId,questionSha256,taskId,rootGuideId,answeredGuideId,answeredSha256,contentSha256}=saved;
    const request={answerRef,questionId,questionSha256,taskId,rootGuideId,answeredGuideId,answeredSha256,contentSha256};
    const observed=checkedAnswer(await readOperatorAnswer(request),saved);
    if(digest(observed)!==digest(saved))throw Error('changed');
    return {...clone(saved),verification:'verified'};
   }catch{return {...clone(saved),verification:'unverified'};}
  }));
 }

/** Focus agent-prepared questions through existing immutable candidate review semantics.
 * The trusted receiving application reads existing authenticated operator answers;
 * recommendations and caller role labels never supply that authority. */
/** @param {import('./teaching-grilling.js').TeachingGrillingOptions} options */
export function createTeachingGrillingClient({review,readOperatorAnswer}){
 async function base(input){
  const view=await review.open(input);
  if(input.taskId!==view.guide.teaching?.sourceTaskId)fail('teaching_grilling_source_task_mismatch');
  if(view.guide.teaching?.state!=='draft'||!view.guide.teaching.routines?.some(routine=>routine.steps?.length)||!view.guide.teaching.playbook?.routineIds?.length)fail('teaching_useful_draft_required');
  return view;
 }
 const verifyAnswers=view=>verifyTeachingClarificationAnswers({view,readOperatorAnswer,readCandidate:input=>review.open(input)});
 function ready(view){
  const plan=view.guide.teaching.clarification;
  if(!plan)return [];
  const settled=new Set(plan.answers.map(answer=>answer.questionId));
  return plan.questions.filter(question=>!settled.has(question.id)&&question.dependsOn.every(id=>settled.has(id)));
 }
 return {
  async prepare(input){
   const view=await base(input),guide=clone(view.guide);
   if(guide.teaching.clarification)fail('teaching_questionnaire_exists');
   if(!Array.isArray(input.questions)||!input.questions.length||input.questions.length>64)fail('teaching_questions_invalid');
   const unknown=new Map(guide.teaching.unresolved.map(question=>[question.id,question]));
   const steps=new Map(view.steps.map(step=>[step.stepRef,step.evidence.map(item=>item.id)]));
   const ids=new Set();
   const questions=input.questions.map(item=>{
    const unresolved=unknown.get(item.id);
    if(!unresolved||ids.has(item.id)||!categories.includes(item.category)||!Array.isArray(item.stepRefs)||!item.stepRefs.length||item.stepRefs.some(ref=>!steps.has(ref)||!steps.get(ref).some(id=>unresolved.evidenceRefs.includes(id)))||!Array.isArray(item.dependsOn)||item.dependsOn.some(id=>!ids.has(id)))fail('teaching_question_source_invalid');
    ids.add(item.id);
    return {...clone(unresolved),category:item.category,stepRefs:clone(item.stepRefs),dependsOn:clone(item.dependsOn)};
   });
   guide.teaching.clarification={questions,answers:[]};
   return review.save({...input,guide});
  },
  async answer(input){
   const view=await base(input);
   if(view.sha256!==input.expectedSha256)return {status:'conflict',reason:'stale_revision',ordinaryReuseAllowed:false};
   if(typeof readOperatorAnswer!=='function')fail('teaching_operator_answer_authority_unconfigured');
   if((await verifyAnswers(view)).some(answer=>answer.verification!=='verified'))fail('teaching_prior_answers_unverified');
   const question=ready(view).find(question=>question.id===input.questionId);
   if(!question)fail('teaching_question_not_ready');
   const request={answerRef:input.answerRef,questionId:question.id,questionSha256:digest(question),taskId:input.taskId,rootGuideId:view.rootGuideId,answeredGuideId:view.guideId,answeredSha256:view.sha256,contentSha256:view.contentSha256};
   const answer=checkedAnswer(await readOperatorAnswer(clone(request)),request);
   const guide=clone(input.revisedGuide);
   if(!guide?.teaching||digest(guide.teaching.clarification)!==digest(view.guide.teaching.clarification)||digest(guide.teaching.unresolved)!==digest(view.guide.teaching.unresolved))fail('teaching_question_history_not_editable');
   // Preserve all unrelated questions and source observations; the agent revises
   // procedural interpretation using this actual answer, never a recommendation.
   guide.teaching.unresolved=guide.teaching.unresolved.filter(item=>item.id!==question.id);
   guide.teaching.clarification.answers.push(answer);
   return review.save({...input,guide});
  },
  lookupOperation(input){return review.lookupOperation(input);},
  async open(input){
   const view=await base(input);
   const answers=await verifyAnswers(view);
   const evidence=new Map(view.guide.teaching.evidence.map(item=>[item.id,item]));
   return {...view,round:(answers.some(answer=>answer.verification!=='verified')?[]:ready(view)).map(question=>({...clone(question),recommendation:question.interpretation,recommendationAdopted:false,questionSha256:digest(question),sources:question.evidenceRefs.map(id=>clone(evidence.get(id)))})),answers,answerAuthority:answers.some(answer=>answer.verification!=='verified')?'unverified':'verified'};
  },
 };
}
