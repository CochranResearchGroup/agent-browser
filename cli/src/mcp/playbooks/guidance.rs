//! Optional login assistance from installed SiteGuides. This module observes
//! predicates only; it never navigates, selects an account or releases credentials.
use super::*;

/// Source-declared knowledge is distinct from independently qualified execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LoginGuidance {
    version: String,
    source: String,
    validation_class: String,
    evidence_date: Option<String>,
    #[serde(default)]
    limitations: Vec<String>,
    #[serde(default)]
    provider_branches: Vec<ProviderBranch>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProviderBranch {
    branch_id: String,
    starting_origin: String,
    provider_origin: String,
    return_origin: String,
    selector: String,
}

impl LoginGuidance {
    pub(super) fn validate(&self, guide: &SiteGuide) -> Result<(), JsonRpcError> {
        if self.version.is_empty() || self.version.len() > 80
            || self.source.is_empty() || self.source.len() > 160
            || !matches!(self.validation_class.as_str(), "authored" | "offline_validated" | "observed_workflow" | "fresh_repeat")
            || self.evidence_date.as_ref().is_some_and(|date| date.len() != 10 || !date.bytes().enumerate().all(|(index, byte)| if index == 4 || index == 7 { byte == b'-' } else { byte.is_ascii_digit() }))
            || self.limitations.len() > 8 || self.limitations.iter().any(|value| value.len() > 160)
            || self.provider_branches.len() > 4
        {
            return Err(fault("siteguide_login_guidance_invalid"));
        }
        let mut ids = std::collections::BTreeSet::new();
        for branch in &self.provider_branches {
            let provider = url::Url::parse(&branch.provider_origin).map_err(|_| fault("siteguide_provider_origin_invalid"))?;
            if !valid_guide_id(&branch.branch_id) || !ids.insert(&branch.branch_id)
                || !guide.allowed_origins.contains(&branch.starting_origin)
                || !guide.allowed_origins.contains(&branch.return_origin)
                || provider.scheme() != "https" || provider.origin().ascii_serialization() != branch.provider_origin
                || branch.selector.is_empty() || branch.selector.len() > 512
            {
                return Err(fault("siteguide_provider_branch_invalid"));
            }
        }
        Ok(())
    }
}

fn summary(guide: &SiteGuide) -> Value {
    let metadata = guide.login_guidance.as_ref();
    json!({"guideId":guide.guide_id,"version":metadata.map(|m|m.version.as_str()).unwrap_or(&guide.schema_version),
        "allowedOrigins":guide.allowed_origins,
        "qualification":{"source":metadata.map(|m|m.source.as_str()).unwrap_or("installed_starter_template"),
            "declaredClass":metadata.map(|m|m.validation_class.as_str()).unwrap_or("authored"),
            "evidenceDate":metadata.and_then(|m|m.evidence_date.as_deref()),
            "liveExecutionProven":false,"freshness":"not_independently_verified",
            "limitations":metadata.map(|m|m.limitations.clone()).unwrap_or_else(||vec!["Starter selectors require site-specific validation.".into()])},
        "credentialUseAuthorized":false})
}

/// Read-only metadata discovery is available before authentication to the site.
pub(super) fn summaries() -> Result<Vec<Value>, JsonRpcError> {
    guide_names()?.iter().map(|name|load_guide(name).map(|guide|summary(&guide))).collect()
}

/// Enrich only a successful public addressed snapshot. Failure of optional
/// assistance leaves the original observation available without retrying input.
pub(in crate::mcp) fn enrich_snapshot(task_id: &str, session: &str, response: &mut Value) {
    let Some(text) = response.pointer("/content/0/text").and_then(Value::as_str) else { return; };
    let Ok(mut payload) = serde_json::from_str::<Value>(text) else { return; };
    if payload["success"] != true || !payload["data"].is_object() { return; }
    match observe(task_id, session) {
        Ok(Some(hint)) => { payload["data"]["loginGuidance"] = hint; }
        Ok(None) => return,
        Err(_) => {
            payload["data"]["loginGuidance"] = json!({"status":"unavailable","taskId":task_id,
                "reasonCode":"login_guidance_unavailable","credentialUseAuthorized":false,
                "nextOperation":{"tool":"browser_discover","arguments":{"category":"playbooks"}},
                "recovery":"Keep this task. Use ordinary observation and supported Auth guidance; do not replay uncertain input."});
        }
    }
    if let Ok(text) = serde_json::to_string(&payload) { response["content"][0]["text"] = json!(text); }
}

/// Observation failure retains the original error and points to same-task
/// readback. It never declares a lost browser or authorizes replacement input.
pub(in crate::mcp) fn observation_recovery(task_id: &str, error: &mut JsonRpcError) {
    if error.data.is_none() { error.data = Some(json!({})); }
    if let Some(data) = error.data.as_mut().and_then(Value::as_object_mut) {
        data.insert("loginRecovery".into(), json!({"taskId":task_id,"classification":"observation_unavailable",
            "browserLost":false,"effectOutcome":"unchanged_by_hint","replayAllowed":false,
            "nextOperation":{"tool":"browser_continue","arguments":{"taskId":task_id}},
            "next":"Inspect this retained task and original operations. A transport/rendering failure is not an authentication denial; do not relaunch or repeat uncertain input."}));
    }
}

fn observe(task_id: &str, session: &str) -> Result<Option<Value>, JsonRpcError> {
    let guides = guide_names()?.iter().map(|name|load_guide(name)).collect::<Result<Vec<_>,_>>()?;
    let descriptors = guides.iter().filter(|guide|guide.authenticated_selector.is_some() && !guide.states.is_empty()).map(|guide| {
        let branches = guide.login_guidance.as_ref().map(|m|m.provider_branches.iter().map(|branch|json!({"branchId":branch.branch_id,"startingOrigin":branch.starting_origin,"selector":branch.selector})).collect::<Vec<_>>()).unwrap_or_default();
        json!({"guideId":guide.guide_id,"origins":guide.allowed_origins,"authenticated":guide.authenticated_selector,
            "states":guide.states,"branches":branches})
    }).collect::<Vec<_>>();
    if descriptors.is_empty() { return Ok(None); }
    let binding = task_context(task_id)?;
    // Only installed descriptors enter the script. Website strings are never
    // parsed as instructions, guide metadata or credential-use authority.
    let script = format!(r#"(() => {{
        const guides={}; const origin=location.origin;
        const visible=selector=>{{try{{return [...document.querySelectorAll(selector)].some(element=>element.getClientRects().length>0 && getComputedStyle(element).visibility!=='hidden');}}catch{{return false;}}}};
        const matches=[];
        for(const guide of guides){{
            if(!guide.origins.includes(origin) || visible(guide.authenticated))continue;
            const state=guide.states.find(item=>visible(item.selector));
            if(!state || !['identifier_required','password_required','mfa_required','login_required'].includes(state.state))continue;
            matches.push({{guideId:guide.guideId,state:state.state,branches:guide.branches.map(branch=>({{branchId:branch.branchId,observed:branch.startingOrigin===origin && visible(branch.selector)}}))}});
        }}
        return {{origin,matches}};
    }})()"#, serde_json::to_string(&descriptors).map_err(|_|fault("siteguide_metadata_invalid"))?);
    let observed = super::super::workflow::execute(json!({"id":uuid::Uuid::new_v4().to_string(),"action":"evaluate",
        "script":script,"sessionName":binding["sessionName"],"sessionId":task_id}), session)?;
    if task_context(task_id)? != binding { return Err(fault("login_guidance_task_changed")); }
    let evidence = &observed["result"];
    let origin = evidence["origin"].as_str().ok_or_else(||fault("login_guidance_observation_invalid"))?;
    let matches = evidence["matches"].as_array().ok_or_else(||fault("login_guidance_observation_invalid"))?;
    if matches.is_empty() { return Ok(None); }
    let mut candidates = Vec::new();
    for observed in matches.iter().take(4) {
        let Some(guide) = guides.iter().find(|guide|observed["guideId"] == guide.guide_id && guide.allowed_origins.iter().any(|allowed|allowed==origin)) else { continue; };
        let mut candidate = summary(guide);
        candidate["observedState"] = observed["state"].clone();
        candidate["providerBranches"] = json!(guide.login_guidance.as_ref().map(|metadata|metadata.provider_branches.iter().map(|branch|json!({
            "branchId":branch.branch_id,"startingOrigin":branch.starting_origin,"providerOrigin":branch.provider_origin,"returnOrigin":branch.return_origin,
            "observed":observed["branches"].as_array().is_some_and(|rows|rows.iter().any(|row|row["branchId"]==branch.branch_id && row["observed"]==true)),
            "credentialUseAuthorized":false})).collect::<Vec<_>>()).unwrap_or_default());
        candidate["nextOperation"] = json!({"tool":"browser_command","taskId":task_id,"action":"playbook","params":{"operation":"inspect_guide","guideId":guide.guide_id}});
        candidates.push(candidate);
    }
    if candidates.is_empty() { return Ok(None); }
    Ok(Some(json!({"status":if candidates.len()>1 {"ambiguous"} else {"available"},
        "taskId":task_id,"profileId":binding["profileId"],"browserId":binding["browserId"],"tabId":binding["tabId"],
        "origin":origin,"evidence":"current_installed_selector_observation","credentialUseAuthorized":false,
        "candidates":candidates,"truncated":matches.len()>4,
        "pathwayOrder":["browser_saved_credentials","lastpass_if_applicable","auth_receipts"],
        "next":"Inspect applicable guidance. Use Native capture/input for browser selectors and LastPass; verify intended website identity independently. Multiple matches require context, never first-match account selection."})))
}
