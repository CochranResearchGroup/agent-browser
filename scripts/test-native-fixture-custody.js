import assert from 'node:assert/strict';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {loadNativeFixtureAudit,loadProtectedInputAudit} from './hosted-auth-native-fixture-host.mjs';
const root=await mkdtemp(join(tmpdir(),'p245-native-audit-'));
try{
 assert.deepEqual(await loadNativeFixtureAudit(root),{privateSubmissions:0,nativeOperationDispatches:0,nativeTextDispatches:0});
 const accepted={privateSubmissions:1,nativeOperationDispatches:6,nativeTextDispatches:2};
 await writeFile(join(root,'native-input-audit.json'),JSON.stringify(accepted),{mode:0o600});
 assert.deepEqual(await loadNativeFixtureAudit(root),accepted);assert.deepEqual(await loadNativeFixtureAudit(root),accepted);
 await writeFile(join(root,'native-input-audit.json'),'{"corrupt":true}');await assert.rejects(loadNativeFixtureAudit(root));
 await assert.rejects(loadProtectedInputAudit(root));
 const protectedAudit={version:1,passwordFills:1,mfaFills:0};await writeFile(join(root,'protected-input-audit.json'),JSON.stringify(protectedAudit),{mode:0o600});
 assert.deepEqual(await loadProtectedInputAudit(root),protectedAudit);assert.deepEqual(await loadProtectedInputAudit(root),protectedAudit);
 await writeFile(join(root,'protected-input-audit.json'),'{"corrupt":true}');await assert.rejects(loadProtectedInputAudit(root));
 console.log('Native fixture custody: restart preserves execution counts; corrupt audit cannot become zero');
}finally{await rm(root,{recursive:true,force:true});}
