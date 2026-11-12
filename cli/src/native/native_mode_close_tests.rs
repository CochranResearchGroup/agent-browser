use super::*;

struct CloseOwner {
    active: bool,
    owner: &'static str,
    epoch: u64,
    requests: Vec<(String, Value)>,
}
impl NativeOwner for CloseOwner {
    fn request(&mut self, operation: &str, body: &Value) -> Result<Value, String> {
        self.requests.push((operation.into(), body.clone()));
        match operation {
            "observe" => Ok(
                json!({"desktopId":"viewer","generation":7,"geometry":{"current":{"width":800,"height":600}},"windows":{"windows":[{"id":11,"pid":123,"active":self.active,"x":0,"y":0,"width":800,"height":600,"processIdentity":{"pid":123,"executable":"/fixture/chrome","boot_id":"boot","start_token":"123"}}]}}),
            ),
            "control" => Ok(
                json!({"desktopId":"viewer","generation":7,"owner":self.owner,"epoch":self.epoch}),
            ),
            "window" | "input" => {
                if body["controlEpoch"] != self.epoch || self.owner != "agent" {
                    return Err("input_control_conflict".into());
                }
                if operation == "window" {
                    return Ok(json!({"operation":"window.close","generation":7,"windowId":11}));
                }
                Ok(json!({"delivery":"dispatched"}))
            }
            _ => panic!("unexpected owner operation {operation}"),
        }
    }
}
fn run(owner: &mut CloseOwner, command: Value) -> Result<Value, String> {
    execute(
        &RecordedProcessIdentity {
            pid: 123,
            start_token: "linux:boot:123".into(),
            executable_path: Some("/fixture/chrome".into()),
            browser_family: Some("chrome".into()),
        },
        &RemoteViewApplicationTarget {
            assignment_id: "assignment".into(),
            registration_id: "registration".into(),
            desktop_id: "lifecycle".into(),
            lifecycle_generation: 1,
            viewing_desktop_id: "viewer".into(),
            viewing_generation: 7,
        },
        &command,
        owner,
    )
}
#[test]
fn native_mode_close_fences_active_and_inactive_window_with_current_agent_epoch() {
    for active in [true, false] {
        let mut owner = CloseOwner {
            active,
            owner: "agent",
            epoch: 9,
            requests: vec![],
        };
        assert_eq!(
            run(&mut owner, json!({"action":"native_close"})).unwrap()["delivery"],
            "dispatched"
        );
        let effects: Vec<_> = owner
            .requests
            .iter()
            .filter(|(op, _)| matches!(op.as_str(), "window" | "input"))
            .collect();
        assert_eq!(effects.len(), 1);
        assert!(effects.iter().all(|(_, body)| body["controlEpoch"] == 9));
        assert_eq!(effects[0].0, "window");
        assert_eq!(effects[0].1["windowId"], 11);
        assert_eq!(
            effects[0].1["expectedProcessIdentity"]["start_token"],
            "123"
        );
        assert_eq!(effects[0].1["action"], "close");
    }
}
#[test]
fn native_mode_close_never_acquires_an_operator_lease_or_autofills_generic_input() {
    let mut owner = CloseOwner {
        active: true,
        owner: "operator",
        epoch: 9,
        requests: vec![],
    };
    assert!(run(&mut owner, json!({"action":"native_close"})).is_err());
    assert!(!owner
        .requests
        .iter()
        .any(|(op, _)| matches!(op.as_str(), "window" | "input")));
    let mut owner = CloseOwner {
        active: true,
        owner: "agent",
        epoch: 9,
        requests: vec![],
    };
    assert_eq!(
        run(
            &mut owner,
            json!({"action":"native_input","actions":[{"kind":"key","chord":"alt+F4"}]})
        )
        .unwrap_err(),
        "input_control_conflict"
    );
    assert!(!owner.requests.iter().any(|(op, _)| op == "control"));
    owner.requests.clear();
    assert!(run(
        &mut owner,
        json!({"action":"native_close","controlEpoch":8})
    )
    .is_err());
    assert!(!owner
        .requests
        .iter()
        .any(|(op, _)| matches!(op.as_str(), "window" | "input")));
}
