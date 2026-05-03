export type ServiceProfileAllocationLookupPayload<TAllocation> = {
  success?: boolean;
  data?: {
    profileAllocation?: TAllocation;
  } | null;
  error?: string | null;
};

export function serviceProfileAllocationLookupUrl(serviceBaseUrl: string, profileId: string): string {
  const trimmedBase = serviceBaseUrl.trim().replace(/\/+$/, "");
  const trimmedProfileId = profileId.trim();
  if (!trimmedBase) {
    throw new TypeError("serviceProfileAllocationLookupUrl requires a service base URL");
  }
  if (!trimmedProfileId) {
    throw new TypeError("serviceProfileAllocationLookupUrl requires a profile ID");
  }
  return `${trimmedBase}/profiles/${encodeURIComponent(trimmedProfileId)}/allocation`;
}

export function profileAllocationFromLookupPayload<TAllocation>(
  payload: ServiceProfileAllocationLookupPayload<TAllocation>,
  fallback: TAllocation,
): TAllocation {
  if (!payload.success) {
    throw new Error(payload.error || "Service profile allocation lookup failed");
  }
  return payload.data?.profileAllocation ?? fallback;
}
