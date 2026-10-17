import assert from 'node:assert/strict';
import {createNativeReceivingBrowser} from '../packages/client/src/native-receiving-browser.js';
const binding={provider:'agent-browser',taskId:'task',profileId:'profile',browserId:'browser',tabId:'tab'};
const desktop={assignmentId:'assignment',desktopId:'desktop',generation:1,viewingGeneration:2};
const guards={windowId:42,expectedProcessIdentity:{pid:123,start_token:'456'},expectedWidth:1440,expectedHeight:900,expectedWindow:{x:0,y:0,width:1440,height:900}};
let changed=false,field='old-user',dispatches=0,lastInput,firstInput;
const browser=createNativeReceivingBrowser({
 async readObservation(){return {binding:{...binding},origin:'https://accounts.example.test',state:'identifier_required',accountMatched:false,visiblePassword:false,maskedPassword:true,viewport:{width:1440,height:813,devicePixelRatio:1,focused:true,visible:true},controls:{identifier:{selector:'#identifier',count:1,tag:'INPUT',type:'text',visible:true,enabled:true,hitTarget:true,rect:{x:changed?110:100,y:100,width:200,height:30}}}};},
 async inputMatches({value}){return field===value;},desktop,guards,
 remote:{async observe(){return {...desktop};},async observation(){return {windows:{windows:[{id:42,active:true,processIdentity:{pid:123,start_token:'456'}}]}};},async captureRegion(_desktop,request){return {region:request.region,capture:{sha256:'visual-proof'}};},async input(_desktop,request){dispatches++;lastInput=request;firstInput??=request;if(request.actions.some(x=>x.kind==='key'&&x.chord==='BackSpace'))field='';const typed=request.actions.find(x=>x.kind==='text');if(typed)field+=typed.text;return {delivery:'dispatched'};}},
});
const before=await browser.observe(binding);
changed=true;await assert.rejects(browser.fill({binding,observation:before,control:'identifier',value:'another-user',guard:async()=>{}}));assert.equal(dispatches,0);changed=false;
await browser.fill({binding,observation:before,control:'identifier',value:'owned-user',guard:async()=>{}});
assert.equal(dispatches,2);assert.equal(field,'owned-user');assert.deepEqual(firstInput.actions[0],{kind:'click',x:200,y:202,button:1,count:1});assert.equal(lastInput.actions[0].method,'keys');assert(firstInput.actions.some(x=>x.kind==='key'&&x.chord==='BackSpace'));assert.equal(lastInput.expectedVisual.sha256,'visual-proof');
changed=true;
await assert.rejects(browser.fill({binding,observation:before,control:'identifier',value:'another-user',guard:async()=>{}}));assert.equal(dispatches,2);
console.log('Native receiving browser: exact owner, viewport conversion, visual-fenced native text and stale-control no dispatch passed');

let inputCount=0;
const privatePort=createNativeReceivingBrowser({desktop,guards,
 async readObservation(){return {binding:{...binding},origin:'https://accounts.example.test',state:'password_required',accountMatched:true,visiblePassword:true,maskedPassword:true,viewport:{width:1440,height:813,devicePixelRatio:1,focused:true,visible:true},controls:{password:{selector:'#password',count:1,tag:'INPUT',type:'password',visible:true,enabled:true,hitTarget:true,rect:{x:100,y:100,width:200,height:30}}}};},
 async inputMatches(){throw Error('Lost acknowledgement must not attempt value readback');},
 remote:{async observe(){return {...desktop};},async observation(){return {windows:{windows:[{id:42,active:true,processIdentity:{pid:123,start_token:'456'}}]}};},async captureRegion(_desktop,request){return {region:request.region,capture:{sha256:'visual-proof'}};},async input(){inputCount++;throw Error('Native acknowledgement lost');}},
});
const privateBefore=await privatePort.observe(binding),input={binding,observation:privateBefore,control:'password',value:'owned-password',guard:async()=>{}};
await assert.rejects(privatePort.fill(input));await assert.rejects(privatePort.fill(input));assert.equal(inputCount,1);
console.log('Native receiving browser: lost native acknowledgement cannot replay the same private control');

let unclearedDispatches=0;
const uncleared=createNativeReceivingBrowser({desktop,guards,
 async readObservation(){return {binding:{...binding},origin:'https://accounts.example.test',state:'password_required',accountMatched:true,visiblePassword:true,maskedPassword:true,viewport:{width:1440,height:813,devicePixelRatio:1,focused:true,visible:true},controls:{password:{selector:'#password',count:1,tag:'INPUT',type:'password',visible:true,enabled:true,hitTarget:true,rect:{x:100,y:100,width:200,height:30}}}};},
 async inputMatches(){return false;},
 remote:{async observe(){return {...desktop};},async observation(){return {windows:{windows:[{id:42,active:true,processIdentity:{pid:123,start_token:'456'}}]}};},async captureRegion(_desktop,request){return {region:request.region,capture:{sha256:'visual-proof'}};},async input(_desktop,request){unclearedDispatches++;assert(!request.actions.some(x=>x.kind==='text'));return {delivery:'dispatched'};}},
});
const unclearedObservation=await uncleared.observe(binding);
await assert.rejects(uncleared.fill({binding,observation:unclearedObservation,control:'password',value:'owned-password',guard:async()=>{}}));assert.equal(unclearedDispatches,1);
console.log('Native receiving browser: missing empty-field proof prevents transmitting private text');

async function popupScenario({lostDismissal=false}={}){
 let popup=true,dismissals=0,submissions=0;
 const port=createNativeReceivingBrowser({desktop,guards,
  async readObservation(){return {binding:{...binding},origin:'https://accounts.example.test',state:'identifier_required',accountMatched:true,visiblePassword:false,maskedPassword:true,viewport:{width:1440,height:813,devicePixelRatio:1,focused:true,visible:true},controls:{identifierSubmit:{selector:'#submit',count:1,tag:'BUTTON',type:'submit',visible:true,enabled:true,hitTarget:!popup,dismissiblePopup:popup,rect:{x:100,y:100,width:200,height:30}}}};},
  async inputMatches(){assert.fail('Popup dismissal must not inspect credential values');},
  remote:{async observe(){return {...desktop};},async observation(){return {windows:{windows:[{id:42,active:true,processIdentity:{pid:123,start_token:'456'}}]}};},async captureRegion(_desktop,request){return {region:request.region,capture:{sha256:'popup-visual-proof'}};},async input(_desktop,request){assert.equal(request.expectedVisual.sha256,'popup-visual-proof');if(request.actions[0].kind==='key'){assert.deepEqual(request.actions,[{kind:'key',chord:'Escape'}]);dismissals++;if(lostDismissal)throw Error('Escape acknowledgement lost');popup=false;}else{assert.equal(popup,false);submissions++;}return {delivery:'dispatched'};}},
 });
 const observation=await port.observe(binding),input={binding,observation,control:'identifierSubmit',guard:async()=>{}};
 if(lostDismissal){await assert.rejects(port.activate(input));await assert.rejects(port.activate(input));assert.equal(dismissals,1);assert.equal(submissions,0);}
 else {await port.activate(input);assert.equal(dismissals,1);assert.equal(submissions,1);}
}
await popupScenario();
await popupScenario({lostDismissal:true});
console.log('Native receiving browser: recognized popup dismissed once before hit-tested submit; unknown dismissal cannot replay');

// A controlled identifier may restore its known account after an attempted clear.
// Exact current equality needs no destructive replacement or native input.
let prefilledDispatches=0,prefilledChanged=false,prefilledReadbacks=0;
const prefilled=createNativeReceivingBrowser({desktop,guards,
 async readObservation(){return {binding:{...binding},origin:'https://accounts.example.test',state:'identifier_required',accountMatched:true,visiblePassword:false,maskedPassword:true,viewport:{width:1440,height:813,devicePixelRatio:1,focused:true,visible:true},controls:{identifier:{selector:'#prefilled',count:1,tag:'INPUT',type:'text',visible:true,enabled:true,hitTarget:true,rect:{x:prefilledChanged?120:100,y:100,width:200,height:30}}}};},
 async inputMatches({selector,value}){prefilledReadbacks++;assert.equal(selector,'#prefilled');return value==='owned-user';},
 remote:{async observe(){return {...desktop};},async observation(){return {windows:{windows:[{id:42,active:true,processIdentity:{pid:123,start_token:'456'}}]}};},async captureRegion(_desktop,request){return {region:request.region,capture:{sha256:'prefilled-proof'}};},async input(){prefilledDispatches++;return {delivery:'dispatched'};}},
});
const prefilledBefore=await prefilled.observe(binding);
await prefilled.fill({binding,observation:prefilledBefore,control:'identifier',value:'owned-user',guard:async()=>{}});
assert.equal(prefilledDispatches,0,'Exact prefilled identifier must not be cleared or typed');
assert.equal(prefilledReadbacks,1);
prefilledChanged=true;
await assert.rejects(prefilled.fill({binding,observation:prefilledBefore,control:'identifier',value:'owned-user',guard:async()=>{}}));
assert.equal(prefilledReadbacks,1,'Stale controls cannot claim equality');assert.equal(prefilledDispatches,0);
console.log('Native receiving browser: exact prefilled identifier preserved, stale equality refused');
let identifierAckLost=false,identifierUnknownDispatches=0;
const identifierUnknown=createNativeReceivingBrowser({desktop,guards,
 async readObservation(){return {binding:{...binding},origin:'https://accounts.example.test',state:'identifier_required',accountMatched:true,visiblePassword:false,maskedPassword:true,viewport:{width:1440,height:813,devicePixelRatio:1,focused:true,visible:true},controls:{identifier:{selector:'#identifier',count:1,tag:'INPUT',type:'text',visible:true,enabled:true,hitTarget:true,rect:{x:100,y:100,width:200,height:30}}}};},
 async inputMatches(){return identifierAckLost;},
 remote:{async observe(){return {...desktop};},async observation(){return {windows:{windows:[{id:42,active:true,processIdentity:{pid:123,start_token:'456'}}]}};},async captureRegion(_desktop,request){return {region:request.region,capture:{sha256:'unknown-proof'}};},async input(){identifierUnknownDispatches++;identifierAckLost=true;throw Error('Unknown identifier delivery');}},
});
const unknownIdentifierBefore=await identifierUnknown.observe(binding),unknownIdentifierInput={binding,observation:unknownIdentifierBefore,control:'identifier',value:'owned-user',guard:async()=>{}};
await assert.rejects(identifierUnknown.fill(unknownIdentifierInput));
await assert.rejects(identifierUnknown.fill(unknownIdentifierInput));
assert.equal(identifierUnknownDispatches,1);
console.log('Native receiving browser: unknown identifier delivery cannot be converted into equality success');
