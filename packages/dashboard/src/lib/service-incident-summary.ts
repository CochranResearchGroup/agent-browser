import {
  formatIncidentField,
  incidentSeverityTone,
  type ServiceIncidentEscalation,
  type ServiceIncidentSeverity,
} from "./service-incidents.ts";

export type ServiceIncidentSummaryGroup = {
  escalation?: ServiceIncidentEscalation | null;
  severity?: ServiceIncidentSeverity | null;
  state?: string | null;
  count?: number;
  latestTimestamp?: string | null;
  recommendedAction?: string | null;
  incidentIds?: string[];
};

export type ServiceIncidentsData = {
  incidents?: unknown[];
  count?: number;
  matched?: number;
  total?: number;
  summary?: {
    groupCount?: number;
    groups?: ServiceIncidentSummaryGroup[];
  };
};

export type ServiceIncidentSummaryGroupView = {
  key: string;
  severityTone: ServiceIncidentSeverity;
  severityLabel: string;
  escalationLabel: string;
  stateLabel: string;
  count: number;
  latestTimestamp?: string | null;
  recommendedAction: string;
  incidentIds: string[];
  incidentIdLabel: string;
};

function incidentSeverityRank(value?: ServiceIncidentSeverity | null): number {
  if (value === "critical") return 4;
  if (value === "error") return 3;
  if (value === "warning") return 2;
  if (value === "info") return 1;
  return 0;
}

export function sortIncidentSummaryGroups(
  groups: ServiceIncidentSummaryGroup[],
): ServiceIncidentSummaryGroup[] {
  return [...groups].sort((left, right) => {
    const severityDelta = incidentSeverityRank(right.severity) - incidentSeverityRank(left.severity);
    if (severityDelta !== 0) return severityDelta;
    const countDelta = (right.count ?? 0) - (left.count ?? 0);
    if (countDelta !== 0) return countDelta;
    return new Date(right.latestTimestamp ?? 0).getTime() - new Date(left.latestTimestamp ?? 0).getTime();
  });
}

export function incidentSummaryGroupView(
  group: ServiceIncidentSummaryGroup,
): ServiceIncidentSummaryGroupView {
  const incidentIds = group.incidentIds ?? [];
  const visibleIncidentIds = incidentIds.slice(0, 4);
  const hiddenCount = Math.max(incidentIds.length - visibleIncidentIds.length, 0);
  return {
    key: `${group.escalation ?? "unknown"}-${group.severity ?? "unknown"}-${group.state ?? "unknown"}`,
    severityTone: incidentSeverityTone(group.severity),
    severityLabel: formatIncidentField(group.severity),
    escalationLabel: formatIncidentField(group.escalation),
    stateLabel: formatIncidentField(group.state),
    count: group.count ?? incidentIds.length,
    latestTimestamp: group.latestTimestamp,
    recommendedAction: group.recommendedAction || "Inspect incident details.",
    incidentIds,
    incidentIdLabel: visibleIncidentIds.length > 0
      ? `${visibleIncidentIds.join(" / ")}${hiddenCount > 0 ? ` +${hiddenCount}` : ""}`
      : "No incident IDs",
  };
}

export function incidentSummaryGroupViews(
  groups: ServiceIncidentSummaryGroup[],
): ServiceIncidentSummaryGroupView[] {
  return sortIncidentSummaryGroups(groups).map(incidentSummaryGroupView);
}
