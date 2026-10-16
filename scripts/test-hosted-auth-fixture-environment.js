import assert from 'node:assert/strict';
import {browserFixtureEnvironment} from './hosted-auth-fixture-environment.js';
const input={HOME:'/owned-home',PATH:'/usr/bin',DISPLAY:':1',REMOTE_VIEW_SLOT_GENERATION:'2',AGENT_BROWSER_HOME:'/owned-home/.agent-browser',OPENAI_API_KEY:'private-marker',MSGCLI_KEYRING_PASSWORD:'private-marker',AGENT_BROWSER_DASHBOARD_PASSWORD:'private-marker',UNRELATED_VALUE:'private-marker'};
const clean=browserFixtureEnvironment(input);
assert.deepEqual(clean,{HOME:'/owned-home',PATH:'/usr/bin',DISPLAY:':1',REMOTE_VIEW_SLOT_GENERATION:'2',AGENT_BROWSER_HOME:'/owned-home/.agent-browser'});
assert.equal(input.OPENAI_API_KEY,'private-marker');assert(!Object.values(clean).includes('private-marker'));
console.log('Hosted fixture environment: only browser runtime values retained; parent credentials unchanged');
