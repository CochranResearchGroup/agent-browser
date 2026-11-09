use super::*;
use base64::Engine;
use image::{Rgba, RgbaImage};
use sha2::{Digest, Sha256};

struct LoginOwner {
    calls: Vec<String>,
    capture: Value,
    observation: Option<Value>,
    epoch: u64,
    last_input: Option<Value>,
}
impl NativeOwner for LoginOwner {
    fn request(&mut self, op: &str, body: &Value) -> Result<Value, String> {
        self.calls.push(op.into());
        match op {
            "observe" => Ok(
                json!({"desktopId":"desktop","generation":7,"geometry":{"current":{"width":800,"height":600}},"windows":{"windows":[{"id":11,"pid":123,"active":true,"x":0,"y":0,"width":800,"height":600,"processIdentity":{"pid":123,"executable":"/fixture/chrome","boot_id":"boot","start_token":"123"}}]}}),
            ),
            "control" => {
                Ok(json!({"desktopId":"desktop","generation":7,"owner":"agent","epoch":self.epoch}))
            }
            "ocr" | "capture_region" => Ok(self.capture.clone()),
            "input" => {
                self.last_input = Some(body.clone());
                Ok(json!({"delivery":"dispatched"}))
            }
            _ => panic!("read-only perception dispatched {op}"),
        }
    }
    fn native_login_attempt(&mut self, _command: &Value) -> Result<Value, String> {
        self.observation
            .clone()
            .ok_or("missing fixture observation".into())
    }
}
fn login_fixture() -> LoginOwner {
    let mut image = RgbaImage::from_pixel(800, 600, Rgba([255, 255, 255, 255]));
    for x in 100..400 {
        image.put_pixel(x, 150, Rgba([100, 100, 100, 255]));
        image.put_pixel(x, 190, Rgba([100, 100, 100, 255]));
    }
    for y in 150..191 {
        image.put_pixel(100, y, Rgba([100, 100, 100, 255]));
        image.put_pixel(399, y, Rgba([100, 100, 100, 255]));
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let png = bytes.into_inner();
    LoginOwner {
        calls: vec![],
        observation: None,
        epoch: 4,
        last_input: None,
        capture: json!({"desktopId":"desktop","generation":7,"windowId":11,"processIdentity":{"pid":123,"executable":"/fixture/chrome","boot_id":"boot","start_token":"123"},"capture":{"sha256":format!("{:x}",Sha256::digest(&png))},"image":{"data":base64::engine::general_purpose::STANDARD.encode(png)},"ocr":{"engine":"tesseract","engineVersion":"5.3.4","language":"eng","minimumConfidence":60.0,"modelSha256":"7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2","state":"observed","lines":[{"text":"Email address","bounds":{"x":100,"y":120,"width":110,"height":18},"confidence":95}],"words":[]}}),
    }
}
fn perceive(owner: &mut LoginOwner) -> Result<Value, String> {
    execute(
        &RecordedProcessIdentity {
            pid: 123,
            start_token: "linux:boot:123".into(),
            executable_path: Some("/fixture/chrome".into()),
            browser_family: Some("chromium".into()),
        },
        &RemoteViewApplicationTarget {
            assignment_id: "assignment".into(),
            registration_id: "registration".into(),
            desktop_id: "allocation".into(),
            lifecycle_generation: 7,
            viewing_desktop_id: "desktop".into(),
            viewing_generation: 7,
        },
        &json!({"action":"native_capture","loginControls":{"profile":"english-chrome-v1"}}),
        owner,
    )
}
#[test]
fn native_login_perception_public_capture_enumerates_field_without_exporting_ocr_or_pixels() {
    let mut owner = login_fixture();
    let result = perceive(&mut owner).unwrap();
    assert_eq!(result["loginControls"]["status"], "observed");
    assert_eq!(result["loginControls"]["controls"][0]["kind"], "username");
    assert_eq!(
        result["loginControls"]["controls"][0]["bounds"],
        json!({"x":100,"y":150,"width":300,"height":41})
    );
    assert!(result["capture"]["image"].is_null());
    assert!(result["capture"]["ocr"].is_null());
    assert_eq!(owner.calls, vec!["observe", "control", "ocr"]);
}

fn select(owner: &mut LoginOwner, reference: &str) -> Result<Value, String> {
    execute(
        &RecordedProcessIdentity {
            pid: 123,
            start_token: "linux:boot:123".into(),
            executable_path: Some("/fixture/chrome".into()),
            browser_family: Some("chromium".into()),
        },
        &RemoteViewApplicationTarget {
            assignment_id: "assignment".into(),
            registration_id: "registration".into(),
            desktop_id: "allocation".into(),
            lifecycle_generation: 7,
            viewing_desktop_id: "desktop".into(),
            viewing_generation: 7,
        },
        &json!({"action":"native_input","loginTarget":{"observationRequestId":"capture","episodeId":"episode","targetRef":reference}}),
        owner,
    )
}
#[test]
fn native_login_perception_public_input_derives_single_click_and_fences_visual_control() {
    let mut owner = login_fixture();
    let frame = perceive(&mut owner).unwrap();
    let reference = frame["loginControls"]["controls"][0]["targetRef"]
        .as_str()
        .unwrap();
    owner.observation = Some(frame["nativeLoginObservation"].clone());
    let r = select(&mut owner, reference).unwrap();
    assert_eq!(r["delivery"], "dispatched");
    let input = owner.last_input.as_ref().unwrap();
    assert_eq!(
        input["actions"],
        json!([{"kind":"click","x":250,"y":170,"button":1,"count":1}])
    );
    assert_eq!(input["controlEpoch"], 4);
    assert_eq!(
        input["expectedVisual"]["sha256"],
        frame["loginControls"]["frameSha256"]
    );
    owner.last_input = None;
    owner.epoch = 5;
    assert!(select(&mut owner, reference)
        .unwrap_err()
        .contains("control_changed"));
    assert!(owner.last_input.is_none());
    owner.epoch = 4;
    owner.observation.as_mut().unwrap()["binding"]["observedAtMs"] = json!(1);
    assert!(select(&mut owner, reference)
        .unwrap_err()
        .contains("expired"));
    assert!(owner.last_input.is_none());
}
#[test]
fn native_login_perception_public_capture_refuses_unmatched_owner_and_provider_profile() {
    let mut owner = login_fixture();
    owner.capture["windowId"] = json!(999);
    assert!(perceive(&mut owner)
        .unwrap_err()
        .contains("context_changed"));
    assert!(owner.last_input.is_none());
    owner.capture["windowId"] = json!(11);
    owner.capture["ocr"]["engineVersion"] = json!("unsupported");
    assert!(perceive(&mut owner)
        .unwrap_err()
        .contains("ocr_profile_unsupported"));
    assert!(owner.last_input.is_none());
}
