import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { getTaskOperation } from '../packages/client/src/task-briefing.js';
const calls=[];
const server=createServer((req,res)=>{
 calls.push({method:req.method,url:req.url});
 res.setHeader('content-type','application/json');
 res.end(JSON.stringify({success:true,data:{schemaVersion:'agent-browser.task-operation.v1',operation:{taskId:'session:task',requestId:'original / fill',state:'uncertain',retryAllowed:false}}}));
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
try {
 const baseUrl=`http://127.0.0.1:${server.address().port}`;
 const result=await getTaskOperation({baseUrl,taskId:'session:task',requestId:'original / fill'});
 assert.equal(result.operation.state,'uncertain');
 assert.deepEqual(calls,[{method:'GET',url:'/api/browser/tasks/session%3Atask/operations/original%20%2F%20fill'}]);
 await assert.rejects(getTaskOperation({baseUrl,taskId:'wrong',requestId:'original / fill'}),/identity/);
 assert.equal(calls.length,2,'mismatch never retries');
 console.log('Original task operation client protocol cases passed');
} finally {await new Promise(resolve=>server.close(resolve));}
