import {connectHostedAuthHelper} from '../packages/client/src/hosted-auth-helper.js';

async function observeOriginalLogin() {
  const auth = await connectHostedAuthHelper();
  const channels: Record<string, unknown> = await auth.discoverLoginChannels({tenant: 'fixture', smsSuffix: '2428', im: {async callTool(request) {return {structuredContent: {accounts: []}, request};}}});
  void channels;
  const result = await auth.followLogin({requestId: 'synthetic-original'});
  const authentication: Record<string, unknown> | undefined = result.authentication;
  return authentication?.method;
}
void observeOriginalLogin;
