//! Task-scoped Native Remote View operations. Never opens a debugging connection.
use agent_browser_service_model::{RecordedProcessIdentity, RemoteViewApplicationTarget};
use serde_json::{json, Value};

pub trait NativeOwner {
    fn request(&mut self, operation: &str, body: &Value) -> Result<Value, String>;
}
/// The caller retains browser ownership and establishes current mode. The native
/// owner independently fences the process, window, geometry and input epoch.
pub fn execute(
    original: &RecordedProcessIdentity,
    target: &RemoteViewApplicationTarget,
    command: &Value,
    owner: &mut impl NativeOwner,
) -> Result<Value, String> {
    if command["action"] != "native_input" {
        return Err("browser_native_action_unavailable".into());
    }
    let actions = command["actions"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 16)
        .ok_or("browser_native_actions_required")?;
    let snapshot = owner.request("observe", &json!({"desktopId":target.viewing_desktop_id,"expectedGeneration":target.viewing_generation}))?;
    if snapshot["desktopId"] != target.viewing_desktop_id
        || snapshot["generation"] != target.viewing_generation
    {
        return Err("browser_native_generation_changed:refresh inventory".into());
    }
    let windows = snapshot
        .pointer("/windows/windows")
        .and_then(Value::as_array)
        .ok_or("browser_native_window_unverified:refresh inventory")?;
    let candidates: Vec<&Value> = windows
        .iter()
        .filter(|window| {
            let process = &window["processIdentity"];
            window["pid"].as_u64() == Some(u64::from(original.pid))
                && process["pid"].as_u64() == Some(u64::from(original.pid))
                && original
                    .executable_path
                    .as_deref()
                    .is_some_and(|path| process["executable"] == path)
                && process["boot_id"]
                    .as_str()
                    .zip(process["start_token"].as_str())
                    .is_some_and(|(boot, start)| {
                        original.start_token == format!("linux:{boot}:{start}")
                    })
                && window["width"].as_u64().is_some_and(|n| n > 0)
                && window["height"].as_u64().is_some_and(|n| n > 0)
        })
        .collect();
    let active: Vec<&Value> = candidates
        .iter()
        .copied()
        .filter(|window| window["active"] == true)
        .collect();
    let window = match (active.as_slice(), candidates.as_slice()) {
        ([window], _) | ([], [window]) => *window,
        _ => {
            return Err(
                "browser_native_window_unverified:focus browser and refresh inventory".into(),
            )
        }
    };
    let width = snapshot
        .pointer("/geometry/current/width")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .ok_or("browser_native_geometry_unverified")?;
    let height = snapshot
        .pointer("/geometry/current/height")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .ok_or("browser_native_geometry_unverified")?;
    let mut body = json!({"desktopId":target.viewing_desktop_id,"expectedGeneration":target.viewing_generation,
        "windowId":window["id"],"expectedProcessIdentity":window["processIdentity"],"expectedWidth":width,"expectedHeight":height,
        "expectedWindow":{"x":window["x"],"y":window["y"],"width":window["width"],"height":window["height"]},"actions":actions});
    if let Some(epoch) = command.get("controlEpoch") {
        body["controlEpoch"] = epoch.clone();
    }
    let reply = owner.request("input", &body)?;
    Ok(
        json!({"source":"remote_view_native","delivery":reply["delivery"].as_str().unwrap_or("unknown"),"taskCompleted":false}),
    )
}
