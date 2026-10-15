//! Bounded website reads over the same guide and task owner. No ranking,
//! credential work, posting or desktop provider is introduced here.
use super::super::{workflow, JsonRpcError};
use super::{fault, task_context, SiteGuide};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RetrievalGuide {
    container_selector: String,
    item_selector: String,
    id_attribute: String,
    title_selector: String,
    url_selector: String,
    no_results_selector: String,
    next_page_selector: Option<String>,
}
impl RetrievalGuide {
    pub(super) fn validate(&self) -> Result<(), JsonRpcError> {
        if [
            &self.container_selector,
            &self.item_selector,
            &self.title_selector,
            &self.url_selector,
            &self.no_results_selector,
        ]
        .iter()
        .any(|selector| selector.is_empty() || selector.len() > 512)
            || self
                .next_page_selector
                .as_ref()
                .is_some_and(|selector| selector.is_empty() || selector.len() > 512)
            || self.id_attribute.len() > 120
            || !(self.id_attribute == "id" || self.id_attribute.starts_with("data-"))
        {
            return Err(fault("siteguide_retrieval_invalid"));
        }
        Ok(())
    }
}
fn budget(
    params: &Value,
    name: &str,
    default: u64,
    min: u64,
    max: u64,
) -> Result<u64, JsonRpcError> {
    match params.get(name) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .filter(|value| (min..=max).contains(value))
            .ok_or_else(|| fault("retrieval_budget_invalid")),
    }
}
fn execute(
    binding: &Value,
    session: &str,
    action: &str,
    params: Value,
) -> Result<Value, JsonRpcError> {
    let mut command = params;
    command["id"] = json!(uuid::Uuid::new_v4().to_string());
    command["action"] = json!(action);
    command["sessionName"] = binding["sessionName"].clone();
    workflow::execute(command, session)
}

/// Collect only bounded normalized item fields. An empty parser is never proof
/// that the site has no results. Every next-page action rechecks its live link.
pub(super) fn collect(
    guide: &SiteGuide,
    params: &Value,
    task_id: &str,
    session: &str,
) -> Result<Value, JsonRpcError> {
    let retrieval = guide
        .retrieval
        .as_ref()
        .ok_or_else(|| fault("siteguide_retrieval_missing"))?;
    let max_items = budget(params, "maxItems", 50, 1, 200)?;
    let max_pages = budget(params, "maxPages", 1, 1, 10)?;
    let max_scrolls = budget(params, "maxScrolls", 0, 0, 10)?;
    let maximum_age = budget(params, "maxObservationAgeMs", 10000, 1, 60000)?;
    let binding = task_context(task_id)?;
    let descriptor = json!({"origins":guide.allowed_origins,"retrieval":retrieval,"states":guide.states,"maxScan":max_items*2});
    let script = format!(
        r#"(() => {{
        const guide={descriptor}, config=guide.retrieval;
        const visible=element=>element && element.getClientRects().length>0 && getComputedStyle(element).visibility!=='hidden';
        if(!guide.origins.includes(location.origin))return {{state:'origin_mismatch',items:[]}};
        const state=guide.states.find(state=>[...document.querySelectorAll(state.selector)].some(visible));
        if(state)return {{state:state.state,items:[]}};
        const containers=[...document.querySelectorAll(config.containerSelector)].filter(visible);
        if(containers.length!==1)return {{state:'container_changed',items:[]}};
        const nodes=containers[0].querySelectorAll(config.itemSelector),items=[];
        let malformed=0;
        for(let i=0;i<Math.min(nodes.length,guide.maxScan);i++) {{
            const node=nodes[i],id=(node.getAttribute(config.idAttribute)||'').trim(),titles=node.querySelectorAll(config.titleSelector),links=node.querySelectorAll(config.urlSelector);
            const title=titles.length===1 ? titles[0].textContent.trim() : '';
            const href=links.length===1 ? links[0].getAttribute('href') : null;
            let url;try{{url=typeof href==='string' && href.trim() && href.length<=2048 ? new URL(href,location.href):null;}}catch{{url=null;}}
            if(!id||id.length>256||!title||!url||!['http:','https:'].includes(url.protocol)||url.username||url.password){{malformed++;continue;}}
            items.push({{id,title:[...title].slice(0,1000).join(''),url:url.origin+url.pathname}});
        }}
        const empty=[...document.querySelectorAll(config.noResultsSelector)].filter(visible).length===1;
        const next=config.nextPageSelector ? [...document.querySelectorAll(config.nextPageSelector)].filter(visible):[];
        let nextHref=null,nextInvalid=false;
        if(next.length===1){{try{{const href=next[0].getAttribute('href');if(typeof href!=='string'||!href.trim()||href.length>2048)throw Error('link_missing');const link=new URL(href,location.href);
            if(next[0].tagName==='A'&&guide.origins.includes(link.origin)&&!link.username&&!link.password)nextHref=link.href;else nextInvalid=true;
        }}catch{{nextInvalid=true;}}}}else if(next.length>1)nextInvalid=true;
        return {{state:'feed',items,malformed,empty,rawCount:nodes.length,truncated:nodes.length>guide.maxScan,nextHref,nextInvalid,pageUrl:location.href}};
    }})()"#
    );
    let mut items = Vec::<Value>::new();
    let mut ids = BTreeSet::new();
    let mut pages = 1_u64;
    let mut scrolls = 0_u64;
    let mut malformed = 0_u64;
    let mut truncated = false;
    let mut seen_links = BTreeSet::new();
    let mut observations = 0_u64;
    let mut max_elapsed = 0_u128;
    let (status, reason, no_results, effect) = loop {
        let started = Instant::now();
        let observed = match execute(&binding, session, "evaluate", json!({"script":script})) {
            Ok(observed) => observed,
            Err(_) => break ("failed", "observation_failed", false, "no_effect"),
        };
        observations += 1;
        max_elapsed = max_elapsed.max(started.elapsed().as_millis());
        if task_context(task_id)? != binding {
            break (
                "adaptation_required",
                "task_target_changed",
                false,
                "no_effect",
            );
        }
        if started.elapsed().as_millis() > u128::from(maximum_age) {
            break (
                "adaptation_required",
                "observation_stale",
                false,
                "no_effect",
            );
        }
        let data = observed
            .get("result")
            .filter(|value| value.is_object())
            .ok_or_else(|| fault("retrieval_observation_invalid"))?;
        match data["state"].as_str() {
            Some("origin_mismatch") => break ("failed", "origin_mismatch", false, "no_effect"),
            Some("blocked") => break ("failed", "site_blocked", false, "no_effect"),
            Some(
                "login_required" | "identifier_required" | "password_required" | "mfa_required",
            ) => break ("waiting", "login_required", false, "no_effect"),
            Some("feed") => {}
            _ => {
                break (
                    "adaptation_required",
                    "container_changed",
                    false,
                    "no_effect",
                )
            }
        }
        malformed += data["malformed"].as_u64().unwrap_or(0);
        truncated |= data["truncated"] == true;
        for item in data["items"]
            .as_array()
            .ok_or_else(|| fault("retrieval_observation_invalid"))?
        {
            let id = item["id"]
                .as_str()
                .ok_or_else(|| fault("retrieval_item_invalid"))?;
            if ids.contains(id) {
                continue;
            }
            if items.len() >= max_items as usize {
                truncated = true;
                continue;
            }
            ids.insert(id.to_owned());
            items.push(item.clone());
        }
        if data["empty"] == true {
            if data["rawCount"] != 0 {
                break (
                    "adaptation_required",
                    "inconsistent_empty_signal",
                    false,
                    "no_effect",
                );
            }
            if malformed > 0 {
                break (
                    "adaptation_required",
                    "malformed_extraction",
                    false,
                    "no_effect",
                );
            }
            if truncated {
                break ("partial", "budget_exhausted", false, "no_effect");
            }
            break (
                "completed",
                if items.is_empty() {
                    "no_results"
                } else {
                    "posts_collected"
                },
                items.is_empty(),
                "no_effect",
            );
        }
        if data["nextInvalid"] == true {
            break (
                "adaptation_required",
                "pagination_changed",
                false,
                "no_effect",
            );
        }
        if let Some(next) = data["nextHref"].as_str() {
            if pages >= max_pages || items.len() >= max_items as usize {
                break ("partial", "budget_exhausted", false, "no_effect");
            }
            if !seen_links.insert(next.to_owned()) {
                break ("partial", "pagination_cycle", false, "no_effect");
            }
            let selector = serde_json::to_string(&retrieval.next_page_selector)
                .map_err(|_| fault("retrieval_metadata_invalid"))?;
            let expected =
                serde_json::to_string(next).map_err(|_| fault("retrieval_metadata_invalid"))?;
            let page_url = serde_json::to_string(&data["pageUrl"])
                .map_err(|_| fault("retrieval_metadata_invalid"))?;
            let check=format!("(() => {{const links=document.querySelectorAll({selector});return location.href==={page_url}&&links.length===1&&links[0].href==={expected};}})()");
            if execute(&binding, session, "evaluate", json!({"script":check}))?["result"] != true
                || task_context(task_id)? != binding
                || started.elapsed().as_millis() > u128::from(maximum_age)
            {
                break (
                    "adaptation_required",
                    "navigation_plan_changed",
                    false,
                    "no_effect",
                );
            }
            if execute(&binding, session, "navigate", json!({"url":next})).is_err() {
                break ("partial", "navigation_outcome_uncertain", false, "unknown");
            }
            pages += 1;
            continue;
        }
        if scrolls < max_scrolls && items.len() < max_items as usize {
            execute(
                &binding,
                session,
                "evaluate",
                json!({"script":"(async()=>{window.scrollBy(0,600);await new Promise(resolve=>requestAnimationFrame(resolve));return true;})()"}),
            )?;
            scrolls += 1;
            continue;
        }
        if malformed > 0 {
            break (
                "adaptation_required",
                "malformed_extraction",
                false,
                "no_effect",
            );
        }
        if truncated || max_scrolls > 0 {
            break ("partial", "budget_exhausted", false, "no_effect");
        }
        if items.is_empty() {
            break (
                "adaptation_required",
                "empty_extraction_unproven",
                false,
                "no_effect",
            );
        }
        break ("completed", "posts_collected", false, "no_effect");
    };
    let status = if !items.is_empty() && (malformed > 0 || status != "completed") {
        "partial"
    } else {
        status
    };
    let data = json!({"schemaVersion":"agent-browser.routine-result.v1","routine":"collect_posts","guideId":guide.guide_id,
        "taskId":task_id,"profileId":binding["profileId"],"browserId":binding["browserId"],"tabId":binding["tabId"],
        "status":status,"reasonCode":reason,"effectState":effect,"items":items,"noResultsVerified":no_results,
        "pagesUsed":pages,"scrollsUsed":scrolls,"malformedItems":malformed,
        "observations":observations,"maximumObservationElapsedMs":max_elapsed,
        "budgets":{"maxItems":max_items,"maxPages":max_pages,"maxScrolls":max_scrolls,"maxObservationAgeMs":maximum_age}});
    Ok(super::super::tool_response_from_payload(
        "browser_command",
        session,
        json!({}),
        data,
        false,
    ))
}
