use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) const CONTEXTUAL_CHAT_PROVIDER_ID: &str = "codex-app-server";
pub(crate) const SELECTED_WORKSPACE_CHAT_PACKET_VERSION: &str = "selected-workspace-chat.v1";
pub(crate) const CODEX_WORKSPACE_OBSERVATION_VERSION: &str = "codex-workspace-observation.v1";

const SENSITIVE_MARKERS: &[&str] = &[
    "authorization",
    "bearer ",
    "cookie=",
    "password",
    "secret=",
    "secret:",
    "api_key",
    "data:image/",
    "localStorage",
    "sessionStorage",
];

pub(crate) fn validate_packet(packet: &Value) -> Result<(), String> {
    if packet.get("version").and_then(Value::as_str) != Some(SELECTED_WORKSPACE_CHAT_PACKET_VERSION)
    {
        return Err("Unsupported selected workspace chat packet version.".to_string());
    }
    if packet.get("provider").and_then(Value::as_str) != Some(CONTEXTUAL_CHAT_PROVIDER_ID) {
        return Err(
            "Selected workspace chat packet provider must be codex-app-server.".to_string(),
        );
    }
    if packet
        .pointer("/redaction/secretsOmitted")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err("Selected workspace chat packet must omit secrets.".to_string());
    }
    reject_sensitive_markers(packet, "Selected workspace chat packet")?;
    Ok(())
}

pub(crate) fn validate_observation(
    observation: &Value,
    packet: &Value,
    run_id: &str,
) -> Result<(), String> {
    if observation.get("version").and_then(Value::as_str)
        != Some(CODEX_WORKSPACE_OBSERVATION_VERSION)
    {
        return Err("Observation has unsupported version.".to_string());
    }
    if observation.get("provider").and_then(Value::as_str) != Some(CONTEXTUAL_CHAT_PROVIDER_ID) {
        return Err("Observation provider must be codex-app-server.".to_string());
    }
    if observation.get("runId").and_then(Value::as_str) != Some(run_id) {
        return Err("Observation run id does not match the host run id.".to_string());
    }
    let packet_workspace_id = packet.pointer("/workspace/id").and_then(Value::as_str);
    let observation_workspace_id = observation.get("workspaceId").and_then(Value::as_str);
    if packet_workspace_id.is_some() && observation_workspace_id != packet_workspace_id {
        return Err(
            "Observation workspace id does not match the selected workspace packet.".to_string(),
        );
    }
    for key in [
        "blockers",
        "risks",
        "suggestedNextInspections",
        "unsupportedActions",
    ] {
        if !observation.get(key).is_some_and(Value::is_array) {
            return Err(format!("Observation `{key}` must be an array."));
        }
    }
    validate_evidence_references(observation, packet)?;
    reject_sensitive_markers(observation, "Observation")?;
    Ok(())
}

pub(crate) fn reject_sensitive_markers(value: &Value, label: &str) -> Result<(), String> {
    let mut strings = Vec::new();
    collect_string_values(value, &mut strings);
    let lower = strings.join("\n").to_lowercase();
    for marker in SENSITIVE_MARKERS {
        if lower.contains(&marker.to_lowercase()) {
            return Err(format!(
                "{label} contains forbidden sensitive marker: {marker}"
            ));
        }
    }
    Ok(())
}

fn collect_string_values<'a>(value: &'a Value, output: &mut Vec<&'a str>) {
    match value {
        Value::String(text) => output.push(text),
        Value::Array(items) => {
            for item in items {
                collect_string_values(item, output);
            }
        }
        Value::Object(map) => {
            for nested in map.values() {
                collect_string_values(nested, output);
            }
        }
        _ => {}
    }
}

pub(crate) fn observation_output_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "version",
            "provider",
            "runId",
            "threadId",
            "createdAt",
            "workspaceId",
            "summary",
            "detectedState",
            "blockers",
            "risks",
            "suggestedNextInspections",
            "unsupportedActions",
            "confidence"
        ],
        "properties": {
            "version": { "type": "string", "const": CODEX_WORKSPACE_OBSERVATION_VERSION },
            "provider": { "type": "string", "const": CONTEXTUAL_CHAT_PROVIDER_ID },
            "runId": { "type": "string" },
            "threadId": { "type": ["string", "null"] },
            "createdAt": { "type": "string" },
            "workspaceId": { "type": ["string", "null"] },
            "summary": { "type": "string" },
            "detectedState": { "type": "string" },
            "blockers": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["severity", "summary", "evidenceIds"],
                    "properties": {
                        "severity": { "type": "string", "enum": ["info", "warning", "blocked"] },
                        "summary": { "type": "string" },
                        "evidenceIds": { "type": "array", "items": { "type": "string" } }
                    }
                }
            },
            "risks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["summary", "evidenceIds"],
                    "properties": {
                        "summary": { "type": "string" },
                        "evidenceIds": { "type": "array", "items": { "type": "string" } }
                    }
                }
            },
            "suggestedNextInspections": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["label", "reason", "evidenceIds"],
                    "properties": {
                        "label": { "type": "string" },
                        "reason": { "type": "string" },
                        "evidenceIds": { "type": "array", "items": { "type": "string" } }
                    }
                }
            },
            "unsupportedActions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["label", "reason"],
                    "properties": {
                        "label": { "type": "string" },
                        "reason": { "type": "string" }
                    }
                }
            },
            "confidence": { "type": "string", "enum": ["low", "medium", "high"] }
        }
    })
}

pub(crate) fn operator_guidance_output_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "summary",
            "targetAssessment",
            "recommendedActions",
            "risks",
            "confirmationRequired",
            "confidence"
        ],
        "properties": {
            "summary": { "type": "string" },
            "targetAssessment": { "type": "string" },
            "recommendedActions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["label", "reason", "toolGroup", "requiresConfirmation"],
                    "properties": {
                        "label": { "type": "string" },
                        "reason": { "type": "string" },
                        "toolGroup": { "type": "string" },
                        "requiresConfirmation": { "type": "boolean" }
                    }
                }
            },
            "risks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["summary", "severity"],
                    "properties": {
                        "summary": { "type": "string" },
                        "severity": { "type": "string", "enum": ["info", "warning", "blocked"] }
                    }
                }
            },
            "confirmationRequired": { "type": "boolean" },
            "confidence": { "type": "string", "enum": ["low", "medium", "high"] }
        }
    })
}

pub(crate) fn validate_operator_guidance(guidance: &Value) -> Result<(), String> {
    for key in ["summary", "targetAssessment", "confidence"] {
        if !guidance.get(key).is_some_and(Value::is_string) {
            return Err(format!("Operator guidance `{key}` must be a string."));
        }
    }
    for key in ["recommendedActions", "risks"] {
        if !guidance.get(key).is_some_and(Value::is_array) {
            return Err(format!("Operator guidance `{key}` must be an array."));
        }
    }
    if !guidance
        .get("confirmationRequired")
        .is_some_and(Value::is_boolean)
    {
        return Err("Operator guidance `confirmationRequired` must be a boolean.".to_string());
    }
    reject_sensitive_markers(guidance, "Operator guidance")?;
    Ok(())
}

fn validate_evidence_references(observation: &Value, packet: &Value) -> Result<(), String> {
    let evidence_ids = packet
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|evidence| evidence.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if evidence_ids.is_empty() {
        return Err("Selected workspace packet has no evidence ids.".to_string());
    }
    for pointer in ["/blockers", "/risks", "/suggestedNextInspections"] {
        for item in observation
            .pointer(pointer)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for evidence_id in item
                .get("evidenceIds")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !evidence_ids.contains(evidence_id) {
                    return Err(format!(
                        "Observation references evidence id not present in the packet: {evidence_id}"
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_sensitive_marker() {
        let value = serde_json::json!({"summary": "Authorization: Bearer token"});
        assert!(reject_sensitive_markers(&value, "test").is_err());
    }
}
