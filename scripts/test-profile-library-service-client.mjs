import assert from 'node:assert/strict';
import {createServiceRequest,createServiceRequestMcpToolCall} from '../packages/client/src/service-request.js';
const request={action:'workspace_profile_library',serviceName:'Profiles',agentName:'fixture',taskName:'Choose intended context',params:{operation:'select',sitePolicyId:'site',account:'a',organization:'org',profileId:'explicit'}};
assert.deepEqual(createServiceRequest(request),request);
assert.deepEqual(createServiceRequestMcpToolCall(request),{name:'service_request',arguments:request});
console.log('Profile library service client and MCP composition passed.');
