// @ts-check
/** @typedef {import('./hosted-login-receiver.d.ts').ReceiverOptions} ReceiverOptions */
/** @typedef {import('./hosted-login-receiver.d.ts').Scope} Scope */
/** @typedef {import('./hosted-login-receiver.d.ts').Guard} Guard */
const refused=()=>Object.assign(Error('Hosted login requires the current exact receiving request'),{code:'auth_not_authorized'});
const targetKeys=['provider','taskId','profileId','browserId','tabId'];
const scopeKeys=['person','tenant','account','origin','source'];
const canonicalOrigin=value=>{try{const url=new URL(value);return ['http:','https:'].includes(url.protocol)&&url.origin===value&&!url.username&&!url.password;}catch{return false;}};
/**
 * Trusted Bots-host receiving adapter, never a caller-side credential API.
 * The existing hosted service owns approval, source retrieval, durable input
 * intent and capture privacy. This module owns shared website state sequencing.
 * Approved authenticated origins may differ from the credential origin only
 * after authentication. Returned origin names the original request scope;
 * independent principal evidence must match the actual current page origin.
 * Browser input callbacks must fence the supplied observation and use the
 * current native Remote View owner for activation; they must never log values.
 * @param {ReceiverOptions} options
 */
export function createHostedLoginReceiver({binding,scope,expectedUsername,browser,readPrincipal,readCaptureSafety,readReconciliationSafety,awaitMfa,authenticatedOrigins=[],maxTransitions=8}) {
 if(!binding||binding.provider!=='agent-browser'||targetKeys.some(key=>typeof binding[key]!=='string'||!binding[key])||!scope||scopeKeys.some(key=>typeof scope[key]!=='string'||!scope[key])||!canonicalOrigin(scope.origin)||!Array.isArray(authenticatedOrigins)||authenticatedOrigins.length>8||authenticatedOrigins.some(origin=>!canonicalOrigin(origin))||!expectedUsername||!Number.isInteger(maxTransitions)||maxTransitions<1||maxTransitions>16||[browser?.observe,browser?.fill,browser?.activate,browser?.waitForChange,readPrincipal,readCaptureSafety].some(fn=>typeof fn!=='function'))throw TypeError('hosted_receiver_configuration_required');
 binding=Object.freeze(structuredClone(binding));scope=structuredClone(scope);
 const authenticatedOriginSet=new Set([scope.origin,...authenticatedOrigins]);
 let attempted=false,submitted=false,verified=false;
 /** @param {Scope} requested */
 const sameScope=requested=>scopeKeys.every(key=>requested?.[key]===scope[key])&&requested.capabilities?.length===1&&requested.capabilities[0]==='login';
 /** @param {Scope} requested @param {Guard} guard @param {boolean} [independent] */
 const observe=async(requested,guard,independent=false)=>{
  if(!sameScope(requested))throw refused();await guard();
  const current=await browser.observe(binding);await guard();
  if(targetKeys.some(key=>current.binding?.[key]!==binding[key])||(current.state==='authenticated'?!authenticatedOriginSet.has(current.origin):current.origin!==scope.origin))throw refused();
  // Readback of an exact owned login/unknown page is negative evidence, not
  // permission for input. Private application retains its strict state checks.
  if(independent)return current;
  if(!['identifier_required','password_required','mfa_required','authenticated'].includes(current.state))throw refused();
  if(current.state!=='identifier_required'&&current.accountMatched!==true)throw refused();
  if(current.state==='password_required'&&(current.visiblePassword!==true||current.maskedPassword!==true))throw refused();
  if(current.state==='mfa_required'&&typeof awaitMfa!=='function')throw refused();
  return current;
 };
 return {
  /** @param {Scope} requested @param {Guard} [guard] */
  async inspect(requested,guard=async()=>{}){if(attempted)throw refused();const current=await observe(requested,guard);if(current.state==='authenticated')throw refused();return {origin:scope.origin,account:scope.account};},
  /** @param {{username:string,password:string}} credential @param {Scope} requested @param {Guard} guard */
  async apply(credential,requested,guard){
   if(attempted||credential?.username!==expectedUsername||typeof credential.password!=='string'||!credential.password)throw refused();
   let current=await observe(requested,guard);
   if(current.state==='authenticated')throw refused();
   // The surrounding hosted service has already durably recorded input intent.
   // A lost acknowledgement can never authorize replay of this receiver.
   attempted=true;
   for(let step=0;step<maxTransitions;step++){
    if(current.state==='authenticated'){submitted=true;return;}
    if(current.state==='mfa_required'){
     if(typeof awaitMfa!=='function')throw refused();
     await guard();const result=await awaitMfa({receivingTarget:binding,scope:requested,guard});await guard();
     if(result?.dispatched!==true)throw refused();
     await browser.waitForChange({binding,observation:current,guard});
     const next=await observe(requested,guard);if(next.state===current.state)throw refused();current=next;continue;
    }
    const identifier=current.state==='identifier_required',control=identifier?'identifier':'password';
    await guard();await browser.fill({binding,observation:current,control,value:identifier?credential.username:credential.password,guard});
    current=await observe(requested,guard);
    if(current.state!==(identifier?'identifier_required':'password_required'))throw refused();
    await guard();await browser.activate({binding,observation:current,control:identifier?'identifierSubmit':'submit',guard});
    await browser.waitForChange({binding,observation:current,guard});
    const next=await observe(requested,guard);if(next.state===current.state)throw refused();current=next;
   }
   if(current.state==='authenticated'){submitted=true;return;}
   throw refused();
  },
  /** @param {Scope} requested @param {Guard} [guard] */
  async verify(requested,guard=async()=>{}){
   verified=false;if(!submitted)throw refused();const current=await observe(requested,guard);if(current.state!=='authenticated')throw refused();
   const proof=await readPrincipal({binding,guard}),after=await observe(requested,guard);
   verified=proof?.authenticated===true&&proof.origin===current.origin&&proof.username===expectedUsername&&after.state==='authenticated'&&after.origin===current.origin;
   return {origin:scope.origin,account:verified?scope.account:null,authenticated:verified};
  },
  /** Independent owner readback after human completion or lost acknowledgement.
   * No prior credential, private input, or remembered local success is reused.
   * @param {Scope} requested @param {Guard} [guard]
   */
  async reconcile(requested,guard=async()=>{}){
   const current=await observe(requested,guard,true),negative={origin:scope.origin,account:null,authenticated:false,captureSafe:false};
   if(current.state!=='authenticated'){
    const safety=typeof readReconciliationSafety==='function'?await readReconciliationSafety({binding,guard,authenticated:false}):{safe:false};
    const after=await observe(requested,guard,true);
    if(after.state!==current.state||after.origin!==current.origin||safety?.safe!==true||safety.protectedInputNotAttempted!==true||safety.receivingLoginVerified!==true)return negative;
    return {...negative,account:scope.account,captureSafe:true,protectedInputNotAttempted:true,receivingLoginVerified:true};
   }
   if(current.visiblePassword)return negative;
   const proof=await readPrincipal({binding,guard}),after=await observe(requested,guard,true);
   if(proof?.authenticated!==true||proof.username!==expectedUsername||proof.origin!==current.origin||after.state!=='authenticated'||after.origin!==current.origin||after.visiblePassword)return negative;
   const safety=typeof readReconciliationSafety==='function'?await readReconciliationSafety({binding,guard,authenticated:true}):{safe:false};
   const final=await observe(requested,guard,true);
   if(final.state!=='authenticated'||final.origin!==current.origin||final.visiblePassword)return negative;
   return {origin:scope.origin,account:scope.account,authenticated:true,captureSafe:safety?.safe===true};
  },
  /** @param {{username:string,password:string}} credential @param {Scope} requested */
  async safeForCapture(credential,requested){
   if(!verified||credential?.username!==expectedUsername)return false;
   const current=await observe(requested,async()=>{});if(current.state!=='authenticated'||current.visiblePassword)return false;
   const proof=await readCaptureSafety({binding,credential}),after=await observe(requested,async()=>{});return proof?.safe===true&&after.state==='authenticated'&&after.origin===current.origin;
  },
 };
}
