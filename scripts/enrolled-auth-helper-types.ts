import {connectEnrolledAuthHelper,connectHostedAuthHelper,type AuthOperationContext,type OwnerAuthHelper} from '../packages/client/src/hosted-auth-helper.js';
const operationContext:AuthOperationContext={operationId:'d03cd7bc-95af-4601-a171-de46a78ea3f1',recoveryId:'932b97f6-2836-4d59-89d4-8db425a67c4d',binding:{taskId:'task',browserId:'browser',profileId:'profile',tabId:'tab',origin:'https://example.test',controlGeneration:'generation'}};
async function enrolled(){
 const helper=await connectEnrolledAuthHelper({discoveryFile:'/private/consumer-discovery.json',operationContext});
 const compatible:OwnerAuthHelper=helper;
 const existing:Awaited<ReturnType<typeof connectHostedAuthHelper>>=compatible;
 return existing.followLogin({requestId:'retained-request'});
}
// @ts-expect-error Enrolled consumers must explicitly select their discovery.
void connectEnrolledAuthHelper({operationContext});
// @ts-expect-error A copied Codex caller is not an enrolled helper option.
void connectEnrolledAuthHelper({discoveryFile:'/private/discovery.json',operationContext,threadId:'foreign'});
void enrolled;
