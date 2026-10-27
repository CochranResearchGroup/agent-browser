//! Native task control borrows the existing Remote View desktop owner.
use super::*;
use agent_browser_service_model::{BrowserLaunchCustodyStore, RemoteViewTabHandoffTarget};
use serde_json::{json, Value};

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    /// Native perception/control retain the existing process and desktop owner.
    /// Optional checkbox admission explicitly reads an already-qualified CDP
    /// task for origin policy; it never converts or probes a CDP-free task.
    pub(crate) fn native_browser_command(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        command: &Value,
    ) -> Result<Value, String> {
        if self.persistence.load_session_state()? != self.persisted_state
            || self.state.browsers.get(&target.browser.id) != Some(&target.browser)
            || self.state.sessions.get(&target.session.id) != Some(&target.session)
            || self.state.tabs.get(&target.tab.id) != Some(&target.tab)
        {
            return Err("browser_native_target_changed:refresh task".into());
        }
        let original = if let Some(attachment) =
            self.state.app_browser_attachments.get(&target.browser.id)
        {
            attachment.request.process.clone()
        } else {
            BrowserSessionSqliteStore::open_read_only(&self.persistence.runtime_database_path()?)?
                .published_launch_process_identity(&target.browser)
                .map_err(|_| "browser_native_original_identity_unavailable")?
                .ok_or("browser_native_original_identity_missing")?
        };
        self.effects.begin_operation(&self.persisted_state)?;
        let assignment = self.effects.native_handoff_assignment_target(target)?;
        if command.get("imageAttempt").is_some() {
            return self.native_image_browser_command(target, &original, &assignment, command);
        }
        if command.get("nativeAttempt").is_none() {
            return self.native_process_command(&original, &assignment, command);
        }
        // The original is already durable. Refuse unknown origin/policy before
        // changing physical selection, then focus the exact existing task once.
        super::super::browser_task_operations::native_attempt_observation(command)?;
        let origin = qualified_origin(
            &self.persistence,
            &self.persisted_state,
            &mut self.effects,
            target,
            command,
        )?;
        let binding = super::super::browser_task_operations::native_site_policy(command, &origin)?;
        let database = self.persistence.runtime_database_path()?;
        let publication =
            super::super::browser_task_operations::retain_native_site_policy_for_owner(
                &mut BrowserSessionSqliteStore::open(
                    database
                        .parent()
                        .ok_or("browser_session_service_directory_missing")?,
                )?,
                command,
                &binding,
                &self.persisted_state,
            )?;
        publication.check_before(&self.persisted_state)?;
        publication.check_current(&self.persistence.load_session_state()?)?;
        self.refresh_before_ordinary_command()?;
        let observation =
            super::super::browser_task_operations::native_attempt_observation(command)?;
        let focus = json!({"id":format!("{}:focus",command["id"].as_str().ok_or("browser_native_original_operation_invalid")?),
            "action":"view_focus","sessionId":target.session.id,"sessionName":target.session.name,"retainedTaskOnly":true,"noTransportReplay":true});
        super::super::browser_host_policy::check_state(&self.state, &target.session.name, &focus)?;
        super::super::browser_host_control::check_state(&self.state, &target.session.name, &focus)?;
        let native_origin = std::env::var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN")
            .map_err(|_| "browser_native_owner_unconfigured")?;
        let mut http = super::super::remote_view_application_http::RemoteViewApplicationHttp::new(
            &native_origin,
            std::time::Duration::from_secs(10),
        )
        .map_err(|_| "browser_native_owner_configuration_invalid")?;
        let control=http.native_request("control",&json!({"desktopId":assignment.viewing_desktop_id,"expectedGeneration":assignment.viewing_generation,"operation":"status"}))
            .map_err(|_| "browser_native_original_observation_control_unverified")?;
        if control["desktopId"] != observation.desktop_id
            || control["generation"] != observation.generation
            || control["owner"] != "agent"
            || control["epoch"] != observation.native_control_epoch
        {
            return Err("browser_native_original_observation_control_changed".into());
        }
        observation.validate(current_unix_ms())?;
        // Existing focus owner: one exact target, no browser/tab creation or
        // recovery. A changed Native frame later refuses the checkbox effect.
        self.focus_native_policy_publication(target, &publication)?;
        let origin = qualified_origin(
            &self.persistence,
            &self.persisted_state,
            &mut self.effects,
            target,
            command,
        )?;
        super::super::browser_task_operations::check_native_site_policy(command, &origin)?;
        let persistence = &self.persistence;
        let expected = &self.persisted_state;
        let effects = &mut self.effects;
        let mut check = |command: &Value| {
            let origin = qualified_origin(persistence, expected, effects, target, command)?;
            super::super::browser_task_operations::check_native_site_policy(command, &origin)?;
            super::super::browser_task_operations::native_attempt_observation(command)
        };
        run_native_owner(
            persistence,
            expected,
            &original,
            &assignment,
            command,
            Some(&mut check),
            None,
        )
    }

    /// A private launched-process proof can be qualified before publication.
    /// Callers retain the exact launch claim; this grants no additional access.
    pub(crate) fn native_process_command(
        &self,
        original: &agent_browser_service_model::RecordedProcessIdentity,
        assignment: &agent_browser_service_model::RemoteViewApplicationTarget,
        command: &Value,
    ) -> Result<Value, String> {
        run_native_owner(
            &self.persistence,
            &self.persisted_state,
            original,
            assignment,
            command,
            None,
            None,
        )
    }

    pub(super) fn execute_native_task(
        &mut self,
        session_name: &str,
        command: &Value,
    ) -> Result<Option<Value>, String> {
        let id = self
            .exact_task_selector(session_name, command)?
            .ok_or("browser_native_exact_task_required")?;
        let session = self
            .state
            .sessions
            .get(id)
            .cloned()
            .ok_or("browser_native_task_missing")?;
        let tab = session
            .current_tab_id
            .as_ref()
            .and_then(|id| self.state.tabs.get(id))
            .cloned()
            .ok_or("browser_native_tab_missing")?;
        let browser = self
            .state
            .browsers
            .get(&session.browser_id)
            .cloned()
            .ok_or("browser_native_browser_missing")?;
        let target = RemoteViewTabHandoffTarget {
            session,
            tab,
            browser,
        };
        let data = self.native_browser_command(&target, command)?;
        Ok(Some(
            json!({"success":true,"data":data,"browserSession":{"sessionId":target.session.id,"browserId":target.browser.id,"profileId":target.session.profile_id,"tabId":target.tab.id}}),
        ))
    }
}

/// Focus is part of the already-pending Native attempt, after the exact owner
/// policy publication. Current state and target joins must still match it.
impl<P: BrowserSessionPersistence, E: BrowserSessionEffects> BrowserSessionHost<P, E> {
    pub(crate) fn focus_native_policy_publication(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        publication: &super::super::browser_task_operations::NativePolicyPublication,
    ) -> Result<(), String> {
        let current = self.persistence.load_session_state()?;
        publication.check_current(&current)?;
        if current.browsers.get(&target.browser.id) != Some(&target.browser)
            || current.sessions.get(&target.session.id) != Some(&target.session)
            || current.tabs.get(&target.tab.id) != Some(&target.tab)
        {
            return Err("browser_native_target_changed:refresh task".into());
        }
        // Reseed the same adapter only from the exact owner-authored publication;
        // never refresh it from arbitrary current state. Durable activity and
        // retirement journals retain their existing admission/readback fences.
        if self.state != self.persisted_state
            || self.effects.pending_launch_intent().is_some()
            || !self.effects.pending_idle_retirement_ids().is_empty()
        {
            return Err("browser_session_publication_conflict".into());
        }
        self.effects.begin_operation(publication.snapshot())?;
        self.effects
            .focus_browser(&target.browser, Some(&target.tab))
    }
}

/// Current CDP origin is an explicit owner policy observation, never Native
/// perception and never a debugger attachment to a CDP-free browser.
fn qualified_origin<P: BrowserSessionPersistence, E: ManagedBrowserCommandEffects>(
    persistence: &P,
    expected: &BrowserSessionState,
    effects: &mut E,
    target: &RemoteViewTabHandoffTarget,
    parent_command: &Value,
) -> Result<String, String> {
    if persistence.load_session_state()? != *expected {
        return Err("browser_native_target_changed:refresh task".into());
    }
    let mode = BrowserSessionSqliteStore::open_read_only(&persistence.runtime_database_path()?)?
        .inspect_task_mode(&target.session.id)?;
    if mode["mode"] != "cdp" {
        return Err(
            "browser_native_site_origin_unverified:CDP-free tasks require human assistance".into(),
        );
    }
    super::super::browser_task_operations::native_site_mode_preflight(parent_command)?;
    let command = json!({"id":uuid::Uuid::new_v4().to_string(),"action":"url","sessionId":target.session.id,"sessionName":target.session.name,
        "profileId":target.session.profile_id,"retainedTaskOnly":true,"noTransportReplay":true,
        "retainedTaskContext":super::super::browser_task_operations::retained_context(&target.session.id)?});
    super::super::browser_task_operations::check_expected(expected, &command)?;
    super::super::browser_host_policy::check_state(expected, &target.session.name, &command)?;
    super::super::browser_host_control::check_state(expected, &target.session.name, &command)?;
    let reply = effects.execute_command(
        &target.browser,
        &target.tab,
        &target.session.id,
        &target.session.name,
        &command,
    )?;
    if reply["success"] == false || persistence.load_session_state()? != *expected {
        return Err("browser_native_site_origin_unverified".into());
    }
    let data = reply.get("data").unwrap_or(&reply);
    if data["urlObserved"] == false {
        return Err("browser_native_site_origin_unverified".into());
    }
    let url = data["url"]
        .as_str()
        .ok_or("browser_native_site_origin_unverified")?;
    let url = url::Url::parse(url).map_err(|_| "browser_native_site_origin_unverified")?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err("browser_native_site_origin_unverified".into());
    }
    Ok(url.origin().ascii_serialization())
}

type NativeAttemptAdmission<'a> = dyn FnMut(
        &Value,
    ) -> Result<super::super::browser_native_control::NativeChallengeObservation, String>
    + 'a;

type NativeImageAttemptAdmission<'a> = dyn FnMut(&Value) -> Result<super::super::browser_native_image::NativeImageObservation, String>
    + 'a;

fn run_native_owner<'a, P: BrowserSessionPersistence>(
    persistence: &'a P,
    expected: &'a BrowserSessionState,
    original: &agent_browser_service_model::RecordedProcessIdentity,
    assignment: &agent_browser_service_model::RemoteViewApplicationTarget,
    command: &'a Value,
    admission: Option<&'a mut NativeAttemptAdmission<'a>>,
    image_admission: Option<&'a mut NativeImageAttemptAdmission<'a>>,
) -> Result<Value, String> {
    let origin = std::env::var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN")
        .map_err(|_| "browser_native_owner_unconfigured")?;
    let http = super::super::remote_view_application_http::RemoteViewApplicationHttp::new(
        &origin,
        std::time::Duration::from_secs(10),
    )
    .map_err(|_| "browser_native_owner_configuration_invalid")?;
    struct Owner<'a, P> {
        command: &'a Value,
        admission: Option<&'a mut NativeAttemptAdmission<'a>>,
        image_admission: Option<&'a mut NativeImageAttemptAdmission<'a>>,
        http: super::super::remote_view_application_http::RemoteViewApplicationHttp,
        persistence: &'a P,
        expected: &'a BrowserSessionState,
    }
    impl<P: BrowserSessionPersistence> super::super::browser_native_control::NativeOwner
        for Owner<'_, P>
    {
        fn challenge_target(
            &mut self,
            png: &[u8],
            ocr: &Value,
            action: &str,
            region: &Value,
        ) -> Result<Value, String> {
            if self.persistence.load_session_state()? != *self.expected {
                return Err("browser_native_target_changed:refresh task".into());
            }
            super::super::desktop_locator::locate_native_frame(png, ocr, action, region)
        }

        fn native_attempt(
            &mut self,
            command: &Value,
        ) -> Result<super::super::browser_native_control::NativeChallengeObservation, String>
        {
            if self.persistence.load_session_state()? != *self.expected {
                return Err("browser_native_target_changed:refresh task".into());
            }
            let check = self
                .admission
                .as_deref_mut()
                .ok_or("browser_native_site_policy_origin_unverified:human assistance required")?;
            check(command)
        }

        fn native_image_attempt(
            &mut self,
            command: &Value,
        ) -> Result<super::super::browser_native_image::NativeImageObservation, String> {
            if self.persistence.load_session_state()? != *self.expected {
                return Err("browser_native_target_changed:refresh task".into());
            }
            self.image_admission
                .as_deref_mut()
                .ok_or("browser_native_site_policy_origin_unverified:human assistance required")?(
                command,
            )
        }

        fn request(&mut self, operation: &str, body: &Value) -> Result<Value, String> {
            if self.persistence.load_session_state()? != *self.expected {
                return Err("browser_native_target_changed:refresh task".into());
            }
            if operation == "input" && self.command.get("imageAttempt").is_some() {
                self.image_admission
                    .as_deref_mut()
                    .ok_or("browser_native_site_policy_origin_unverified")?(
                    self.command
                )?;
                if self.persistence.load_session_state()? != *self.expected {
                    return Err("browser_native_target_changed:refresh task".into());
                }
            }
            if operation == "input" && self.command.get("nativeAttempt").is_some() {
                let check = self
                    .admission
                    .as_deref_mut()
                    .ok_or("browser_native_site_policy_origin_unverified")?;
                check(self.command)?;
                if self.persistence.load_session_state()? != *self.expected {
                    return Err("browser_native_target_changed:refresh task".into());
                }
            }
            self.http.native_request(operation, body)
                    .map_err(|error| format!("browser_native_owner_request:{error:?}:observe before retry; input may have been delivered"))
        }
    }
    let mut owner = Owner {
        http,
        persistence,
        expected,
        command,
        admission,
        image_admission,
    };
    super::super::browser_native_control::execute(original, assignment, command, &mut owner)
}

#[path = "browser_native_host_image.rs"]
mod image;
