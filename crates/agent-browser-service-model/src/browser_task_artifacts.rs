//! Completed artifact references belong to the retained logical task.
use crate::{BrowserSessionState, BrowserTaskArtifactReference};

impl BrowserSessionState {
    /// Retain completion metadata after an owner adapter has produced the file.
    /// Publication tolerates activity/expiry and peer changes, fences the original
    /// task, target and process, and never repeats capture or retains file contents.
    pub fn retain_completed_task_artifact(
        &mut self,
        before: &Self,
        task_id: &str,
        reference: BrowserTaskArtifactReference,
    ) -> Result<(), String> {
        let original = before
            .sessions
            .get(task_id)
            .ok_or("browser_artifact_completed_reference_context_changed")?;
        let current = self
            .sessions
            .get(task_id)
            .ok_or("browser_artifact_completed_reference_context_changed")?;
        if current.id != task_id
            || original.id != task_id
            || current.name != original.name
            || current.profile_id != original.profile_id
            || current.browser_id != original.browser_id
            || current.created_at_ms != original.created_at_ms
            || current.current_tab_id != original.current_tab_id
        {
            return Err("browser_artifact_completed_reference_context_changed".into());
        }
        let tab_id = original
            .current_tab_id
            .as_deref()
            .ok_or("browser_artifact_completed_reference_context_changed")?;
        let original_tab = before
            .tabs
            .get(tab_id)
            .ok_or("browser_artifact_completed_reference_context_changed")?;
        let current_tab = self
            .tabs
            .get(tab_id)
            .ok_or("browser_artifact_completed_reference_context_changed")?;
        if original_tab.target_id.is_empty()
            || original_tab.id != tab_id
            || original_tab.session_id != task_id
            || original_tab.browser_id != original.browser_id
            || (
                &current_tab.id,
                &current_tab.target_id,
                &current_tab.session_id,
                &current_tab.browser_id,
            ) != (
                &original_tab.id,
                &original_tab.target_id,
                &original_tab.session_id,
                &original_tab.browser_id,
            )
        {
            return Err("browser_artifact_completed_reference_context_changed".into());
        }
        let original_browser = before
            .browsers
            .get(&original.browser_id)
            .ok_or("browser_artifact_completed_reference_context_changed")?;
        let current_browser = self
            .browsers
            .get(&original.browser_id)
            .ok_or("browser_artifact_completed_reference_context_changed")?;
        if original_browser.id != original.browser_id
            || original_browser.profile_id != original.profile_id
            || !original_browser
                .active_session_ids
                .iter()
                .any(|id| id == task_id)
            || !current_browser
                .active_session_ids
                .iter()
                .any(|id| id == task_id)
            || (
                &current_browser.id,
                &current_browser.profile_id,
                current_browser.pid,
                &current_browser.cdp_endpoint,
                current_browser
                    .desktop
                    .as_ref()
                    .map(|d| (&d.desktop_id, d.generation)),
            ) != (
                &original_browser.id,
                &original_browser.profile_id,
                original_browser.pid,
                &original_browser.cdp_endpoint,
                original_browser
                    .desktop
                    .as_ref()
                    .map(|d| (&d.desktop_id, d.generation)),
            )
        {
            return Err("browser_artifact_completed_reference_context_changed".into());
        }
        let references = self
            .task_artifact_references
            .entry(task_id.into())
            .or_default();
        if references
            .iter()
            .any(|row| row.artifact_id == reference.artifact_id)
        {
            return Ok(());
        }
        if references.len() >= 256 {
            return Err("browser_artifact_completed_reference_capacity_reached".into());
        }
        references.push(reference);
        Ok(())
    }
}
