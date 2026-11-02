'use client';
import {useEffect,useState} from 'react';
import {Button} from '@/components/ui/button';
import {requestSiteGuides,confirmTeachingReceipt,teachingOperationId,type SiteGuideSummary,type TeachingContext,type TeachingView,type SiteGuidesTransport} from '@/lib/workspace-siteguides';
// Descriptive original-operation references only. These client records never
// establish receipt authority and never authorize replay after an unknown result.
const readPending=(key:string)=>{try{return JSON.parse(sessionStorage.getItem(key)??'null');}catch{return null;}};
const retainPending=(key:string,value:unknown)=>{try{if(value===null)sessionStorage.removeItem(key);else sessionStorage.setItem(key,JSON.stringify(value));}catch{/* UI still keeps the in-memory reference. */}};
export interface SiteGuidesLibraryProps {
 context:TeachingContext|null;
 transport?:SiteGuidesTransport;
 onProfile?:(profileId:string,sitePolicyId:string)=>void;
 onNativeView?:()=>void;
 /** Existing chat/agent integration consumes exact evidence and stages a useful
  * inert candidate through the public owner. No response means awaiting agent. */
 onInterpret?:(packet:Record<string,unknown>)=>Promise<{guideId:string}|void>;
}
export function SiteGuidesLibrary({context,transport=requestSiteGuides,onNativeView,onInterpret}:SiteGuidesLibraryProps) {
 const [guides,setGuides]=useState<SiteGuideSummary[]>([]),[view,setView]=useState<TeachingView|null>(null),[history,setHistory]=useState<SiteGuideSummary['history']>([]);
 const [query,setQuery]=useState(''),[message,setMessage]=useState(''),[busy,setBusy]=useState(false);
 const [pending,setPendingState]=useState<Record<string,unknown>|null>(null);
 const setPending=(value:Record<string,unknown>|null)=>{setPendingState(value);retainPending('siteguides-review-pending',value);};
 useEffect(()=>{setPendingState(readPending('siteguides-review-pending'));},[]);
 const [repeatRef,setRepeatRef]=useState(''),[start,setStart]=useState(''),[end,setEnd]=useState(''),[report,setReport]=useState('');
 const [ack,setAck]=useState(false);
 const load=async()=>{try{setGuides((await transport({operation:'list'})).guides);}catch(error){setMessage(String(error));}};
 useEffect(()=>{void load();},[transport]);
 const open=async(taskId:string,guideId:string)=>{const result=await transport({operation:'open',taskId,guideId});setView(result.view);setHistory(result.history);};
 async function run(work:()=>Promise<void>){setBusy(true);setMessage('');try{await work();}catch(error){setMessage(String(error));}finally{setBusy(false);}}
 async function mutate(operation:string,extra:Record<string,unknown>={}) {
  if(!view)return;
  const request={operation,taskId:view.guide.teaching.sourceTaskId,guideId:view.guideId,rootGuideId:view.rootGuideId,expectedSha256:view.sha256,operationId:teachingOperationId(),...extra};
  setPending(request);
  const result=await transport(request);
  if(result.status==='unknown'){setMessage('Reply unconfirmed. Read the original operation; do not submit it again.');return;}
  if(result.status==='conflict'){setMessage('Candidate changed. Reload and review the exact current content.');return;}
  setPending(null);await open(request.taskId,result.guideId??view.guideId);await load();
 }
 return <section aria-label="SiteGuides library" className="space-y-4">
  <div><h2>SiteGuides</h2><p>Review reusable site knowledge, exact evidence, focused questions and fresh-repeat progress.</p></div>
  <input aria-label="Search SiteGuides" value={query} onChange={event=>setQuery(event.target.value)} placeholder="Search purpose or site"/>
  <TeachSite context={context} transport={transport} onNativeView={onNativeView} onInterpret={onInterpret} onCandidate={async guideId=>{if(context)await run(async()=>{await open(context.taskId,guideId);await load();});}}/>
  {message&&<p role="status">{message}</p>}
  <ul>{guides.filter(guide=>`${guide.name} ${guide.purpose} ${guide.allowedOrigins.join(' ')}`.toLowerCase().includes(query.toLowerCase())).map(guide=><li key={guide.rootGuideId}>
   <Button variant="ghost" disabled={busy} onClick={()=>void run(()=>open(guide.sourceTaskId,guide.guideId))}>{guide.name}</Button>
   <p>{guide.purpose}</p><p>Review: {guide.review}; fresh repeat: {guide.repeat}; source: {guide.source.state}. {guide.ordinaryReuseAllowed?'Accepted for ordinary reuse.':'Candidate remains inert.'}</p>
  </li>)}</ul>
  {view&&<article aria-label="Exact teaching candidate">
   <h3>{view.guide.name}</h3><p>{view.guide.teaching.playbook.goal}</p>
   <p>Exact content: <code>{view.sha256}</code></p><p>Source: {view.source.state}. Review: {view.review.state}. Fresh repeat: {view.validation.state}.</p>
   <ol>{view.steps.map(step=><li key={step.stepRef}><p>{step.explanation}</p><p>Expected: {step.expectedResult.items.join('; ')}</p>{step.evidence.map(item=><p key={item.id}>{item.summary} ({item.replayable?'source available':'source unavailable or unchecked'})</p>)}{step.uncertainty.map((text,index)=><p key={index}>Unresolved: {text}</p>)}</li>)}</ol>
   {view.round?.map(question=><div key={question.id}><h4>{question.question}</h4><p>Agent proposal, not adopted: {question.recommendation}</p><p>Answers must be bound to this question and candidate by the authenticated teaching receipt owner. Ask the connected agent to revise the inert candidate after your answer.</p></div>)}
   <details><summary>Exact candidate and retained history</summary><pre>{JSON.stringify(view.guide,null,2)}</pre><ul>{history.map(item=><li key={item.guideId}><Button variant="ghost" onClick={()=>void run(()=>open(view.guide.teaching.sourceTaskId,item.guideId))}>Revision {item.revision}</Button><code>{item.sha256}</code></li>)}</ul></details>
   {!view.source.replayable&&<label><input type="checkbox" checked={ack} onChange={event=>setAck(event.target.checked)}/>I have reviewed the source limitation and explicitly acknowledge unavailable evidence.</label>}
   <Button disabled={busy||(!view.source.replayable&&!ack)} onClick={()=>void run(async()=>{
    const decisionRef=teachingOperationId();
    await confirmTeachingReceipt(decisionRef,{purpose:'teaching_review',taskId:view.guide.teaching.sourceTaskId,rootGuideId:view.rootGuideId,reviewedGuideId:view.guideId,reviewedSha256:view.sha256,contentSha256:view.contentSha256,decision:'approved',sourceUnavailableAcknowledged:ack});
    await mutate('record_decision',{decisionRef});
   })}>Approve this exact content</Button>
   <fieldset><legend>Distinct fresh repeat</legend><p>The receiving application must verify a new task, changed dates, report identity and CSV content. A successful click or agent statement is insufficient.</p>
    <input aria-label="Verifier receipt reference" value={repeatRef} onChange={event=>setRepeatRef(event.target.value)}/>
    <input aria-label="Fresh repeat start date" type="date" value={start} onChange={event=>setStart(event.target.value)}/><input aria-label="Fresh repeat end date" type="date" value={end} onChange={event=>setEnd(event.target.value)}/>
    <input aria-label="Verified report identity" value={report} onChange={event=>setReport(event.target.value)}/>
    <Button disabled={busy||!repeatRef||!start||!end||!report} onClick={()=>void run(()=>mutate('record_repeat',{validationRef:repeatRef,requestedStartDate:start,requestedEndDate:end,reportId:report}))}>Read verified fresh repeat</Button>
   </fieldset>
   <Button disabled={busy||view.validation.state!=='verified'||view.review.state!=='approved'} onClick={()=>void run(()=>mutate('promote'))}>Accept immutable replacement</Button>

  </article>}
   {pending&&<Button disabled={busy} onClick={()=>void run(async()=>{const result=await transport({operation:'lookup_operation',taskId:pending.taskId,rootGuideId:pending.rootGuideId,operationId:pending.operationId});setMessage(result.status==='saved'?'Original operation recovered.':'Original operation not observed. Retry authority is not inferred.');if(result.status==='saved'){setPending(null);await open(String(pending.taskId),result.guideId);}})}>Read original operation</Button>}
 </section>;
}
export function TeachSite({context,transport=requestSiteGuides,onNativeView,onInterpret,onCandidate}:SiteGuidesLibraryProps&{onCandidate?:(guideId:string)=>Promise<void>}) {
 const [sourceKey,setSourceKey]=useState(''),[source,setSource]=useState<any>(null),[message,setMessage]=useState(''),[busy,setBusy]=useState(false),[pending,setPendingState]=useState<Record<string,unknown>|null>(null);
 const pendingKey='siteguides-agent-pending:'+context?.taskId;
 const setPending=(value:Record<string,unknown>|null)=>{setPendingState(value);retainPending(pendingKey,value);};
 useEffect(()=>{setPendingState(readPending(pendingKey));},[pendingKey]);
 const [bundle,setBundleState]=useState<any>(null),[importRef,setImportRefState]=useState(''),[segments,setSegmentsState]=useState<any[]>([]),[transcripts,setTranscripts]=useState<any[]>([]);
 const [rangeRequests,setRangeRequestsState]=useState<Record<number,string>>({});
 const speechKey='siteguides-speech-originals:'+context?.taskId;
 const retainSpeech=(patch:Record<string,unknown>)=>retainPending(speechKey,{...readPending(speechKey),sourceKey,...patch});
 const setBundle=(value:any)=>{setBundleState(value);retainSpeech({bundle:value});};
 const setImportRef=(value:string)=>{setImportRefState(value);retainSpeech({importRef:value});};
 const setSegments=(update:(current:any[])=>any[])=>setSegmentsState(current=>{const next=update(current);retainSpeech({segments:next});return next;});
 const setRangeRequests=(update:(current:Record<number,string>)=>Record<number,string>)=>setRangeRequestsState(current=>{const next=update(current);retainSpeech({rangeRequests:next});return next;});
 useEffect(()=>{const original=readPending(speechKey);setSourceKey(original?.sourceKey??'');setBundleState(original?.bundle??null);setImportRefState(original?.importRef??'');setSegmentsState(original?.segments??[]);setRangeRequestsState(original?.rangeRequests??{});setTranscripts([]);setSource(null);},[speechKey]);
 const [sources,setSources]=useState<Array<{sourceKey:string;recordingId:string;narrationSourceId:string}>>([]);
 async function run(work:()=>Promise<void>){setBusy(true);setMessage('');try{await work();}catch(error){setMessage(String(error));}finally{setBusy(false);}}
 return <section aria-label="Teach this site"><h3>Teach this site</h3>
  {context?<p>Task {context.taskId}; profile {context.profileId}; site {context.sitePolicyId}; intended account {context.account||'unspecified'}; organization {context.organization||'unspecified'}. Intended identity does not establish authentication freshness.</p>:<p>Select a browser task to teach this site.</p>}
  <Button disabled={!context||!onNativeView} onClick={onNativeView}>Open native Remote View</Button><p>Use the existing native recording and narration controls. Select finalized evidence after recording finishes.</p>
  <Button disabled={!context||busy} onClick={()=>void run(async()=>{const result=await transport({operation:'list_sources',taskId:context!.taskId});setSources(result.sources);if(!result.sources.length)setMessage('No authorized finalized recording sources are configured for this exact task.');})}>Find authorized recording sources</Button>
  {!!sources.length&&<select aria-label="Choose recording and narration source" value={sourceKey} onChange={event=>{setSourceKey(event.target.value);setSource(null);}}><option value="">Choose source</option>{sources.map(item=><option key={item.sourceKey} value={item.sourceKey}>{item.recordingId}: {item.narrationSourceId}</option>)}</select>}
  <input aria-label="Finalized teaching source reference" value={sourceKey} onChange={event=>{setSourceKey(event.target.value);setSource(null);}} disabled={!context} placeholder="Authorized recording source reference"/>
  <Button disabled={!context||!sourceKey||busy} onClick={()=>void run(async()=>{const result=await transport({operation:'inspect_source',taskId:context!.taskId,sourceKey});setSource(result);setMessage('Finalized source checked. Speaker identity and timeline alignment remain unverified.');})}>Check finalized evidence</Button>
  {source&&<p>Recording {source.recordingId}; video <code>{source.video?.sha256}</code>; narration <code>{source.narration?.sha256}</code>. Gaps: {source.gaps?.length??0}.</p>}
  <Button disabled={!context||!source||busy||!!importRef} onClick={()=>void run(async()=>{
   const requestId=teachingOperationId();setImportRef(requestId);
   const result=await transport({operation:'import_narration',taskId:context!.taskId,sourceKey,requestId});setBundle(result);
   setMessage('Narration copied into the existing Speech Saved owner. Original recording is unchanged.');
  })}>Import selected narration into Speech</Button>
  {importRef&&!bundle&&<Button disabled={busy} onClick={()=>void run(async()=>{const result=await transport({operation:'reconcile_narration',taskId:context!.taskId,sourceKey,requestId:importRef});if(result.status==='applied'){setBundle(result.bundle);setMessage('Original Saved import recovered.');}else setMessage('Original import not observed. Do not upload or convert again.');})}>Read original narration import</Button>}
  {bundle&&Array.from({length:Math.ceil(bundle.speechSource.durationMs/300000)},(_,index)=>({startMs:index*300000,endMs:Math.min((index+1)*300000,bundle.speechSource.durationMs)})).map((range,index)=><div key={index}>
   <Button disabled={busy||!!rangeRequests[index]} onClick={()=>void run(async()=>{const requestId=teachingOperationId();setRangeRequests(current=>({...current,[index]:requestId}));const result=await transport({operation:'transcribe',taskId:context!.taskId,sourceKey,bundle,range:{...range,requestId}});setSegments(current=>[...current,result]);})}>Transcribe narration {range.startMs}–{range.endMs} ms</Button>
   {rangeRequests[index]&&!segments.some(segment=>segment.request.requestId===rangeRequests[index])&&<p>Original transcription reply unconfirmed. Request {rangeRequests[index]}; do not submit recognition again.</p>}
  </div>)}
  {segments.map(segment=><div key={segment.operationId}><p>Original Speech operation {segment.operationId}</p><Button disabled={busy} onClick={()=>void run(async()=>{const result=await transport({operation:'read_transcript',taskId:context!.taskId,sourceKey,bundle,segment});setTranscripts(current=>[...current.filter(item=>item.operationId!==segment.operationId),result]);})}>Read original transcript</Button></div>)}
  {transcripts.map(result=><p key={result.operationId}>{result.status==='available'?result.transcript.text:`Transcript ${result.status}; no new recognition requested.`}</p>)}
  <Button disabled={!context||!source||busy||!!pending||!onInterpret} onClick={()=>void run(async()=>{
   const operationId=teachingOperationId(),request={operation:'agent_packet',taskId:context!.taskId,sourceKey,rootGuideId:operationId,operationId};setPending(request);
   const packet=await transport(request);const result=await onInterpret!({...packet,context,bundle,segments,transcripts});
   if(result?.guideId){await onCandidate?.(result.guideId);setPending(null);}else setMessage('Awaiting ordinary agent interpretation. No draft, review or repeat has been claimed.');
  })}>Ask agent to interpret exact evidence</Button>
  {!onInterpret&&<p>Agent interpretation is unavailable until the existing agent connection is configured.</p>}
  {pending&&<Button disabled={busy} onClick={()=>void run(async()=>{const result=await transport({...pending,operation:'lookup_operation'});if(result.status==='saved'){await onCandidate?.(result.guideId);setPending(null);setMessage('Original agent publication recovered.');}else setMessage('Original agent publication not observed. No automatic resubmission.');})}>Read original agent publication</Button>}
  {message&&<p role="status">{message}</p>}
 </section>;
}
