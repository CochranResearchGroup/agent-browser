import {dashboardLocalFailure} from "./response-failure.ts";
import {dashboardResponseFailureError,responseFailureMetadata} from "./response-failure.ts";
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
    throw dashboardLocalFailure(new TypeError("serviceProfileAllocationLookupUrl requires a service base URL"),"dashboard_argument_invalid","packages/dashboard/src/lib/service-profile-allocation.ts","serviceProfileAllocationLookupUrl");
  }
  if (!trimmedProfileId) {
    throw dashboardLocalFailure(new TypeError("serviceProfileAllocationLookupUrl requires a profile ID"),"dashboard_argument_invalid","packages/dashboard/src/lib/service-profile-allocation.ts","serviceProfileAllocationLookupUrl");
  }
  return `${trimmedBase}/profiles/${encodeURIComponent(trimmedProfileId)}/allocation`;
}

export function profileAllocationFromLookupPayload<TAllocation>(
  payload: ServiceProfileAllocationLookupPayload<TAllocation>,
  fallback: TAllocation,
): TAllocation {
  if (!payload.success || responseFailureMetadata(payload)) {
    throw dashboardResponseFailureError(payload, "service_profile_allocation_failed");
  }
  return payload.data?.profileAllocation ?? fallback;
}
