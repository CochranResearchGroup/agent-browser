#!/usr/bin/env node
/** Real public MCP discovery and refusal trial. No browser/provider is launched. */
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync,readdirSync,existsSync} from 'node:fs';
import {isAbsolute} from 'node:path';
import {createMcpStdioClient,createSmokeContext} from './smoke-utils.js';

const binary=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
assert(binary&&isAbsolute(binary),'Pin an absolute reviewed candidate executable');
const digest=createHash('sha256').update(readFileSync(binary)).digest('hex');
if(process.env.HOST_GUIDANCE_EXPECTED_BINARY_SHA256)assert.equal(digest,process.env.HOST_GUIDANCE_EXPECTED_BINARY_SHA256);
const context=createSmokeContext({prefix:'ab-host-guide-',sessionPrefix:'host-guide'});
// Discard inherited provider, profile, login and runtime configuration.
context.env=Object.fromEntries(Object.entries(context.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')));
Object.assign(context.env,{AGENT_BROWSER_HOME:context.agentHome,AGENT_BROWSER_SOCKET_DIR:context.socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary});
const client=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{}});
const checks=[];
const call=(name,args)=>client.send('tools/call',{name,arguments:args});
const data=result=>{assert(!result.isError);const payload=JSON.parse(result.content.find(x=>x.type==='text').text);assert(payload.success);return payload.data;};
const fail=async(name,args,pattern)=>{await assert.rejects(call(name,args),pattern);checks.push(pattern.source);};
try{
 await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'host-guidance-v1-trial',version:'1'}});
 client.notify('notifications/initialized');
 const {tools}=await client.send('tools/list');
 assert.equal(tools.length,16,'Current host guidance requires the16-tool connected catalog');
 for(const name of ['browser_reconnect','browser_policy','browser_artifact'])assert(tools.some(tool=>tool.name===name),`Discover ${name} before dispatch`);
 const reconnect=tools.find(tool=>tool.name==='browser_reconnect').inputSchema.properties;
 assert.deepEqual(reconnect.operation.enum,['status','disconnect','reconnect','rebind','configure','reopen']);
 assert.deepEqual(reconnect.lossPolicy.enum,['report_only','reopen']);
 const policy=tools.find(tool=>tool.name==='browser_policy').inputSchema.properties;
 assert.deepEqual(policy.operation.enum,['status','set_app','set_task','clear_app','clear_task','override']);
 assert.equal(policy.policy.properties.version.const,1);
 const artifact=tools.find(tool=>tool.name==='browser_artifact').inputSchema.properties;
 assert.deepEqual(artifact.kind.enum,['screenshot','download']);
 checks.push('16-tool catalog and reconnect/policy/artifact schemas discovered');
 const open=tools.find(x=>x.name==='browser_open').inputSchema;
 for(const field of ['taskKey','serviceName','agentName','taskName'])assert(open.required.includes(field));
 for(const field of ['profileMode','hostBinding','attachment','taskPolicy'])assert(open.properties[field]);
 const control=tools.find(x=>x.name==='browser_control');assert(control.inputSchema.properties.operation);
 checks.push('connected launch/attach/control schemas discovered');
 const contracts=await client.send('resources/read',{uri:'agent-browser://contracts'});
 assert(JSON.parse(contracts.contents[0].text).mcpCatalog.tools.includes('browser_open'));
 await client.send('resources/read',{uri:'agent-browser://operating-guide'});
 const advanced=data(await call('browser_discover',{category:'browser',operation:'press'}));assert(advanced.inputSchema);
 checks.push('contracts, operating guide and selected advanced schema read');
 const request={taskKey:'synthetic-public-draft',serviceName:'guidance-fixture',agentName:'synthetic-worker',taskName:'Inspect public example.test draft',url:'https://example.test',profileMode:'existing',profile:'synthetic-public'};
 await fail('browser_open',{...request,profileMode:'invalid'},/browser_profile_mode_invalid/);
 await fail('browser_open',{...request,profileMode:'disposable'},/disposable_profile_must_not_name_a_persistent_profile/);
 const {profile,...unnamed}=request;await fail('browser_open',unnamed,/browser_profile_name_required/);
 await fail('browser_open',{...request,attachment:{}},/browser_attachment_does_not_navigate_or_allocate_profile/);
 await fail('browser_open',{...request,hostBinding:{version:99,hostId:'synthetic-app',participantId:'worker',sessionId:'original-session',placement:{mode:'required',desktop:{desktopId:'synthetic-desktop',generation:1}}}},/browser_host_binding_version_unsupported/);
 await fail('browser_command',{taskId:'not-created',action:'press',params:{key:'Enter',sessionName:'peer'}},/reserved/);
 await fail('browser_control',{taskId:'not-created',operation:'status'},/browser_runtime_database_read_only_unavailable/);
 await fail('browser_reconnect',{taskId:'not-created',operation:'status'},/browser_runtime_database_read_only_unavailable/);
 await fail('browser_artifact',{taskId:'not-created',kind:'publish',handle:'private',name:'page.png'},/browser_artifact_kind_invalid/);
 assert(!existsSync(context.agentHome+'/runtime-profiles'),'No profile allocated');
 assert(!readdirSync(context.socketDir).some(name=>name.endsWith('.sock')),'No daemon socket created');
 const skill=readFileSync(new URL('../skills/agent-browser/SKILL.md',import.meta.url));assert(skill.byteLength<10000,'Entry skill stays below10k');
 console.log(JSON.stringify({guidance:'agent-browser.host-guidance.v1',binarySha256:digest,scope:'Real public MCP discovery and synthetic invalid-intent refusals only; no successful browser workflow or provider-readiness claim',checks,skillBytes:skill.byteLength}));
}finally{client.close();context.cleanupTempHome();}
