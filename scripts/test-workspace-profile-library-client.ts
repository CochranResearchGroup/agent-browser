import assert from 'node:assert/strict';
import { loadProfileLibrary, saveProfileAssociations, selectLibraryProfile } from '../packages/dashboard/src/lib/workspace-profile-library.ts';
const requests: unknown[] = [];
const original = globalThis.fetch;
globalThis.fetch = async (url, options) => {
    assert.equal(url, '/api/dashboard/profile-library');
    const body = options?.body ? JSON.parse(String(options.body)) : null;
    requests.push(body);
    const data = body?.operation === 'select' ? { state: 'choice_required', profileId: null, candidates: ['a', 'b'], reason: 'Multiple compatible profiles match', identityVerificationRequired: true } : body?.operation === 'replace_associations' ? { profile: { id: 'a', siteAssociations: body.associations, targetReadiness: [{ state: 'stale' }] } } : { profiles: [], sitePolicies: [] };
    return new Response(JSON.stringify({ success: true, data }));
};
try {
    assert.deepEqual(await loadProfileLibrary(), { profiles: [], sitePolicies: [] });
    const association = { sitePolicyId: 'site', account: 'person@example.test', organization: 'Research' };
    const saved = await saveProfileAssociations('a', [association], []);
    assert.equal(saved.profile.id, 'a');
    assert.deepEqual(requests[1], { operation: 'replace_associations', profileId: 'a', associations: [association], expectedAssociations: [] });
    const choice = await selectLibraryProfile({ ...association, profileId: 'b', browserBuild: 'stock_chrome', cdpConnected: false });
    assert.equal(choice.state, 'choice_required');
    assert.equal(choice.profileId, null);
    assert.deepEqual(requests[2], { operation: 'select', ...association, profileId: 'b', browserBuild: 'stock_chrome', cdpConnected: false });
    globalThis.fetch = async () => new Response(JSON.stringify({ success: false, error: 'Profile association conflict' }), { status: 409 });
    await assert.rejects(() => saveProfileAssociations('a', [], []), /Profile association conflict/);
    console.log('Profile library client public read/write/select and typed failure passed.');
}
finally {
    globalThis.fetch = original;
}
