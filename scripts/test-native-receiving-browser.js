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
