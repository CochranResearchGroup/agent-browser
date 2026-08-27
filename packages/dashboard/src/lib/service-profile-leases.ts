export type ServiceProfileLeaseAction = "rejoin" | "renew" | "release" | "reconcile";

export type ServiceProfileLeaseRecord = {
  id: string;
  leaseRevision: string;
  principalId?: string | null;
  principalProvenance?: string | null;
  profileId: string;
  browserId?: string | null;
  sessionIds?: string[];
  tabIds?: string[];
  mode?: string;
  state?: string;
  ownerGeneration?: number | null;
  routeIds?: string[];
  lastHeartbeatAt?: string | null;
  expiresAt?: string | null;
  cleanupObligation?: string | null;
  blockingIdentityAxes?: string[];
  authorizedActions?: string[];
  recourse?: string;
  observationOnly?: boolean;
};

export type ServiceProfileLeaseFinding = {
  code: string;
  severity: string;
  leaseId: string;
  profileId: string;
  message: string;
  safeActions?: string[];
};

export type ServiceProfileLeasesData = {
  profileLeases?: ServiceProfileLeaseRecord[];
  count?: number;
  observedAt?: string;
  doctor?: {
    healthy?: boolean;
    leaseCount?: number;
    findings?: ServiceProfileLeaseFinding[];
  };
};

export type ProfileLeaseProjection = {
  profileId?: string | null;
  profileLease?: ServiceProfileLeaseRecord | null;
  profileLeaseFindings?: ServiceProfileLeaseFinding[];
};

export function projectProfileLeases<T extends ProfileLeaseProjection>(
  allocations: T[],
  data?: ServiceProfileLeasesData | null,
): T[] {
  const leaseByProfileId = new Map(
    (data?.profileLeases ?? []).map((lease) => [lease.profileId, lease]),
  );
  const findingsByLeaseId = new Map<string, ServiceProfileLeaseFinding[]>();
  for (const finding of data?.doctor?.findings ?? []) {
    const findings = findingsByLeaseId.get(finding.leaseId) ?? [];
    findings.push(finding);
    findingsByLeaseId.set(finding.leaseId, findings);
  }
  return allocations.map((allocation) => {
    const profileLease = allocation.profileId
      ? leaseByProfileId.get(allocation.profileId) ?? null
      : null;
    return {
      ...allocation,
      profileLease,
      profileLeaseFindings: profileLease
        ? findingsByLeaseId.get(profileLease.id) ?? []
        : [],
    };
  });
}

export function profileLeaseActionAllowed(
  lease: ServiceProfileLeaseRecord | null | undefined,
  action: ServiceProfileLeaseAction,
): boolean {
  return Boolean(
    lease &&
    !lease.observationOnly &&
    lease.authorizedActions?.includes(action),
  );
}

export function defaultProfileLeaseExpiry(now = new Date()): string {
  return new Date(now.getTime() + 60 * 60 * 1000).toISOString().slice(0, 16);
}

export function profileLeaseExpiryToIso(value: string): string {
  const parsed = new Date(value);
  if (!value.trim() || Number.isNaN(parsed.getTime())) {
    throw new TypeError("Enter a valid lease expiry time");
  }
  return parsed.toISOString();
}
