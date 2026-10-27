import {connectHostedAuthHelper} from '../packages/client/src/hosted-auth-helper.js';

async function observeOriginalLogin() {
  const auth = await connectHostedAuthHelper();
  const result = await auth.followLogin({requestId: 'synthetic-original'});
  const authentication: Record<string, unknown> | undefined = result.authentication;
  return authentication?.method;
}
void observeOriginalLogin;
