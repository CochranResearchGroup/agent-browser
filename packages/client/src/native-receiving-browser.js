// @ts-check
/** @typedef {import('./native-receiving-browser.js').NativeOptions} NativeOptions */
/** @typedef {import('./native-receiving-browser.js').NativeAction} NativeAction */
import {createHash} from 'node:crypto';
import {setTimeout as delay} from 'node:timers/promises';
const refused=()=>Object.assign(Error('Native receiving controls or owner changed'),{code:'auth_receiving_changed'});
const unknown=()=>Object.assign(Error('Native private input requires reconciliation'),{code:'auth_input_unknown'});
const keys=['provider','taskId','profileId','browserId','tabId'];
const desktopKeys=['assignmentId','desktopId','generation','viewingGeneration'];
const digest=value=>createHash('sha256').update(JSON.stringify(value)).digest('hex');

/**
 * Trusted-host port for the shared login receiver. readObservation returns only
 * semantic state and control geometry, never field values. inputMatches performs
 * private equality readback and returns a boolean. Remote View alone delivers
 * native input and enforces the exact process, window and visual guards.
 * No browser, desktop, viewer, credential store or caller-side transport is made.
 * @param {NativeOptions} options
 */
export function createNativeReceivingBrowser({readObservation,inputMatches,remote,desktop,guards,timeoutMs=30000}){
 if(typeof readObservation!=='function'||typeof inputMatches!=='function'||!remote||!desktop||!guards||!Number.isInteger(timeoutMs)||timeoutMs<1||timeoutMs>45000)throw TypeError('native_receiving_configuration_required');
 desktop=structuredClone(desktop);guards=structuredClone(guards);
 const dispatched=new Set();
 const owner=async()=>{
  const current=await remote.observe(desktop);
  if(desktopKeys.some(key=>current[key]!==desktop[key]))throw refused();
  const view=await remote.observation(desktop),process=guards.expectedProcessIdentity;
  if(!view.windows?.windows?.some(window=>window.active&&window.id===guards.windowId&&window.processIdentity?.pid===process.pid&&window.processIdentity?.start_token===process.start_token))throw refused();
 };
 const observe=async binding=>{
  const raw=await readObservation(binding),v=raw.viewport;
  if(keys.some(key=>raw.binding?.[key]!==binding[key])||!v||v.devicePixelRatio!==1||v.width!==guards.expectedWindow.width||v.height<=0||v.height>guards.expectedWindow.height||v.focused!==true||v.visible!==true)throw refused();
  await owner();
  const current={binding:raw.binding,origin:raw.origin,state:raw.state,accountMatched:raw.accountMatched,visiblePassword:raw.visiblePassword,maskedPassword:raw.maskedPassword,controls:raw.controls,viewport:v};
  return {...current,fingerprint:digest(current)};
 };
 const dispatch=async({binding,observation,control,guard,phase='activate',value=''})=>{
  await guard();const current=await observe(binding);
  if(current.fingerprint!==observation.fingerprint)throw refused();
  const c=current.controls?.[control],dismissing=phase==='dismissPopup',typing=phase!=='activate'&&!dismissing;
  if(!c||c.count!==1||c.visible!==true||c.enabled!==true||(dismissing?c.hitTarget!==false||c.dismissiblePopup!==true:c.hitTarget!==true)||!c.rect||typing&&(c.tag!=='INPUT'||control==='password'&&c.type!=='password'||control!=='password'&&!['text','email','tel','number'].includes(c.type)))throw refused();
  const frame=guards.expectedWindow,rect=c.rect;
  if(![rect.x,rect.y,rect.width,rect.height].every(Number.isFinite)||rect.width<1||rect.height<1)throw refused();
  const region={x:Math.floor(frame.x+rect.x),y:Math.floor(frame.y+frame.height-current.viewport.height+rect.y),width:Math.ceil(rect.width),height:Math.ceil(rect.height)};
  if(region.x<frame.x||region.y<frame.y||region.x+region.width>frame.x+frame.width||region.y+region.height>frame.y+frame.height)throw refused();
  const actionKey=current.fingerprint+'\0'+control+'\0'+phase;
  if(dispatched.has(actionKey))throw unknown();
  // Input borders are stable while caret and selection pixels blink. DOM hit
  // testing and the full control fingerprint still fence the actual field.
  const visualRegion=typing?{...region,height:Math.min(3,region.height)}:region;
  const capture=await remote.captureRegion(desktop,{...guards,region:visualRegion});
  await guard();const after=await observe(binding);if(after.fingerprint!==current.fingerprint)throw refused();
  /** @type {NativeAction[]} */
  const actions=dismissing?[{kind:'key',chord:'Escape'}]:phase==='type'?[{kind:'text',method:'keys',text:value}]:[{kind:'click',x:Math.floor(region.x+region.width/2),y:Math.floor(region.y+region.height/2),button:1,count:1}];
  if(phase==='clear')actions.push({kind:'key',chord:'ctrl+a'},{kind:'key',chord:'BackSpace'});
  // Mark before dispatch. A timeout or absent acknowledgement is not a retry.
  dispatched.add(actionKey);
  const result=await remote.input(desktop,{...guards,expectedVisual:{region:capture.region,sha256:capture.capture.sha256},actions});
  await guard();await owner();if(result?.delivery!=='dispatched')throw unknown();
  if(typing&&!await inputMatches({binding,control,selector:c.selector,value:phase==='clear'?'':value,guard}))throw unknown();
 };
 return {
  observe,
  async fill(input){
   if(!['identifier','password','mfa'].includes(input.control)||typeof input.value!=='string')throw refused();
   if(input.control==='identifier'){
    await input.guard();const current=await observe(input.binding),c=current.controls.identifier;
    const prefix=current.fingerprint+'\0identifier\0';if(dispatched.has(prefix+'clear')||dispatched.has(prefix+'type'))throw unknown();
    if(current.fingerprint!==input.observation.fingerprint||!c||c.count!==1||c.tag!=='INPUT'||!['text','email','tel','number'].includes(c.type??'')||!c.visible||!c.enabled||!c.hitTarget)throw refused();
    // Preserve a current exact identifier. Controlled pages can refill it after
    // clearing; no clear, key delivery or private-field shortcut is necessary.
    if(await inputMatches({binding:input.binding,control:input.control,selector:c.selector,value:input.value,guard:input.guard})){
     await input.guard();const after=await observe(input.binding);if(after.fingerprint!==current.fingerprint)throw refused();return;
    }
   }
   await dispatch({...input,phase:'clear'});
   // Native text delivery can change keyboard mappings. Acknowledged empty
   // readback must precede typing, rather than relying on retained selection.
   const current=await observe(input.binding),before=input.observation;
   if(current.state!==before.state||current.origin!==before.origin||digest(current.controls[input.control])!==digest(before.controls[input.control])||digest(current.viewport)!==digest(before.viewport)||input.control==='password'&&current.accountMatched!==true)throw refused();
   await dispatch({...input,observation:current,phase:'type'});
  },
  async activate(input){
   if(!['identifierSubmit','submit','mfaSubmit'].includes(input.control))throw refused();
   const before=input.observation,c=before.controls[input.control];
   if(c?.hitTarget===false&&c.dismissiblePopup===true){
    // The trusted observer identified a dismissible popup, rather than an
    // arbitrary obstruction. Escape never clicks through it or submits a form.
    await dispatch({...input,phase:'dismissPopup'});
    const current=await observe(input.binding);
    if(current.state!==before.state||current.origin!==before.origin||current.accountMatched!==before.accountMatched)throw refused();
    await dispatch({...input,observation:current});
   }else await dispatch(input);
  },
  async waitForChange({binding,observation,guard}){
   const end=Date.now()+timeoutMs;
   do{await guard();const current=await observe(binding);if(['identifier_required','password_required','mfa_required','authenticated'].includes(current.state)&&current.state!==observation.state)return;await delay(100);}while(Date.now()<end);
   throw unknown();
  },
 };
}
