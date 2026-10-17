import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { buildNativeRailGroups } from '../packages/dashboard/src/lib/native-rail.ts';

const nodes = Array.from({length: 500}, (_, i) => ({ id: `node-${i}`, browserId: `b-${i}`, label: `Browser ${i}`, profileId: `profile-${i}`, live: true })) as any;
const tabs = Array.from({length: 10000}, (_, i) => ({ id: `t-${i}`, browserId: `b-${i % 500}`, title: `Tab ${i}`, url: `https://example.test/${i}`, managedTask: true, presence: {state: i % 13 === 0 ? 'unknown' : i % 17 === 0 ? 'absent' : 'live'}, lifecycle: i % 17 === 0 ? 'expired' : 'ready' })) as any;
const closed = new Set(['t-1', 't-502']);
function previous(query: string) {
  const search = query.trim().toLowerCase();
  return nodes.map((node: any) => {
    const browserTabs = tabs.filter((tab: any) => (tab.managedTask !== true || tab.presence?.state === 'live') && tab.browserId === node.browserId && tab.lifecycle !== 'closed' && tab.lifecycle !== 'expired' && !closed.has(tab.id));
    const browserMatches = `${node.label} ${node.profileId ?? ''}`.toLowerCase().includes(search);
    return {node,browserTabs,visibleTabs:browserTabs.filter((tab: any) => !search || browserMatches || `${tab.title ?? ''} ${tab.taskLabel ?? ''} ${tab.url ?? ''}`.toLowerCase().includes(search))};
  }).filter((group: any) => group.browserTabs.length > 0 && (!search || `${group.node.label} ${group.node.profileId}`.toLowerCase().includes(search) || group.visibleTabs.length));
}
const shape = (groups: any[]) => groups.map(group => [group.node.id, group.browserTabs.map((tab: any) => tab.id), group.visibleTabs.map((tab: any) => tab.id)]);
for (const query of ['', 'PROFILE-12', ' Tab 502 ', 'missing']) assert.deepEqual(shape(buildNativeRailGroups(nodes,tabs,closed,query).groups), shape(previous(query)));
const document = {...tabs[1],title:null,url:'data:text/html,HIDDEN-PAYLOAD-' + 'x'.repeat(1_000_000)};
assert.equal(buildNativeRailGroups(nodes,[document],new Set(),'HTML document').groups.length,1,'search must match the displayed document label');
assert.equal(buildNativeRailGroups(nodes,[document],new Set(),'HIDDEN-PAYLOAD').groups.length,0,'embedded document payloads are not rail search metadata');
let inspected = 0;
const counted = tabs.map((tab: any) => ({...tab,get browserId(){ inspected++; return tab.browserId; }}));
const result = buildNativeRailGroups(nodes,counted,closed,'');
assert.ok(inspected <= tabs.length * 4, 'tab grouping must stay linear as browser count grows');
assert.equal(result.unverifiedTabs,true);
assert.equal(buildNativeRailGroups([{...nodes[0],browserId:'empty'}],[],new Set(),'').groups.length,0);
const samples = (run: () => unknown) => Array.from({length:15},()=>{const start=performance.now();run();return performance.now()-start;}).sort((a,b)=>a-b);
const old = samples(()=>previous(''));
const current = samples(()=>buildNativeRailGroups(nodes,tabs,closed,''));
console.log(JSON.stringify({browsers:nodes.length,tabs:tabs.length,tabIdentityReads:inspected,beforeMedianMs:old[7],afterMedianMs:current[7],speedup:old[7]/current[7],note:'grouping computation only; not end-to-end UI latency'}));
