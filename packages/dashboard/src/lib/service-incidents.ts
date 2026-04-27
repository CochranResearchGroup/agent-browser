export type ServiceIncidentSeverity = "info" | "warning" | "error" | "critical";

export type ServiceIncidentEscalation =
  | "none"
  | "browser_degraded"
  | "browser_recovery"
  | "job_attention"
  | "service_triage"
  | "os_degraded_possible";

export type ServiceIncidentPriorityInput = {
  label: string;
  severity?: ServiceIncidentSeverity | null;
  escalation?: ServiceIncidentEscalation | null;
  recommendedAction?: string | null;
};

export type ServiceIncidentPriorityView = {
  severityTone: ServiceIncidentSeverity;
  severityLabel: string;
  escalationLabel: string;
  recommendedAction: string | null;
  ariaLabel: string;
};

export function formatIncidentField(value?: string | null): string {
  return value ? value.replaceAll("_", " ") : "unknown";
}

export function incidentSeverityTone(value?: ServiceIncidentSeverity | null): ServiceIncidentSeverity {
  if (value === "warning" || value === "error" || value === "critical") return value;
  return "info";
}

export function incidentPriorityView(
  incident: ServiceIncidentPriorityInput,
): ServiceIncidentPriorityView {
  const severityTone = incidentSeverityTone(incident.severity);
  const recommendedAction = incident.recommendedAction?.trim() || null;
  return {
    severityTone,
    severityLabel: formatIncidentField(incident.severity),
    escalationLabel: formatIncidentField(incident.escalation),
    recommendedAction,
    ariaLabel: `Inspect ${formatIncidentField(incident.severity)} incident for ${incident.label}`,
  };
}
