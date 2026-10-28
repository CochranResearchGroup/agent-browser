//! Static image input consumes the same exact owner-publication/focus protocol
//! as checkbox input. It creates no route, browser, viewer or controller owner.
use super::*;
use crate::native::browser_task_operations as operations;

impl<P: BrowserSessionPersistence, E: BrowserSessionEffects + ManagedBrowserCommandEffects>
    BrowserSessionHost<P, E>
{
    pub(super) fn native_image_browser_command(
        &mut self,
        target: &RemoteViewTabHandoffTarget,
        original: &agent_browser_service_model::RecordedProcessIdentity,
        assignment: &agent_browser_service_model::RemoteViewApplicationTarget,
        command: &Value,
    ) -> Result<Value, String> {
        operations::native_image_attempt_observation(command)?;
        let origin = qualified_origin(
            &self.persistence,
            &self.persisted_state,
            &mut self.effects,
            target,
            command,
        )?;
        let binding = operations::native_site_policy(command, &origin)?;
        let database = self.persistence.runtime_database_path()?;
        let publication = operations::retain_native_site_policy_for_owner(
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
        let observation = operations::native_image_attempt_observation(command)?;
        let configured = std::env::var("AGENT_BROWSER_REMOTE_VIEW_ORIGIN")
            .map_err(|_| "browser_native_owner_unconfigured")?;
        let mut http = crate::native::remote_view_application_http::RemoteViewApplicationHttp::new(
            &configured,
            std::time::Duration::from_secs(10),
        )
        .map_err(|_| "browser_native_owner_configuration_invalid")?;
        let control=http.native_request("control",&json!({"desktopId":assignment.viewing_desktop_id,"expectedGeneration":assignment.viewing_generation,"operation":"status"})).map_err(|_|"browser_native_original_observation_control_unverified")?;
        if control["desktopId"] != observation.desktop_id
            || control["generation"] != observation.generation
            || control["owner"] != "agent"
            || control["epoch"] != observation.native_control_epoch
        {
            return Err("browser_native_original_observation_control_changed".into());
        }
        observation.validate_current()?;
        self.focus_native_policy_publication(target, &publication)?;
        let origin = qualified_origin(
            &self.persistence,
            &self.persisted_state,
            &mut self.effects,
            target,
            command,
        )?;
        operations::check_native_site_policy(command, &origin)?;
        let persistence = &self.persistence;
        let expected = &self.persisted_state;
        let effects = &mut self.effects;
        let mut check = |command: &Value| {
            let origin = qualified_origin(persistence, expected, effects, target, command)?;
            operations::check_native_site_policy(command, &origin)?;
            operations::native_image_attempt_observation(command)
        };
        run_native_owner(
            persistence,
            expected,
            original,
            assignment,
            command,
            None,
            Some(&mut check),
        )
    }
}
