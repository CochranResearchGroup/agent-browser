/** Intended site labels are metadata, never authenticated identity evidence. */
export type ProfileSiteAssociation = {
    sitePolicyId: string;
    account: string;
    organization: string;
};
export type LibraryProfile = {
    id: string;
    name: string;
    browserBuild?: string | null;
    siteAssociations: ProfileSiteAssociation[];
    siteAssociationsUpdatedAt?: string | null;
    targetReadiness?: {
        targetServiceId: string;
        loginId?: string | null;
        state: string;
        evidence?: string;
        lastVerifiedAt?: string | null;
        freshnessExpiresAt?: string | null;
    }[];
    browserCompatibilityEvidence?: {
        browserFamily?: string | null;
        browserVersion?: string | null;
        evidence: string;
    }[];
    accountLabels?: string[];
    loginIds?: string[];
    accountIds?: string[];
    aliases?: string[];
    origins?: string[];
};
export type ProfileLibraryRow = {
    profile: LibraryProfile;
    currentUse: {
        browserId: string;
        mode: string;
        sessionIds: string[];
    }[];
    observedIdentity: {
        state: string;
        account: string | null;
    };
    relevantSitePolicyIds: string[];
};
export type ProfileLibraryData = {
    profiles: ProfileLibraryRow[];
    sitePolicies: {
        id: string;
        name: string;
    }[];
};
export type ProfileSelectionRequest = {
    sitePolicyId: string;
    account: string;
    organization: string;
    profileId?: string;
    browserBuild?: string;
    cdpConnected?: boolean;
};
export type ProfileSelection = {
    state: 'selected' | 'choice_required' | 'conflict';
    profileId: string | null;
    candidates: string[];
    reason: string;
    identityVerificationRequired: boolean;
};
async function request<T>(body?: unknown): Promise<T> {
    const response = await fetch('/api/dashboard/profile-library', body === undefined ? { cache: 'no-store' } : { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
    const payload = await response.json();
    if (!response.ok || !payload.success || !payload.data)
        throw new Error(payload.error || 'Profile library request could not be confirmed.');
    return payload.data as T;
}
export function loadProfileLibrary(): Promise<ProfileLibraryData> { return request(); }
/** Replace only intended metadata, using a compare-and-set to preserve concurrent edits. */
export function saveProfileAssociations(profileId: string, associations: ProfileSiteAssociation[], expectedAssociations: ProfileSiteAssociation[]): Promise<{
    profile: LibraryProfile;
}> { return request({ operation: 'replace_associations', profileId, associations, expectedAssociations }); }
/** Selection grants no website authority and requires independent procedure identity checks. */
export function selectLibraryProfile(selection: ProfileSelectionRequest): Promise<ProfileSelection> { return request({ operation: 'select', ...selection }); }
/** Retained verification never renews when the library is read. */
export function profileAuthenticationState(readiness: {
    state: string;
    lastVerifiedAt?: string | null;
    freshnessExpiresAt?: string | null;
}, now = Date.now()): string {
    if (readiness.state !== 'fresh')
        return readiness.state;
    const verified = Date.parse(readiness.lastVerifiedAt || ''), expires = Date.parse(readiness.freshnessExpiresAt || '');
    if (!Number.isFinite(verified) || !Number.isFinite(expires) || verified > now)
        return 'unknown';
    return expires <= now ? 'stale' : 'fresh';
}
