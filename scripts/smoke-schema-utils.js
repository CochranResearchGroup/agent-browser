import { readFileSync } from 'node:fs';

import { assert } from './smoke-utils.js';

export function loadServiceRecordSchema(relativePath) {
  return JSON.parse(readFileSync(new URL(relativePath, import.meta.url), 'utf8'));
}

export function parseMcpJsonResource(result, uri, label) {
  const content = result.contents?.[0];
  assert(
    content?.mimeType === 'application/json',
    `${label} content MIME mismatch: ${JSON.stringify(content)}`,
  );
  assert(content?.uri === uri, `${label} content URI mismatch: ${JSON.stringify(content)}`);
  assert(typeof content.text === 'string', `${label} content missing JSON text`);
  return JSON.parse(content.text);
}

function assertRequiredFields(record, schema, label) {
  assert(record && typeof record === 'object', `${label} is not an object: ${JSON.stringify(record)}`);
  for (const field of schema.required) {
    assert(Object.hasOwn(record, field), `${label} missing schema field ${field}: ${JSON.stringify(record)}`);
  }
}

function assertNoSnakeCaseFields(record, fields, label) {
  for (const field of fields) {
    assert(!Object.hasOwn(record, field), `${label} leaked snake_case field ${field}`);
  }
}

function schemaEnum(schema, property) {
  return schema.properties[property].enum;
}

function schemaOneOfEnum(schema, property) {
  return schema.properties[property].oneOf?.find((entry) => Array.isArray(entry.enum))?.enum;
}

export function assertServiceCollectionSchemaRecord(record, schema, label, options = {}) {
  const {
    arrayFields = [],
    booleanFields = [],
    enumFields = [],
    nullableEnumFields = [],
    objectFields = [],
    snakeCaseFields = [],
  } = options;
  assertRequiredFields(record, schema, label);
  assertNoSnakeCaseFields(record, snakeCaseFields, label);

  for (const field of enumFields) {
    assert(
      schemaEnum(schema, field).includes(record[field]),
      `${label} ${field} is outside schema enum: ${JSON.stringify(record)}`,
    );
  }
  for (const field of nullableEnumFields) {
    const values = schemaOneOfEnum(schema, field);
    assert(Array.isArray(values), `${label} ${field} schema missing nullable enum`);
    assert(
      record[field] === null || values.includes(record[field]),
      `${label} ${field} is outside schema enum: ${JSON.stringify(record)}`,
    );
  }
  for (const field of arrayFields) {
    assert(Array.isArray(record[field]), `${label} missing ${field} array: ${JSON.stringify(record)}`);
  }
  for (const field of booleanFields) {
    assert(typeof record[field] === 'boolean', `${label} missing ${field} boolean: ${JSON.stringify(record)}`);
  }
  for (const field of objectFields) {
    assert(
      record[field] && typeof record[field] === 'object' && !Array.isArray(record[field]),
      `${label} missing ${field} object: ${JSON.stringify(record)}`,
    );
  }
}

export function assertServiceTraceSummarySchemaRecord(summary, schema, label) {
  assertRequiredFields(summary, schema, label);
  assertNoSnakeCaseFields(
    summary,
    ['context_count', 'has_trace_context', 'naming_warning_count'],
    label,
  );
  assert(Array.isArray(summary.contexts), `${label} missing contexts array: ${JSON.stringify(summary)}`);
  assert(
    summary.contextCount === summary.contexts.length,
    `${label} contextCount mismatch: ${JSON.stringify(summary)}`,
  );
  assert(typeof summary.hasTraceContext === 'boolean', `${label} missing hasTraceContext boolean`);
  assert(Number.isInteger(summary.namingWarningCount), `${label} missing namingWarningCount integer`);

  const contextSchema = schema.properties.contexts.items;
  for (const [index, context] of summary.contexts.entries()) {
    const contextLabel = `${label} contexts[${index}]`;
    assertRequiredFields(context, contextSchema, contextLabel);
    assertNoSnakeCaseFields(
      context,
      [
        'service_name',
        'agent_name',
        'task_name',
        'browser_id',
        'profile_id',
        'session_id',
        'naming_warnings',
        'has_naming_warning',
        'event_count',
        'job_count',
        'incident_count',
        'activity_count',
        'latest_timestamp',
      ],
      contextLabel,
    );
    assert(Array.isArray(context.namingWarnings), `${contextLabel} missing namingWarnings array`);
    for (const warning of context.namingWarnings) {
      assert(
        contextSchema.properties.namingWarnings.items.enum.includes(warning),
        `${contextLabel} warning is outside schema enum: ${warning}`,
      );
    }
    for (const field of ['eventCount', 'jobCount', 'incidentCount', 'activityCount']) {
      assert(Number.isInteger(context[field]), `${contextLabel} missing ${field} integer`);
    }
    assert(typeof context.hasNamingWarning === 'boolean', `${contextLabel} missing hasNamingWarning boolean`);
  }
}

export function assertServiceTraceActivitySchemaRecord(activity, schema, label) {
  assertRequiredFields(activity, schema, label);
  assertNoSnakeCaseFields(
    activity,
    [
      'event_id',
      'job_id',
      'browser_id',
      'profile_id',
      'session_id',
      'service_name',
      'agent_name',
      'task_name',
      'job_state',
      'job_action',
    ],
    label,
  );
  assert(schemaEnum(schema, 'source').includes(activity.source), `${label} source is outside schema enum`);
  assert(schemaEnum(schema, 'kind').includes(activity.kind), `${label} kind is outside schema enum`);
  if (Object.hasOwn(activity, 'jobState')) {
    assert(schemaEnum(schema, 'jobState').includes(activity.jobState), `${label} jobState is outside schema enum`);
  }
}

export function assertServiceTraceResponseSchemaRecord(trace, schema, label) {
  assertRequiredFields(trace, schema, label);
  assertNoSnakeCaseFields(trace, ['browser_id', 'profile_id', 'session_id', 'service_name', 'agent_name', 'task_name'], label);
  const filtersSchema = schema.properties.filters;
  assertRequiredFields(trace.filters, filtersSchema, `${label} filters`);
  assertNoSnakeCaseFields(
    trace.filters,
    ['browser_id', 'profile_id', 'session_id', 'service_name', 'agent_name', 'task_name'],
    `${label} filters`,
  );
  for (const field of ['events', 'jobs', 'incidents', 'activity']) {
    assert(Array.isArray(trace[field]), `${label} missing ${field} array`);
    assert(Number.isInteger(trace.counts?.[field]), `${label} missing counts.${field} integer`);
    assert(trace.counts[field] === trace[field].length, `${label} counts.${field} does not match ${field} length`);
    assert(Number.isInteger(trace.matched?.[field]), `${label} missing matched.${field} integer`);
  }
  for (const field of ['events', 'jobs', 'incidents']) {
    assert(Number.isInteger(trace.total?.[field]), `${label} missing total.${field} integer`);
  }
  assert(Number.isInteger(trace.filters.limit), `${label} missing filters.limit integer`);
  assert(trace.summary && typeof trace.summary === 'object', `${label} missing summary object`);
}

export function assertServiceIncidentActivityResponseSchemaRecord(response, schema, label) {
  assertRequiredFields(response, schema, label);
  assert(response.incident && typeof response.incident === 'object', `${label} missing incident object`);
  assert(Array.isArray(response.activity), `${label} missing activity array`);
  assert(Number.isInteger(response.count), `${label} missing count integer`);
  assert(response.count === response.activity.length, `${label} count does not match activity length`);
}

export function assertServiceIncidentsResponseSchemaRecord(response, schema, label) {
  assertRequiredFields(response, schema, label);
  assertNoSnakeCaseFields(response, ['handling_state', 'browser_id', 'profile_id', 'session_id'], label);
  assert(Array.isArray(response.incidents), `${label} missing incidents array`);
  assert(Number.isInteger(response.count), `${label} missing count integer`);
  assert(Number.isInteger(response.matched), `${label} missing matched integer`);
  assert(Number.isInteger(response.total), `${label} missing total integer`);
  assert(response.count === response.incidents.length, `${label} count does not match incidents length`);
  if (Object.hasOwn(response, 'filters')) {
    assertRequiredFields(response.filters, schema.properties.filters, `${label} filters`);
    assertNoSnakeCaseFields(
      response.filters,
      [
        'handling_state',
        'browser_id',
        'profile_id',
        'session_id',
        'service_name',
        'agent_name',
        'task_name',
      ],
      `${label} filters`,
    );
    assert(Number.isInteger(response.filters.limit), `${label} filters missing limit integer`);
  }
  if (Object.hasOwn(response, 'incident')) {
    assert(response.incident && typeof response.incident === 'object', `${label} missing incident object`);
  }
  if (Object.hasOwn(response, 'events')) {
    assert(Array.isArray(response.events), `${label} events is not an array`);
  }
  if (Object.hasOwn(response, 'jobs')) {
    assert(Array.isArray(response.jobs), `${label} jobs is not an array`);
  }
}

export function assertServiceEventsResponseSchemaRecord(response, schema, label) {
  assertRequiredFields(response, schema, label);
  assertNoSnakeCaseFields(response, ['browser_id', 'profile_id', 'session_id', 'service_name', 'agent_name', 'task_name'], label);
  assert(Array.isArray(response.events), `${label} missing events array`);
  assert(Number.isInteger(response.count), `${label} missing count integer`);
  assert(Number.isInteger(response.matched), `${label} missing matched integer`);
  assert(Number.isInteger(response.total), `${label} missing total integer`);
  assert(response.count === response.events.length, `${label} count does not match events length`);
}

export function assertServiceJobsResponseSchemaRecord(response, schema, label) {
  assertRequiredFields(response, schema, label);
  assertNoSnakeCaseFields(response, ['service_name', 'agent_name', 'task_name'], label);
  assert(Array.isArray(response.jobs), `${label} missing jobs array`);
  assert(Number.isInteger(response.count), `${label} missing count integer`);
  assert(Number.isInteger(response.matched), `${label} missing matched integer`);
  assert(Number.isInteger(response.total), `${label} missing total integer`);
  assert(response.count === response.jobs.length, `${label} count does not match jobs length`);
  if (Object.hasOwn(response, 'job')) {
    assert(response.job && typeof response.job === 'object', `${label} missing job object`);
    assert(
      response.jobs.some((job) => job.id === response.job.id),
      `${label} detail job is not present in jobs array`,
    );
  }
}

export function assertServiceSitePolicyUpsertResponseSchemaRecord(response, schema, label) {
  assertRequiredFields(response, schema, label);
  assertNoSnakeCaseFields(response, ['site_policy'], label);
  assert(typeof response.id === 'string', `${label} missing id string`);
  assert(response.upserted === true, `${label} upserted should be true: ${JSON.stringify(response)}`);
  assert(response.sitePolicy && typeof response.sitePolicy === 'object', `${label} missing sitePolicy object`);
}

export function assertServiceSitePolicyDeleteResponseSchemaRecord(response, schema, label) {
  assertRequiredFields(response, schema, label);
  assertNoSnakeCaseFields(response, ['site_policy'], label);
  assert(typeof response.id === 'string', `${label} missing id string`);
  assert(typeof response.deleted === 'boolean', `${label} missing deleted boolean`);
  assert(
    response.sitePolicy === null || (response.sitePolicy && typeof response.sitePolicy === 'object'),
    `${label} sitePolicy should be object or null`,
  );
}

export function assertServiceProviderUpsertResponseSchemaRecord(response, schema, label) {
  assertRequiredFields(response, schema, label);
  assert(typeof response.id === 'string', `${label} missing id string`);
  assert(response.upserted === true, `${label} upserted should be true: ${JSON.stringify(response)}`);
  assert(response.provider && typeof response.provider === 'object', `${label} missing provider object`);
}

export function assertServiceProviderDeleteResponseSchemaRecord(response, schema, label) {
  assertRequiredFields(response, schema, label);
  assert(typeof response.id === 'string', `${label} missing id string`);
  assert(typeof response.deleted === 'boolean', `${label} missing deleted boolean`);
  assert(
    response.provider === null || (response.provider && typeof response.provider === 'object'),
    `${label} provider should be object or null`,
  );
}

export function assertServiceJobSchemaRecord(job, schema, label) {
  assertRequiredFields(job, schema, label);
  assertNoSnakeCaseFields(
    job,
    [
      'service_name',
      'agent_name',
      'task_name',
      'naming_warnings',
      'has_naming_warning',
      'submitted_at',
      'started_at',
      'completed_at',
      'timeout_ms',
    ],
    label,
  );
  assert(schemaEnum(schema, 'state').includes(job.state), `${label} state is outside schema enum`);
  assert(schemaEnum(schema, 'priority').includes(job.priority), `${label} priority is outside schema enum`);
  assert(Array.isArray(job.namingWarnings), `${label} missing namingWarnings array`);
  for (const warning of job.namingWarnings) {
    assert(
      schema.properties.namingWarnings.items.enum.includes(warning),
      `${label} warning is outside schema enum: ${warning}`,
    );
  }
  assert(typeof job.hasNamingWarning === 'boolean', `${label} missing hasNamingWarning boolean`);
}

export function assertServiceIncidentSchemaRecord(incident, schema, label) {
  assertRequiredFields(incident, schema, `${label} incident`);
  assertNoSnakeCaseFields(
    incident,
    [
      'browser_id',
      'recommended_action',
      'acknowledged_at',
      'acknowledged_by',
      'acknowledgement_note',
      'resolved_at',
      'resolved_by',
      'resolution_note',
      'latest_timestamp',
      'latest_message',
      'latest_kind',
      'current_health',
      'event_ids',
      'job_ids',
    ],
    `${label} incident`,
  );
  assert(schemaEnum(schema, 'state').includes(incident.state), `${label} incident state is outside schema enum`);
  assert(
    schemaEnum(schema, 'severity').includes(incident.severity),
    `${label} incident severity is outside schema enum`,
  );
  assert(
    schemaEnum(schema, 'escalation').includes(incident.escalation),
    `${label} incident escalation is outside schema enum`,
  );
  const currentHealthEnum = schema.properties.currentHealth.oneOf[0].enum;
  assert(
    incident.currentHealth === null || currentHealthEnum.includes(incident.currentHealth),
    `${label} incident currentHealth is outside schema enum: ${JSON.stringify(incident)}`,
  );
  assert(Array.isArray(incident.eventIds), `${label} incident missing eventIds array`);
  assert(Array.isArray(incident.jobIds), `${label} incident missing jobIds array`);
}

export function assertServiceEventSchemaRecord(event, schema, label) {
  assertRequiredFields(event, schema, label);
  assertNoSnakeCaseFields(
    event,
    [
      'browser_id',
      'profile_id',
      'session_id',
      'service_name',
      'agent_name',
      'task_name',
      'previous_health',
      'current_health',
    ],
    label,
  );
  assert(schemaEnum(schema, 'kind').includes(event.kind), `${label} kind is outside schema enum`);
  const healthEnum = schema.properties.currentHealth.oneOf[0].enum;
  assert(
    event.previousHealth === null || healthEnum.includes(event.previousHealth),
    `${label} previousHealth is outside schema enum: ${JSON.stringify(event)}`,
  );
  assert(
    event.currentHealth === null || healthEnum.includes(event.currentHealth),
    `${label} currentHealth is outside schema enum: ${JSON.stringify(event)}`,
  );
}
