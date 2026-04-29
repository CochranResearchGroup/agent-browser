export type ServiceTraceFiltersData = {
  browserId?: string | null;
  profileId?: string | null;
  sessionId?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
  since?: string | null;
  limit?: number;
};

export type ServiceTraceEvent = {
  id: string;
  timestamp: string;
  kind: string;
  message?: string | null;
  browserId?: string | null;
  profileId?: string | null;
  sessionId?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
};

export type ServiceTraceJob = {
  id: string;
  action?: string | null;
  state?: string | null;
  submittedAt?: string | null;
  startedAt?: string | null;
  completedAt?: string | null;
  error?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
};

export type ServiceTraceTimelineItem = {
  id: string;
  timestamp: string;
  kind: string;
  title: string;
  message?: string | null;
  source?: string;
  browserId?: string | null;
  profileId?: string | null;
  sessionId?: string | null;
  serviceName?: string | null;
  agentName?: string | null;
  taskName?: string | null;
  eventId?: string | null;
  jobId?: string | null;
};

export type ServiceTraceData = {
  filters?: ServiceTraceFiltersData;
  events?: ServiceTraceEvent[];
  jobs?: ServiceTraceJob[];
  incidents?: unknown[];
  activity?: ServiceTraceTimelineItem[];
  summary?: {
    contextCount?: number;
    hasTraceContext?: boolean;
    contexts?: {
      serviceName?: string | null;
      agentName?: string | null;
      taskName?: string | null;
      browserId?: string | null;
      profileId?: string | null;
      sessionId?: string | null;
      eventCount?: number;
      jobCount?: number;
      incidentCount?: number;
      activityCount?: number;
      latestTimestamp?: string | null;
    }[];
  };
  counts?: {
    events?: number;
    jobs?: number;
    incidents?: number;
    activity?: number;
  };
  matched?: {
    events?: number;
    jobs?: number;
    incidents?: number;
    activity?: number;
  };
  total?: {
    events?: number;
    jobs?: number;
    incidents?: number;
  };
};

export type ServiceTraceSummaryContext = NonNullable<
  NonNullable<ServiceTraceData["summary"]>["contexts"]
>[number];

export type ServiceTraceSummaryCard = {
  key: string;
  title: string;
  subtitle: string;
  total: number;
  meta: string[];
  counts: string[];
};

export type ServiceTraceToolPayload = {
  tool?: string;
  success?: boolean;
  data?: ServiceTraceData;
  error?: unknown;
};

function formatEventKind(kind: string): string {
  return kind.replaceAll("_", " ");
}

function serviceJobTimestamp(job: ServiceTraceJob): string {
  return job.completedAt ?? job.startedAt ?? job.submittedAt ?? "";
}

export function normalizeServiceTraceData(
  value: ServiceTraceData | ServiceTraceToolPayload | null | undefined,
): ServiceTraceData | null {
  if (!value) return null;
  if ("tool" in value && value.tool === "service_trace") return value.data ?? null;
  return value as ServiceTraceData;
}

export function traceFilterSummary(filters?: ServiceTraceFiltersData): string {
  if (!filters) return "No returned filters";
  const parts = [
    filters.serviceName && `service ${filters.serviceName}`,
    filters.agentName && `agent ${filters.agentName}`,
    filters.taskName && `task ${filters.taskName}`,
    filters.browserId && `browser ${filters.browserId}`,
    filters.profileId && `profile ${filters.profileId}`,
    filters.sessionId && `session ${filters.sessionId}`,
    filters.since && `since ${filters.since}`,
    filters.limit && `limit ${filters.limit}`,
  ].filter((value): value is string => typeof value === "string" && value.length > 0);
  return parts.length > 0 ? parts.join(" / ") : "No returned filters";
}

export function traceSummaryContexts(trace: ServiceTraceData | null): ServiceTraceSummaryContext[] {
  return [...(trace?.summary?.contexts ?? [])].sort((left, right) => {
    const rightCount =
      (right.eventCount ?? 0) +
      (right.jobCount ?? 0) +
      (right.incidentCount ?? 0) +
      (right.activityCount ?? 0);
    const leftCount =
      (left.eventCount ?? 0) +
      (left.jobCount ?? 0) +
      (left.incidentCount ?? 0) +
      (left.activityCount ?? 0);
    if (rightCount !== leftCount) return rightCount - leftCount;
    return (right.latestTimestamp ?? "").localeCompare(left.latestTimestamp ?? "");
  });
}

export function traceSummaryCards(trace: ServiceTraceData | null, limit = 4): ServiceTraceSummaryCard[] {
  return traceSummaryContexts(trace)
    .slice(0, limit)
    .map((context, index) => {
      const total =
        (context.eventCount ?? 0) +
        (context.jobCount ?? 0) +
        (context.incidentCount ?? 0) +
        (context.activityCount ?? 0);
      return {
        key: [
          context.serviceName,
          context.agentName,
          context.taskName,
          context.browserId,
          context.profileId,
          context.sessionId,
          index,
        ].join(":"),
        title: context.serviceName ?? "Unlabeled service",
        subtitle: context.taskName ?? "untitled task",
        total,
        meta: [
          context.agentName && `agent ${context.agentName}`,
          context.browserId && `browser ${context.browserId}`,
          context.profileId && `profile ${context.profileId}`,
          context.sessionId && `session ${context.sessionId}`,
        ].filter((value): value is string => typeof value === "string" && value.length > 0),
        counts: [
          `${context.eventCount ?? 0} ev`,
          `${context.jobCount ?? 0} jobs`,
          `${context.incidentCount ?? 0} inc`,
          `${context.activityCount ?? 0} act`,
        ],
      };
    });
}

export function traceTimelineItems(trace: ServiceTraceData | null): ServiceTraceTimelineItem[] {
  if (!trace) return [];
  const seenEventIds = new Set(
    (trace.activity ?? [])
      .map((item) => item.eventId)
      .filter((id): id is string => typeof id === "string" && id.length > 0),
  );
  const seenJobIds = new Set(
    (trace.activity ?? [])
      .map((item) => item.jobId)
      .filter((id): id is string => typeof id === "string" && id.length > 0),
  );
  const activityItems = (trace.activity ?? []).map((item) => ({
    ...item,
    source: item.source ?? "activity",
    title: item.title || formatEventKind(item.kind),
  }));
  const eventItems = (trace.events ?? [])
    .filter((event) => !seenEventIds.has(event.id))
    .map((event) => ({
      id: `trace-event-${event.id}`,
      eventId: event.id,
      source: "event",
      timestamp: event.timestamp,
      kind: event.kind,
      title: formatEventKind(event.kind),
      message: event.message,
      browserId: event.browserId,
      profileId: event.profileId,
      sessionId: event.sessionId,
      serviceName: event.serviceName,
      agentName: event.agentName,
      taskName: event.taskName,
    }));
  const jobItems = (trace.jobs ?? [])
    .filter((job) => !seenJobIds.has(job.id))
    .map((job) => ({
      id: `trace-job-${job.id}`,
      jobId: job.id,
      source: "job",
      timestamp: serviceJobTimestamp(job),
      kind: job.state ?? "service_job",
      title: job.action ?? "Service job",
      message: job.error || job.id,
      serviceName: job.serviceName,
      agentName: job.agentName,
      taskName: job.taskName,
    }));

  return [...activityItems, ...eventItems, ...jobItems].sort(
    (left, right) => new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime(),
  );
}
