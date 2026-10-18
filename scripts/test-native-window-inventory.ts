import assert from 'node:assert/strict';
import { nativeBrowserWindows } from '../packages/dashboard/src/lib/native-window-inventory.ts';

const desktop = { desktopId: '11111111-1111-4111-8111-111111111111', slot: 2, generation: 7,
  displayName: 'Research', application: 'research', windows: [
    { id: 31, pid: 500, title: 'Paper search', class: 'Chromium' },
    { id: 32, pid: 501, title: '', class: 'Firefox' },
  ] };
const snapshot = { schemaVersion: 1, source: 'remote-view-native-windows', observedAtMs: 1000,
  observationAgeMs: 300, unverifiedDesktopCount: 1, desktops: [desktop] };
const result = nativeBrowserWindows(snapshot);
assert.equal(result.windows.length, 2);
assert.equal(result.windows[0].key, `${desktop.desktopId}:7:31`);
assert.equal(result.windows[1].title, 'Firefox');
assert.equal(result.ageMs, 300);
assert.equal(result.unverified, 1);
assert.equal(nativeBrowserWindows({ ...snapshot, desktops: [{ ...desktop, windows: [] }] }).windows.length, 0);
for (const invalid of [
  { ...snapshot, source: 'managed-owner' },
  { ...snapshot, observationAgeMs: -1 },
  { ...snapshot, observedAtMs: Number.NaN },
  { ...snapshot, desktops: [desktop, desktop] },
  { ...snapshot, desktops: [{ ...desktop, generation: 0 }] },
  { ...snapshot, desktops: [{ ...desktop, windows: [{ ...desktop.windows[0], id: 0 }] }] },
]) assert.throws(() => nativeBrowserWindows(invalid));
assert.equal(nativeBrowserWindows({ ...snapshot, desktops: [{ ...desktop, generation: 8 }] }).windows[0].key,
  `${desktop.desktopId}:8:31`, 'a new desktop generation must not reuse the previous selection');
console.log('Native window inventory identity, empty-desktop and invalid snapshot checks passed');
