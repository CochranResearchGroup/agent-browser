import {createHash} from 'node:crypto';

const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const clone = value => structuredClone(value);
const fail = code => {throw new Error(code);};
const integer = value => Number.isSafeInteger(value) && value >= 0;
const hash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const text = value => typeof value === 'string' && value.length>0 && value.length<=128 && !/[\u0000-\u001f\u007f]/.test(value);
// Opaque task IDs use the service identity bound of4,000characters.
const task = value => typeof value === 'string' && value.trim().length>0 && value.length<=4000 && !/\p{Cc}/u.test(value);
const id = value => typeof value === 'string' && /^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$/.test(value);
const MAX_EVENTS_BYTES = 8 * 1024 * 1024;

/**
 * Join existing recording and Speech owner APIs in a trusted backend. This is
 * neither a recorder nor an authority store. The ports must preserve their
 * owner's scoped authentication, quota, revocation and deletion checks.
 * Capsules contain references, never credentials or retained transcript text.
 */
export function createTeachingEvidenceClient({recordings, speech}) {
  if (typeof recordings !== 'function' || !speech) fail('teaching_owner_adapters_required');

  async function artifact(recordingId, name, maximum, expectedHash) {
    const chunks=[];
    let offset=0;
    try {
      for (;;) {
        const reply=await recordings({recordingId,artifact:name,offset,length:65536});
        const bytes=Buffer.from(reply.base64 ?? '', 'base64');
        if (reply.recordingId!==recordingId || reply.artifact!==name || reply.offset!==offset ||
            !integer(reply.size) || reply.size>maximum || reply.nextOffset!==offset+bytes.length ||
            reply.nextOffset>reply.size || bytes.length>65536 || bytes.toString('base64')!==reply.base64 ||
            digest(bytes)!==reply.sha256 || reply.eof!==(reply.nextOffset===reply.size) ||
            (!reply.eof && bytes.length===0)) fail('teaching_artifact_invalid');
        chunks.push(bytes);offset=reply.nextOffset;
        if(reply.eof) break;
      }
      const bytes=Buffer.concat(chunks);
      if(digest(bytes)!==expectedHash) {bytes.fill(0);fail('teaching_artifact_changed');}
      return bytes;
    } finally {for(const chunk of chunks)chunk.fill(0);}
  }

  async function inspect(selection) {
    if (!selection || !/^[a-f0-9]{32}$/.test(selection.recordingId) || !id(selection.narrationSourceId)) fail('teaching_selection_invalid');
    const expected=selection.expected;
    if(!expected || !['owner','desktopId','application','sessionId'].every(key=>text(expected[key])) || !task(expected.taskId) ||
        !['contextRevision','slot','generation'].every(key=>integer(expected[key]))) fail('teaching_binding_required');
    const reply=await recordings({recordingId:selection.recordingId});
    if(!Array.isArray(reply.recordings) || reply.recordings.length!==1) fail('teaching_recording_ambiguous');
    const record=reply.recordings[0], start=record.start;
    if(record.recordingId!==selection.recordingId || record.owner!==expected.owner || record.desktopId!==expected.desktopId ||
       record.contextRevision!==expected.contextRevision || start?.slot!==expected.slot || start.generation!==expected.generation ||
       ['application','taskId','sessionId'].some(key=>start.context?.[key]!==expected[key])) fail('teaching_binding_changed');
    if(record.pinned!==true && record.expiresAtUnixMs!==null && record.expiresAtUnixMs!==undefined && record.expiresAtUnixMs<=Date.now()) fail('teaching_source_expired');
    if(!['complete','interrupted'].includes(record.state) || !integer(record.durationMs) || record.durationMs<1 ||
       !hash(record.videoSha256) || !hash(record.eventsSha256)) fail('teaching_recording_not_finalized_av');
    const tracks=start.sourceTracks;
    if(!Array.isArray(tracks) || tracks.length>4 || tracks.filter(track=>track.sourceId===selection.narrationSourceId).length!==1) fail('teaching_narration_unavailable');
    const index=tracks.findIndex(track=>track.sourceId===selection.narrationSourceId);
    if(!hash(record.sourceSha256?.[index])) fail('teaching_narration_unavailable');
    const bytes=await artifact(selection.recordingId,'events.jsonl',MAX_EVENTS_BYTES,record.eventsSha256);
    let events;
    try {events=bytes.toString('utf8').trim().split('\n').filter(Boolean).map(line=>JSON.parse(line));}
    catch {fail('teaching_events_invalid');}
    finally {bytes.fill(0);}
    let previous=-1;
    for(const event of events) {
      if(typeof event.type!=='string' || !Number.isFinite(event.tMs) || event.tMs<previous || event.tMs>record.durationMs) fail('teaching_events_invalid');
      previous=event.tMs;
    }
    const sourceBindings=events.filter(event=>event.type==='mode' && event.scope==='audio' && event.reason==='source_binding')
      .map(event=>({tMs:event.tMs,detail:clone(event.detail)}));
    const gaps=events.filter(event=>event.type==='gap').map(event=>({tMs:event.tMs,scope:event.scope,reason:event.reason}));
    if(!sourceBindings.length) gaps.push({tMs:0,scope:'audio',reason:'source_binding_unavailable'});
    if(record.state==='interrupted') gaps.push({tMs:record.durationMs,scope:'recording',reason:'recording_interrupted'});
    return {schemaVersion:1,selection:clone(selection),taskId:expected.taskId,binding:clone(expected),
      recordingId:record.recordingId,state:record.state,durationMs:record.durationMs,
      retention:{pinned:record.pinned===true,expiresAtUnixMs:record.expiresAtUnixMs ?? null},
      video:{artifact:'video.webm',sha256:record.videoSha256},
      events:{artifact:'events.jsonl',sha256:record.eventsSha256},
      narration:{artifact:`source-${index}.webm`,sourceId:selection.narrationSourceId,sha256:record.sourceSha256[index]},
      sourceBindings,gaps,
      inputEvidence:events.filter(event=>event.type==='input').map(event=>({tMs:event.tMs,source:event.source,delivery:event.delivery,applicationAcknowledged:false})),
      alignment:{state:'unverified',recordingOriginMs:0,normalizedOriginMs:0,conversionOffsetMs:null,uncertaintyMs:null,reason:'conversion_and_encoder_padding_not_measured'},
      speakerIdentity:'unverified',criticalTokens:'requires_review'};
  }

  async function current(bundle) {
    const observed=await inspect(bundle.selection);
    if(observed.recordingId!==bundle.recordingId || observed.taskId!==bundle.taskId || observed.durationMs!==bundle.durationMs ||
        ['owner','desktopId','contextRevision','slot','generation','application','taskId','sessionId'].some(key=>observed.binding[key]!==bundle.binding?.[key]) ||
        observed.narration.sourceId!==bundle.narration?.sourceId ||
        ['video','events','narration'].some(key=>observed[key].sha256!==bundle[key]?.sha256 || observed[key].artifact!==bundle[key]?.artifact)) fail('teaching_source_changed');
    return observed;
  }

  /** Revalidate persisted Speech references against the owner, never the capsule alone. */
  async function savedSource(bundle) {
    const observed=await current(bundle),source=clone(bundle.speechSource);
    if(!source || !id(source.artifactId) || !hash(source.revision) || !integer(source.durationMs) || source.durationMs<1 ||
        source.originalSha256!==observed.narration.sha256) fail('teaching_saved_source_invalid');
    const metadata=await speech.savedRecording(source.artifactId);
    if(metadata?.id!==source.artifactId || metadata.kind!=='recording' || metadata.mediaType!=='audio/wav' ||
        metadata.sha256!==source.revision || metadata.provenance?.sourceRevision!==observed.narration.sha256 ||
        metadata.provenance.durationMs!==source.durationMs) fail('teaching_saved_source_invalid');
    return {observed,source:clone(source)};
  }

  return {
    inspect,
    /** Join explicit stop/resume records without inventing excluded-interval timing or authority. */
    async inspectSequence({selections,gaps}) {
      if(!Array.isArray(selections) || selections.length<1 || selections.length>16 || !Array.isArray(gaps) || gaps.length!==selections.length-1) fail('teaching_sequence_invalid');
      const results=[];
      for(const selection of selections) results.push(await inspect(selection));
      const first=results[0];
      if(new Set(results.map(record=>record.recordingId)).size!==results.length || results.some(record=>['owner','application','taskId'].some(key=>record.binding[key]!==first.binding[key]))) fail('teaching_sequence_binding_changed');
      const references=gaps.map((gap,index)=>{
        if(gap.fromRecordingId!==results[index].recordingId || gap.toRecordingId!==results[index+1].recordingId || !['protected_input','operator_pause','source_change'].includes(gap.reason) || !id(gap.evidenceReference)) fail('teaching_gap_invalid');
        return {fromRecordingId:gap.fromRecordingId,toRecordingId:gap.toRecordingId,reason:gap.reason,evidenceReference:gap.evidenceReference,provenance:'caller_supplied',durationMs:null};
      });
      return {schemaVersion:1,taskId:first.taskId,recordings:results,gaps:references,captureSuppressionVerified:false};
    },
    /** Explicit ownership permits a Speech-owned normalized copy; originals are untouched. */
    async importNarration(selection,{requestId,owned}) {
      if(!id(requestId) || requestId.length>96 || owned!==true) fail('teaching_owned_source_required');
      const bundle=await inspect(selection);
      const name=`Teaching ${digest(JSON.stringify(selection))}`;
      const prior=await speech.reconcileSaved(`${requestId}-save`);
      if(prior.status==='applied') {
        const metadata=await speech.savedRecording(prior.artifactId);
        if(metadata.name!==name || metadata.provenance?.sourceRevision!==bundle.narration.sha256 || !hash(metadata.sha256) || !integer(metadata.provenance?.durationMs)) fail('teaching_import_conflict');
        return {...bundle,speechSource:{artifactId:metadata.id,revision:metadata.sha256,durationMs:metadata.provenance.durationMs,originalSha256:bundle.narration.sha256}};
      }
      if(prior.status!=='not_found') fail('teaching_import_unresolved');
      const capabilities=await speech.capabilities();
      if(!integer(capabilities.maximumInputBytes) || !integer(capabilities.maximumDurationMs) ||
          bundle.durationMs>capabilities.maximumDurationMs) fail('teaching_conversion_limit');
      const bytes=await artifact(bundle.recordingId,bundle.narration.artifact,capabilities.maximumInputBytes,bundle.narration.sha256);
      let upload;
      try {
        await current(bundle);
        upload=await speech.upload({requestId:`${requestId}-upload`,owned:true,audioBase64:bytes.toString('base64')});
      } finally {bytes.fill(0);}
      if(!id(upload.artifactId) || upload.revision!==bundle.narration.sha256 || !integer(upload.durationMs)) fail('teaching_upload_invalid');
      try {
        const saved=await speech.saveRecording({requestId:`${requestId}-save`,name,kind:'recording',artifactId:upload.artifactId,revision:upload.revision});
        const metadata=await speech.savedRecording(saved.artifactId);
        if(metadata.id!==saved.artifactId || metadata.kind!=='recording' || metadata.mediaType!=='audio/wav' ||
            !hash(metadata.sha256) || metadata.provenance?.sourceRevision!==upload.revision ||
            metadata.provenance.durationMs!==upload.durationMs) fail('teaching_saved_source_invalid');
        await current(bundle);
        return {...bundle,speechSource:{artifactId:metadata.id,revision:metadata.sha256,durationMs:upload.durationMs,originalSha256:upload.revision}};
      } finally {await speech.discardUpload(upload.artifactId);}
    },
    /** Speech's public range ceiling is five minutes, independent of conversion limits. */
    segmentRanges(bundle) {
      const duration=bundle.speechSource?.durationMs;
      if(!integer(duration) || duration<1) fail('teaching_saved_source_required');
      const ranges=[];
      for(let startMs=0;startMs<duration;startMs+=300000) ranges.push({startMs,endMs:Math.min(startMs+300000,duration)});
      return ranges;
    },
    async transcribeSegment(bundle,{requestId,startMs,endMs}) {
      if(!id(requestId) || !integer(startMs) || !integer(endMs) || endMs<=startMs || endMs-startMs>300000 || endMs>bundle.speechSource?.durationMs) fail('teaching_range_invalid');
      const {source}=await savedSource(bundle);
      const request={requestId,artifactId:source.artifactId,revision:source.revision,startMs,endMs,language:'en-US'};
      const receipt=await speech.transcribe(request);
      if(!id(receipt.id)) fail('teaching_operation_invalid');
      return {operationId:receipt.id,request,recordingId:bundle.recordingId,narrationSha256:bundle.narration.sha256};
    },
    /** Reopen reads original operations; it never resubmits recognition. */
    async readTranscript(bundle,segment) {
      try {await savedSource(bundle);} catch {return {status:'source_unavailable',recordingId:bundle.recordingId,operationId:segment.operationId};}
      if(segment.recordingId!==bundle.recordingId || segment.narrationSha256!==bundle.narration.sha256 ||
          segment.request.artifactId!==bundle.speechSource?.artifactId || segment.request.revision!==bundle.speechSource?.revision) fail('teaching_segment_binding_changed');
      const receipt=await speech.recordingStatus(segment.operationId);
      if(receipt.id!==segment.operationId) fail('teaching_operation_invalid');
      if(receipt.state!=='completed') return {status:'transcript_unavailable',operationId:segment.operationId,state:receipt.state};
      let transcript;
      try {transcript=await speech.transcript(segment.operationId);} catch {return {status:'transcript_unavailable',operationId:segment.operationId};}
      if(transcript.operationId!==segment.operationId || ['artifactId','revision','startMs','endMs'].some(key=>transcript.source?.[key]!==segment.request[key])) fail('teaching_transcript_source_changed');
      const {observed}=await savedSource(bundle);
      return {status:'available',operationId:segment.operationId,transcript,alignment:observed.alignment,gaps:observed.gaps,criticalTokens:'requires_review'};
    },
    /** Owner deletion removes derived text only, without deleting the recording or Saved audio. */
    deleteTranscript: operationId=>speech.deleteTranscript(operationId),
    /** Bounded media bytes stay outside model-visible summaries and retain scoped authorization. */
    async readVideoSlice(bundle,{offset=0,length=65536}={}) {
      if(!integer(offset) || !integer(length) || length<1 || length>65536) fail('teaching_read_limit');
      await current(bundle);
      return recordings({recordingId:bundle.recordingId,artifact:'video.webm',offset,length});
    },
  };
}
