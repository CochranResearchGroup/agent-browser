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
