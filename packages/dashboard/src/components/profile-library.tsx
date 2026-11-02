"use client";
import { useCallback, useEffect, useState } from 'react';
import { loadProfileLibrary, saveProfileAssociations, profileAuthenticationState, type LibraryProfile, type ProfileLibraryData, type ProfileLibraryRow, type ProfileSiteAssociation } from '../lib/workspace-profile-library';
/** Public manager consumer. All writes preserve the owner's authentication evidence. */
export function ProfileLibrary({ onOpenGuides, onProfilesChange }: {
    onOpenGuides?: (sitePolicyId: string) => void;
    onProfilesChange?: (profiles: LibraryProfile[]) => void;
}) {
    const [data, setData] = useState<ProfileLibraryData>({ profiles: [], sitePolicies: [] });
    const [search, setSearch] = useState(''), [selected, setSelected] = useState<string | null>(null);
    const [draft, setDraft] = useState<ProfileSiteAssociation[]>([]), [error, setError] = useState(''), [notice, setNotice] = useState(''), [busy, setBusy] = useState(false);
    const load = useCallback(async () => { setBusy(true); setError(''); try {
        const next = await loadProfileLibrary();
        setData(next);
        onProfilesChange?.(next.profiles.map(row => row.profile));
    }
    catch (e) {
        setError(e instanceof Error ? e.message : 'Profile library unavailable');
    }
    finally {
        setBusy(false);
    } }, [onProfilesChange]);
    useEffect(() => { void load(); }, [load]);
    const row = data.profiles.find(row => row.profile.id === selected);
    const choose = (row: ProfileLibraryRow) => { setSelected(row.profile.id); setDraft(row.profile.siteAssociations.map(a => ({ ...a }))); setNotice(''); setError(''); };
    const edit = (index: number, key: keyof ProfileSiteAssociation, value: string) => setDraft(current => current.map((a, i) => i === index ? { ...a, [key]: value } : a));
    const save = async () => { if (!row)
        return; setBusy(true); setError(''); setNotice(''); try {
        const result = await saveProfileAssociations(row.profile.id, draft, row.profile.siteAssociations);
        setData(current => ({ ...current, profiles: current.profiles.map(item => item.profile.id === result.profile.id ? { ...item, profile: result.profile, relevantSitePolicyIds: [...new Set(result.profile.siteAssociations.map(a => a.sitePolicyId))] } : item) }));
        setNotice('Intended associations saved. Authentication evidence is unchanged.');
    }
    catch (e) {
        setError(e instanceof Error ? e.message : 'Association change could not be confirmed.');
    }
    finally {
        setBusy(false);
    } };
    const query = search.trim().toLowerCase();
    return <section aria-label="Profiles" style={{ minWidth: 0, maxWidth: '100%', overflowWrap: 'anywhere' }}>
  <label>Search profiles<input aria-label="Search profiles" value={search} onChange={e => setSearch(e.target.value)} style={{ maxWidth: '100%' }}/></label>
  <button type="button" disabled={busy} onClick={() => void load()}>Refresh profiles</button>
  {error && <p role="alert">{error}</p>}{notice && <p role="status">{notice}</p>}
  <ul>{data.profiles.filter(({ profile }) => JSON.stringify([profile.name, profile.id, profile.accountLabels, profile.loginIds, profile.accountIds, profile.aliases, profile.origins, profile.siteAssociations]).toLowerCase().includes(query)).map(item => <li key={item.profile.id}><button type="button" onClick={() => choose(item)} aria-pressed={selected === item.profile.id}>{item.profile.name || item.profile.id}</button><small> {item.profile.browserBuild || 'Browser build unknown'} · {item.currentUse.length ? `${item.currentUse.length} browser(s) in use` : 'No current browser use recorded'}</small></li>)}</ul>
  {!data.profiles.length && !busy && <p>No registered profiles.</p>}
  {row && <div aria-label={`Profile details for ${row.profile.name}`}>
   <h3>{row.profile.name}</h3><p>Stable profile ID: {row.profile.id}</p>
   <p>Browser/build: {row.profile.browserBuild || 'unknown'}</p>
   {row.profile.browserCompatibilityEvidence?.map((e, i) => <p key={i}>{e.browserFamily || 'Unknown browser'} {e.browserVersion || 'unknown version'}: {e.evidence}</p>)}
   <p>Current use: {row.currentUse.length ? row.currentUse.map(use => `${use.browserId} (${use.mode}), sessions ${use.sessionIds.join(', ') || 'unknown'}`).join('; ') : 'none recorded; mode unknown'}</p>
   <p>Observed identity: {row.observedIdentity.state}</p><p>Observed account: {row.observedIdentity.account || 'unknown'}</p>
   {!row.profile.targetReadiness?.length && <p>Authentication: unknown</p>}
   {row.profile.targetReadiness?.map((r, i) => <div key={i}><p>Authentication: {profileAuthenticationState(r)}</p><p>{r.targetServiceId}: {r.evidence || 'No observation evidence'}. Last verified: {r.lastVerifiedAt || 'unknown'}. Expires: {r.freshnessExpiresAt || 'unknown'}.</p></div>)}
   <p>Authentication is retained evidence. This view performs no live login verification.</p>
   <h4>Intended use</h4><p>These labels do not verify login or authorize website actions.</p>
   {draft.map((a, index) => <fieldset key={index} style={{ minWidth: 0 }}><legend>Association {index + 1}</legend>
    <label>Site<select aria-label={`Site ${index + 1}`} value={a.sitePolicyId} onChange={e => edit(index, 'sitePolicyId', e.target.value)}><option value="">Select site</option>{data.sitePolicies.map(site => <option key={site.id} value={site.id}>{site.name || site.id}</option>)}</select></label>
    <label>Account<input aria-label={`Account ${index + 1}`} value={a.account} onChange={e => edit(index, 'account', e.target.value)} style={{ maxWidth: '100%' }}/></label>
    <label>Organization / workspace<input aria-label={`Organization / workspace ${index + 1}`} value={a.organization} onChange={e => edit(index, 'organization', e.target.value)} style={{ maxWidth: '100%' }}/></label>
    <button type="button" disabled={busy} onClick={() => setDraft(current => current.filter((_, i) => i !== index))} aria-label={`Remove association ${index + 1}`}>Remove</button>
   </fieldset>)}
   <button type="button" disabled={busy} onClick={() => setDraft(current => [...current, { sitePolicyId: '', account: '', organization: '' }])}>Add association</button>
   <button type="button" disabled={busy || draft.some(a => !a.sitePolicyId || !a.account.trim() || !a.organization.trim())} onClick={() => void save()}>Save intended associations</button>
   <h4>Relevant SiteGuides</h4>{row.relevantSitePolicyIds.map(site => <button key={site} type="button" disabled={!onOpenGuides} title={onOpenGuides ? undefined : 'SiteGuides is unavailable in this view'} onClick={() => onOpenGuides?.(site)}>SiteGuides for {site}</button>)}
  </div>}
 </section>;
}
