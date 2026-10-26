/** HTTP adapters for the existing Remote View and Speech owner routes. */
function endpoint(value) {
  const url=new URL(value);
  if(url.username || url.password || url.search || url.hash ||
      !(url.protocol==='https:' || url.protocol==='http:' && ['127.0.0.1','localhost','[::1]'].includes(url.hostname))) throw new Error('teaching_endpoint_invalid');
  return url.href.replace(/\/$/,'');
}
function transport({url,token,fetch:fetcher=globalThis.fetch,maximumResponseBytes=1048576,origin}) {
  const base=endpoint(url);
  if(typeof token!=='string' || !token || /[\r\n]/.test(token)) throw new Error('teaching_authority_required');
  return async(path,body)=>{
    let response;
    try {response=await fetcher(base+path,{method:body===undefined?'GET':'POST',redirect:'error',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json',...(origin?{Origin:origin}:{})},...(body===undefined?{}:{body:JSON.stringify(body)}),signal:AbortSignal.timeout(30000)});}
    catch {throw new Error('teaching_owner_unavailable');}
    if(!response.ok) {await response.body?.cancel();throw new Error(`teaching_owner_rejected_${response.status}`);}
    const chunks=[];let size=0;
    try {
      for await(const chunk of response.body ?? []) {
        size+=chunk.length;
        if(size>maximumResponseBytes) throw new Error('teaching_owner_response_limit');
        chunks.push(Buffer.from(chunk));
      }
      try{return JSON.parse(Buffer.concat(chunks).toString('utf8'));}
      catch{throw new Error('teaching_owner_response_invalid');}
    } finally {for(const chunk of chunks)chunk.fill(0);}
  };
}
/** A single-record read grant only. Never configure an installation-master token. */
export function createTeachingRecordingReader({url,accessToken,fetch}) {
  const call=transport({url,token:accessToken,fetch});
  return input=>call('/v1/recording-access',input);
}
/**
 * Existing authenticated Speech operator routes. conversionLimits is reviewed
 * deployment configuration, not a claim that discovery or inference is ready.
 * The owner independently enforces actual conversion/storage/backend limits.
 */
export function createTeachingSpeechAdapter({url,token,conversionLimits,origin,fetch}) {
  if(!conversionLimits || !Number.isSafeInteger(conversionLimits.maximumInputBytes) || conversionLimits.maximumInputBytes<1 ||
      !Number.isSafeInteger(conversionLimits.maximumDurationMs) || conversionLimits.maximumDurationMs<1 || conversionLimits.maximumDurationMs>1800000) throw new Error('teaching_conversion_limits_required');
  if(typeof origin!=='string' || new URL(origin).origin!==origin) throw new Error('teaching_speech_origin_required');
  const limits=structuredClone(conversionLimits);
  const call=transport({url,token,fetch,origin,maximumResponseBytes:Math.ceil(limits.maximumDurationMs*32/3)*4+1048576});
  const operation=(id,suffix='')=>{
    if(typeof id!=='string' || !/^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$/.test(id)) throw new Error('teaching_operation_invalid');
    return `/v1/recordings/${encodeURIComponent(id)}${suffix}`;
  };
  return {
    capabilities:async()=>structuredClone(limits),
    upload:input=>call('/dashboard/api/uploads',input),
    discardUpload:artifactId=>call('/dashboard/api/uploads/discard',{artifactId}),
    saveRecording:input=>call('/v1/saved',input),
    reconcileSaved:requestId=>{operation(requestId);return call(`/v1/saved/requests/${encodeURIComponent(requestId)}`);},
    savedRecording:async id=>{
      operation(id);
      const value=await call(`/v1/saved/${encodeURIComponent(id)}`);
      return value.metadata;
    },
    transcribe:input=>call('/v1/recordings',input),
    recordingStatus:id=>call(operation(id)),
    transcript:id=>call(operation(id,'/transcript')),
    deleteTranscript:id=>call(operation(id,'/delete'),{}),
  };
}
