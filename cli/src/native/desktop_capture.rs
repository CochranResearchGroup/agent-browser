//! Display-bound desktop frame capture.
//!
//! This module owns the full observe-only capture workflow behind one caller-facing
//! function. Callers supply a browser identity, never a display name or provider
//! route. The implementation resolves the current service-owned RDP binding,
//! captures through an internal frame-provider seam, and rejects post-capture
//! identity or geometry drift.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use image::{ImageFormat, ImageReader, Limits};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::remote_view_attachability::derive_stream_attachability;
use super::service_contracts::{DESKTOP_CAPTURE_DEFAULT_MAX_BYTES, DESKTOP_CAPTURE_HARD_MAX_BYTES};
use super::service_model::{
    BrowserHealth, BrowserHost, BrowserProcess, DisplayAllocation, RemoteViewRoute, ServiceState,
    ViewStream, ViewStreamProvider,
};
use super::service_store::load_default_service_state_snapshot;
use crate::flags::load_config;

const DESKTOP_CONTEXT_SCHEMA_VERSION: &str = "v1";
const FRAME_RECEIPT_SCHEMA_VERSION: &str = "v1";
const COORDINATE_SPACE: &str = "desktop_physical_pixels";
const CAPTURE_PROVIDER: &str = "x11_root";
const CAPTURE_PROVIDER_VERSION: &str = "imagemagick-import-v1";
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_STDERR_BYTES: u64 = 4096;
const MAX_DIMENSION: u32 = 16384;
const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_DECODE_BYTES: u64 = MAX_PIXELS * 4;
static FRAME_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) const DEFAULT_MAX_BYTES: u64 = DESKTOP_CAPTURE_DEFAULT_MAX_BYTES;
pub(crate) const HARD_MAX_BYTES: u64 = DESKTOP_CAPTURE_HARD_MAX_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopCaptureRequest {
    pub browser_id: String,
    pub session_name: Option<String>,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopContext {
    pub context_id: String,
    pub schema_version: &'static str,
    pub browser_id: String,
    pub session_name: String,
    pub profile_id: Option<String>,
    pub display_allocation_id: String,
    pub stream_id: String,
    pub route_id: String,
    pub capture_provider: &'static str,
    pub view_stream_provider: ViewStreamProvider,
    pub display_isolation: String,
    pub coordinate_space: &'static str,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub geometry_epoch: String,
    pub resolved_at: String,
    #[serde(rename = "readinessEvidence")]
    pub readiness: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrameReceipt {
    pub frame_id: String,
    pub schema_version: &'static str,
    pub context_id: String,
    pub capture_provider: &'static str,
    pub provider_version: String,
    pub sequence: u64,
    pub captured_at: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub geometry_epoch: String,
    pub mime_type: &'static str,
    pub byte_length: usize,
    #[serde(rename = "sha256")]
    pub content_sha256: String,
    pub freshness: &'static str,
    pub retention: &'static str,
    pub persisted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DesktopCaptureResult {
    pub context: DesktopContext,
    pub frame_receipt: FrameReceipt,
    pub image_bytes: Vec<u8>,
}

/// Exact service-owned binding reused by configured desktop evidence adapters.
/// The display name remains internal and is never projected through a product
/// action or receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopCaptureBinding {
    pub(crate) browser_id: String,
    pub(crate) display_allocation_id: String,
    pub(crate) display_name: String,
    pub(crate) route_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geometry {
    width: u32,
    height: u32,
    scale_factor_millis: u32,
}

impl Geometry {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            scale_factor_millis: 1000,
        }
    }

    fn scale_factor(self) -> f64 {
        self.scale_factor_millis as f64 / 1000.0
    }
}

#[derive(Debug, Clone)]
struct ResolvedDesktop {
    browser_id: String,
    session_name: String,
    profile_id: Option<String>,
    display_allocation_id: String,
    display_name: String,
    display_isolation: String,
    stream_id: String,
    route_id: String,
    view_stream_provider: ViewStreamProvider,
    readiness: Value,
}

trait StateSource: Send + Sync {
    fn snapshot(&self) -> Result<ServiceState, DesktopCaptureError>;
}

trait FrameProvider: Send + Sync {
    fn provider_version(&self) -> &str;
    fn geometry(&self, display_name: &str) -> Result<Geometry, DesktopCaptureError>;
    fn capture_png(
        &self,
        display_name: &str,
        max_bytes: u64,
    ) -> Result<Vec<u8>, DesktopCaptureError>;
}

trait CaptureClock: Send + Sync {
    fn now(&self) -> String;
}

trait SequenceSource: Send + Sync {
    fn next(&self) -> u64;
}

struct CaptureDependencies<'a> {
    state_source: &'a dyn StateSource,
    frame_provider: &'a dyn FrameProvider,
    clock: &'a dyn CaptureClock,
    sequence: &'a dyn SequenceSource,
}

impl<'a> CaptureDependencies<'a> {
    fn new(
        state_source: &'a dyn StateSource,
        frame_provider: &'a dyn FrameProvider,
        clock: &'a dyn CaptureClock,
        sequence: &'a dyn SequenceSource,
    ) -> Self {
        Self {
            state_source,
            frame_provider,
            clock,
            sequence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopCaptureError {
    code: &'static str,
    message: String,
}

impl DesktopCaptureError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl std::fmt::Display for DesktopCaptureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DesktopCaptureError {}

fn capture_desktop_frame(
    request: DesktopCaptureRequest,
    dependencies: CaptureDependencies<'_>,
) -> Result<DesktopCaptureResult, DesktopCaptureError> {
    validate_request(&request)?;
    let before_state = dependencies.state_source.snapshot()?;
    let before = resolve_desktop(&before_state, &request)?;
    let before_geometry = dependencies.frame_provider.geometry(&before.display_name)?;
    validate_geometry(before_geometry)?;
    let image_bytes = dependencies
        .frame_provider
        .capture_png(&before.display_name, request.max_bytes)?;
    if image_bytes.len() as u64 > request.max_bytes {
        return Err(DesktopCaptureError::new(
            "desktop_frame_too_large",
            format!(
                "captured frame is {} bytes and exceeds maxBytes {}",
                image_bytes.len(),
                request.max_bytes
            ),
        ));
    }
    let decoded_geometry = bounded_png_geometry(&image_bytes)?;
    if decoded_geometry.width != before_geometry.width
        || decoded_geometry.height != before_geometry.height
    {
        return Err(DesktopCaptureError::new(
            "desktop_geometry_changed",
            "captured PNG dimensions do not match the resolved display geometry",
        ));
    }

    let after_state = dependencies.state_source.snapshot()?;
    let after = resolve_desktop(&after_state, &request)?;
    let after_geometry = dependencies.frame_provider.geometry(&after.display_name)?;
    if !same_binding(&before, &after) || before_geometry != after_geometry {
        return Err(DesktopCaptureError::new(
            "desktop_geometry_changed",
            "desktop binding or geometry changed during capture",
        ));
    }

    let resolved_at = dependencies.clock.now();
    let captured_at = dependencies.clock.now();
    let sequence = dependencies.sequence.next();
    let geometry_epoch = desktop_geometry_epoch(
        &DesktopCaptureBinding {
            browser_id: before.browser_id.clone(),
            display_allocation_id: before.display_allocation_id.clone(),
            display_name: before.display_name.clone(),
            route_id: before.route_id.clone(),
        },
        before_geometry.width,
        before_geometry.height,
        before_geometry.scale_factor_millis,
    );
    let context_id = format!(
        "desktop-context-{}",
        &digest_text(&format!(
            "{}\0{}\0{}\0{}\0{}",
            before.browser_id, before.stream_id, before.route_id, geometry_epoch, resolved_at
        ))[..24]
    );
    let content_sha256 = digest_bytes(&image_bytes);
    let frame_id = format!(
        "desktop-frame-{}",
        &digest_text(&format!(
            "{}\0{}\0{}\0{}",
            context_id, sequence, captured_at, content_sha256
        ))[..24]
    );
    let scale_factor = before_geometry.scale_factor();
    let context = DesktopContext {
        context_id: context_id.clone(),
        schema_version: DESKTOP_CONTEXT_SCHEMA_VERSION,
        browser_id: before.browser_id,
        session_name: before.session_name,
        profile_id: before.profile_id,
        display_allocation_id: before.display_allocation_id,
        stream_id: before.stream_id,
        route_id: before.route_id,
        capture_provider: CAPTURE_PROVIDER,
        view_stream_provider: before.view_stream_provider,
        display_isolation: before.display_isolation,
        coordinate_space: COORDINATE_SPACE,
        width: before_geometry.width,
        height: before_geometry.height,
        scale_factor,
        geometry_epoch: geometry_epoch.clone(),
        resolved_at,
        readiness: before.readiness,
    };
    let frame_receipt = FrameReceipt {
        frame_id,
        schema_version: FRAME_RECEIPT_SCHEMA_VERSION,
        context_id,
        capture_provider: CAPTURE_PROVIDER,
        provider_version: dependencies.frame_provider.provider_version().to_string(),
        sequence,
        captured_at,
        width: before_geometry.width,
        height: before_geometry.height,
        scale_factor,
        geometry_epoch,
        mime_type: "image/png",
        byte_length: image_bytes.len(),
        content_sha256,
        freshness: "fresh_capture",
        retention: "ephemeral",
        persisted: false,
    };
    Ok(DesktopCaptureResult {
        context,
        frame_receipt,
        image_bytes,
    })
}

/// Capture one frame through the configured display-bound provider while
/// preserving all pre-capture and post-capture identity checks.
pub(crate) fn capture_configured_desktop_frame(
    request: DesktopCaptureRequest,
) -> Result<DesktopCaptureResult, DesktopCaptureError> {
    capture_desktop_frame(
        request,
        CaptureDependencies::new(
            &ConfiguredStateSource,
            &X11RootFrameProvider,
            &SystemClock,
            &ProcessSequence,
        ),
    )
}

pub(crate) fn resolve_desktop_capture_binding(
    state: &ServiceState,
    browser_id: &str,
) -> Result<DesktopCaptureBinding, DesktopCaptureError> {
    let resolved = resolve_desktop(
        state,
        &DesktopCaptureRequest {
            browser_id: browser_id.to_string(),
            session_name: None,
            max_bytes: DEFAULT_MAX_BYTES,
        },
    )?;
    Ok(DesktopCaptureBinding {
        browser_id: resolved.browser_id,
        display_allocation_id: resolved.display_allocation_id,
        display_name: resolved.display_name,
        route_id: resolved.route_id,
    })
}

pub(crate) fn desktop_geometry_epoch(
    binding: &DesktopCaptureBinding,
    width: u32,
    height: u32,
    scale_factor_millis: u32,
) -> String {
    digest_text(&format!(
        "{}\0{}\0{}\0{}\0{}\0{}",
        binding.browser_id,
        binding.display_allocation_id,
        binding.route_id,
        width,
        height,
        scale_factor_millis
    ))
}

fn validate_request(request: &DesktopCaptureRequest) -> Result<(), DesktopCaptureError> {
    if request.browser_id.trim().is_empty() {
        return Err(DesktopCaptureError::new(
            "desktop_workspace_not_found",
            "browserId must not be empty",
        ));
    }
    if request.max_bytes == 0 || request.max_bytes > HARD_MAX_BYTES {
        return Err(DesktopCaptureError::new(
            "desktop_frame_too_large",
            format!("maxBytes must be between 1 and {HARD_MAX_BYTES}"),
        ));
    }
    Ok(())
}

fn resolve_desktop(
    state: &ServiceState,
    request: &DesktopCaptureRequest,
) -> Result<ResolvedDesktop, DesktopCaptureError> {
    let browser = state.browsers.get(&request.browser_id).ok_or_else(|| {
        DesktopCaptureError::new(
            "desktop_workspace_not_found",
            format!(
                "browser {} is not present in service state",
                request.browser_id
            ),
        )
    })?;
    if browser.id != request.browser_id {
        return Err(identity_mismatch(
            "browser map key and browser identity disagree",
        ));
    }
    if terminal_browser_health(browser.health) {
        return Err(DesktopCaptureError::new(
            "desktop_display_not_ready",
            format!("browser {} has terminal health", browser.id),
        ));
    }
    let session_name = resolve_session(browser, request.session_name.as_deref())?;
    let streams = browser
        .view_streams
        .iter()
        .filter(|stream| stream.provider == ViewStreamProvider::RdpGateway)
        .collect::<Vec<_>>();
    if streams.is_empty() {
        return Err(DesktopCaptureError::new(
            "desktop_workspace_not_found",
            format!("browser {} has no RDP desktop stream", browser.id),
        ));
    }
    if streams.len() != 1 {
        return Err(DesktopCaptureError::new(
            "desktop_workspace_ambiguous",
            format!(
                "browser {} has {} RDP desktop streams",
                browser.id,
                streams.len()
            ),
        ));
    }
    resolve_candidate(state, browser, streams[0], &session_name)
}

fn resolve_candidate(
    state: &ServiceState,
    browser: &BrowserProcess,
    stream: &ViewStream,
    session_name: &str,
) -> Result<ResolvedDesktop, DesktopCaptureError> {
    let route_id = required_id(stream.route_id.as_deref(), "stream route")?;
    let route = state.remote_view_routes.get(route_id).ok_or_else(|| {
        DesktopCaptureError::new(
            "desktop_route_not_ready",
            format!("route {route_id} is missing"),
        )
    })?;
    if route.id != route_id {
        return Err(identity_mismatch(
            "route map key and route identity disagree",
        ));
    }
    validate_route(route, browser, session_name)?;
    let display_id = exact_display_id(browser, stream, route)?;
    let display = state.display_allocations.get(display_id).ok_or_else(|| {
        DesktopCaptureError::new(
            "desktop_display_not_ready",
            format!("display allocation {display_id} is missing"),
        )
    })?;
    if display.id != display_id {
        return Err(identity_mismatch(
            "display map key and display allocation identity disagree",
        ));
    }
    validate_display(display, browser, route, session_name)?;
    let display_name = display.display_name.as_deref().ok_or_else(|| {
        DesktopCaptureError::new(
            "desktop_geometry_unavailable",
            format!("display allocation {} has no display binding", display.id),
        )
    })?;
    if browser.display_name.as_deref() != Some(display_name) {
        return Err(identity_mismatch(
            "browser and display allocation names disagree",
        ));
    }
    let attachability = derive_stream_attachability(browser, stream, state);
    if attachability.get("state").and_then(Value::as_str) != Some("attached_ready")
        || attachability.get("proofState").and_then(Value::as_str) != Some("ready")
        || attachability
            .get("displayContentState")
            .and_then(Value::as_str)
            != Some("browser_window_visible")
    {
        return Err(DesktopCaptureError::new(
            "desktop_route_not_ready",
            format!("route {route_id} lacks current operator-visible display proof"),
        ));
    }
    Ok(ResolvedDesktop {
        browser_id: browser.id.clone(),
        session_name: session_name.to_string(),
        profile_id: browser.profile_id.clone(),
        display_allocation_id: display.id.clone(),
        display_name: display_name.to_string(),
        display_isolation: display.display_isolation.clone(),
        stream_id: stream.id.clone(),
        route_id: route.id.clone(),
        view_stream_provider: stream.provider,
        readiness: json!({
            "attachabilityState": attachability.get("state").cloned().unwrap_or(Value::Null),
            "proofState": attachability.get("proofState").cloned().unwrap_or(Value::Null),
            "displayContentState": attachability.get("displayContentState").cloned().unwrap_or(Value::Null),
            "routeState": route.state,
            "displayState": display.state,
        }),
    })
}

fn resolve_session(
    browser: &BrowserProcess,
    requested: Option<&str>,
) -> Result<String, DesktopCaptureError> {
    if let Some(requested) = requested {
        if requested.trim().is_empty()
            || !browser.active_session_ids.iter().any(|id| id == requested)
        {
            return Err(identity_mismatch(
                "requested session is not active on the selected browser",
            ));
        }
        return Ok(requested.to_string());
    }
    if browser.active_session_ids.len() != 1 {
        return Err(DesktopCaptureError::new(
            "desktop_workspace_ambiguous",
            format!(
                "browser {} requires sessionName because it has {} active sessions",
                browser.id,
                browser.active_session_ids.len()
            ),
        ));
    }
    Ok(browser.active_session_ids[0].clone())
}

fn validate_route(
    route: &RemoteViewRoute,
    browser: &BrowserProcess,
    session_name: &str,
) -> Result<(), DesktopCaptureError> {
    if route.provider != ViewStreamProvider::RdpGateway
        || route.browser_id.as_deref() != Some(browser.id.as_str())
        || route.session_id.as_deref() != Some(session_name)
    {
        return Err(identity_mismatch(
            "route ownership does not match the request",
        ));
    }
    if route.state != "ready" {
        return Err(DesktopCaptureError::new(
            "desktop_route_not_ready",
            format!("route {} state is not ready", route.id),
        ));
    }
    Ok(())
}

fn exact_display_id<'a>(
    browser: &'a BrowserProcess,
    stream: &'a ViewStream,
    route: &'a RemoteViewRoute,
) -> Result<&'a str, DesktopCaptureError> {
    let browser_id = required_id(browser.display_allocation_id.as_deref(), "browser display")?;
    let stream_id = required_id(stream.display_allocation_id.as_deref(), "stream display")?;
    let route_id = required_id(route.display_allocation_id.as_deref(), "route display")?;
    if browser_id != stream_id || stream_id != route_id {
        return Err(identity_mismatch(
            "browser, stream, and route display allocations disagree",
        ));
    }
    Ok(browser_id)
}

fn validate_display(
    display: &DisplayAllocation,
    browser: &BrowserProcess,
    route: &RemoteViewRoute,
    session_name: &str,
) -> Result<(), DesktopCaptureError> {
    if display.owner_browser_id.as_deref() != Some(browser.id.as_str())
        || display.owner_session_id.as_deref() != Some(session_name)
        || !display.route_ids.iter().any(|id| id == &route.id)
        || display.host != Some(BrowserHost::RemoteHeaded)
    {
        return Err(identity_mismatch(
            "display allocation ownership does not match the route",
        ));
    }
    if display.state != "ready" {
        return Err(DesktopCaptureError::new(
            "desktop_display_not_ready",
            format!("display allocation {} state is not ready", display.id),
        ));
    }
    if !matches!(
        display.display_isolation.as_str(),
        "private_virtual_display" | "shared_display" | "ambient_display"
    ) {
        return Err(identity_mismatch(
            "display allocation isolation is unsupported",
        ));
    }
    Ok(())
}

fn required_id<'a>(value: Option<&'a str>, label: &str) -> Result<&'a str, DesktopCaptureError> {
    value
        .filter(|value| !value.is_empty())
        .ok_or_else(|| identity_mismatch(format!("{label} identity is missing")))
}

fn identity_mismatch(message: impl Into<String>) -> DesktopCaptureError {
    DesktopCaptureError::new("desktop_identity_mismatch", message)
}

fn terminal_browser_health(health: BrowserHealth) -> bool {
    matches!(
        health,
        BrowserHealth::NotStarted
            | BrowserHealth::ProcessExited
            | BrowserHealth::Closing
            | BrowserHealth::Faulted
    )
}

fn validate_geometry(geometry: Geometry) -> Result<(), DesktopCaptureError> {
    if geometry.width == 0
        || geometry.height == 0
        || geometry.width > MAX_DIMENSION
        || geometry.height > MAX_DIMENSION
        || geometry.scale_factor_millis == 0
        || u64::from(geometry.width) * u64::from(geometry.height) > MAX_PIXELS
    {
        return Err(DesktopCaptureError::new(
            "desktop_geometry_unavailable",
            "display geometry is absent or outside supported bounds",
        ));
    }
    Ok(())
}

fn bounded_png_geometry(bytes: &[u8]) -> Result<Geometry, DesktopCaptureError> {
    let mut reader = ImageReader::with_format(std::io::Cursor::new(bytes), ImageFormat::Png);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let (width, height) = reader.into_dimensions().map_err(|_| {
        DesktopCaptureError::new(
            "desktop_capture_failed",
            "capture provider returned an invalid or resource-unbounded PNG frame",
        )
    })?;
    let geometry = Geometry::new(width, height);
    validate_geometry(geometry)?;
    Ok(geometry)
}

fn same_binding(left: &ResolvedDesktop, right: &ResolvedDesktop) -> bool {
    left.browser_id == right.browser_id
        && left.session_name == right.session_name
        && left.profile_id == right.profile_id
        && left.display_allocation_id == right.display_allocation_id
        && left.display_name == right.display_name
        && left.stream_id == right.stream_id
        && left.route_id == right.route_id
        && left.view_stream_provider == right.view_stream_provider
}

fn digest_text(value: &str) -> String {
    digest_bytes(value.as_bytes())
}

fn digest_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

struct SystemClock;

impl CaptureClock for SystemClock {
    fn now(&self) -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }
}

struct ProcessSequence;

impl SequenceSource for ProcessSequence {
    fn next(&self) -> u64 {
        FRAME_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    }
}

struct ConfiguredStateSource;

impl StateSource for ConfiguredStateSource {
    fn snapshot(&self) -> Result<ServiceState, DesktopCaptureError> {
        let mut state = load_default_service_state_snapshot().map_err(|_| {
            DesktopCaptureError::new(
                "desktop_capture_failed",
                "failed to load the current service-state snapshot",
            )
        })?;
        let configured = load_config(&[]).map_err(|_| {
            DesktopCaptureError::new(
                "desktop_capture_failed",
                "failed to load configured service state",
            )
        })?;
        state.overlay_configured_entities(configured.service_state_snapshot());
        state.refresh_derived_views();
        Ok(state)
    }
}

struct X11RootFrameProvider;

impl FrameProvider for X11RootFrameProvider {
    fn provider_version(&self) -> &str {
        CAPTURE_PROVIDER_VERSION
    }

    fn geometry(&self, display_name: &str) -> Result<Geometry, DesktopCaptureError> {
        let output = run_bounded_command(
            "xdpyinfo",
            &["-display", display_name],
            256 * 1024,
            CAPTURE_TIMEOUT,
        )?;
        parse_xdpyinfo_geometry(&output)
    }

    fn capture_png(
        &self,
        display_name: &str,
        max_bytes: u64,
    ) -> Result<Vec<u8>, DesktopCaptureError> {
        run_bounded_command(
            "import",
            &["-display", display_name, "-window", "root", "png:-"],
            max_bytes,
            CAPTURE_TIMEOUT,
        )
    }
}

fn run_bounded_command(
    program: &str,
    arguments: &[&str],
    max_stdout_bytes: u64,
    timeout: Duration,
) -> Result<Vec<u8>, DesktopCaptureError> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            DesktopCaptureError::new(
                if error.kind() == std::io::ErrorKind::NotFound {
                    "desktop_capture_provider_unavailable"
                } else {
                    "desktop_capture_failed"
                },
                format!("capture provider command {program} could not start"),
            )
        })?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stdout_reader = thread::spawn(move || read_bounded_and_drain(stdout, max_stdout_bytes));
    let stderr_reader = thread::spawn(move || read_bounded_and_drain(stderr, MAX_STDERR_BYTES));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(DesktopCaptureError::new(
                    "desktop_capture_failed",
                    format!("capture provider command {program} timed out"),
                ));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(DesktopCaptureError::new(
                    "desktop_capture_failed",
                    format!("capture provider command {program} could not be observed"),
                ));
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| {
            DesktopCaptureError::new("desktop_capture_failed", "capture output reader failed")
        })?
        .map_err(|_| {
            DesktopCaptureError::new("desktop_capture_failed", "capture output could not be read")
        })?;
    let _redacted_stderr = stderr_reader.join();
    if stdout.overflowed {
        return Err(DesktopCaptureError::new(
            "desktop_frame_too_large",
            format!("capture provider output exceeds maxBytes {max_stdout_bytes}"),
        ));
    }
    if !status.success() {
        return Err(DesktopCaptureError::new(
            "desktop_capture_failed",
            format!("capture provider command {program} failed"),
        ));
    }
    Ok(stdout.bytes)
}

#[derive(Debug, PartialEq, Eq)]
struct BoundedRead {
    bytes: Vec<u8>,
    overflowed: bool,
}

fn read_bounded_and_drain(
    mut reader: impl Read,
    max_retained_bytes: u64,
) -> std::io::Result<BoundedRead> {
    let max_retained_bytes = usize::try_from(max_retained_bytes).unwrap_or(usize::MAX);
    let mut bytes = Vec::with_capacity(max_retained_bytes.min(64 * 1024));
    let mut overflowed = false;
    let mut chunk = [0_u8; 8192];
    loop {
        let read = reader.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        let remaining = max_retained_bytes.saturating_sub(bytes.len());
        let retain = remaining.min(read);
        bytes.extend_from_slice(&chunk[..retain]);
        overflowed |= retain < read;
    }
    Ok(BoundedRead { bytes, overflowed })
}

fn parse_xdpyinfo_geometry(output: &[u8]) -> Result<Geometry, DesktopCaptureError> {
    let output = std::str::from_utf8(output).map_err(|_| {
        DesktopCaptureError::new(
            "desktop_geometry_unavailable",
            "display geometry output is not UTF-8",
        )
    })?;
    for line in output.lines() {
        let Some(rest) = line.trim().strip_prefix("dimensions:") else {
            continue;
        };
        let dimensions = rest.split_whitespace().next().unwrap_or_default();
        let Some((width, height)) = dimensions.split_once('x') else {
            continue;
        };
        if let (Ok(width), Ok(height)) = (width.parse::<u32>(), height.parse::<u32>()) {
            let geometry = Geometry::new(width, height);
            validate_geometry(geometry)?;
            return Ok(geometry);
        }
    }
    Err(DesktopCaptureError::new(
        "desktop_geometry_unavailable",
        "capture provider did not report bounded display dimensions",
    ))
}

pub(crate) async fn handle_desktop_capture(cmd: &Value) -> Result<Value, String> {
    let request = parse_request(cmd).map_err(|error| error.to_string())?;
    let result = tokio::task::spawn_blocking(move || capture_configured_desktop_frame(request))
        .await
        .map_err(|_| "desktop_capture_failed: desktop capture task failed".to_string())?
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "ok": true,
        "action": "desktop_capture",
        "context": result.context,
        "frameReceipt": result.frame_receipt,
        "imageBase64": BASE64_STANDARD.encode(result.image_bytes),
    }))
}

/// Remove response-only pixels before a desktop capture result enters the
/// long-lived stream event channel. The immediate request response is not
/// passed through this projection.
pub(crate) fn redact_desktop_capture_stream_result(data: &Value) -> Value {
    let mut redacted = data.clone();
    if let Some(record) = redacted.as_object_mut() {
        record.remove("imageBase64");
        record.insert("imagePayload".to_string(), json!("response_only"));
    }
    redacted
}

fn parse_request(cmd: &Value) -> Result<DesktopCaptureRequest, DesktopCaptureError> {
    for forbidden in [
        "displayName",
        "displayAllocationId",
        "routeId",
        "providerUrl",
        "xauthorityPath",
        "outputPath",
        "crop",
    ] {
        if cmd.get(forbidden).is_some() {
            return Err(identity_mismatch(format!(
                "desktop_capture does not accept caller-controlled {forbidden}"
            )));
        }
    }
    let browser_id = cmd
        .get("browserId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            DesktopCaptureError::new(
                "desktop_workspace_not_found",
                "desktop_capture requires browserId",
            )
        })?;
    let format = cmd.get("format").and_then(Value::as_str).unwrap_or("png");
    if format != "png" {
        return Err(DesktopCaptureError::new(
            "desktop_capture_failed",
            "desktop_capture format must be png",
        ));
    }
    let request = DesktopCaptureRequest {
        browser_id: browser_id.to_string(),
        session_name: cmd
            .get("sessionName")
            .and_then(Value::as_str)
            .map(str::to_string),
        max_bytes: cmd
            .get("maxBytes")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_MAX_BYTES),
    };
    validate_request(&request)?;
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::service_model::{
        BrowserHealth, BrowserHost, BrowserProcess, DisplayAllocation, RemoteViewRoute,
        ServiceState, ViewStream, ViewStreamProvider,
    };
    use serde_json::json;
    use std::collections::{BTreeMap, VecDeque};
    use std::io::{Cursor, Write};
    use std::sync::Mutex;

    struct FixedClock(&'static str);

    impl CaptureClock for FixedClock {
        fn now(&self) -> String {
            self.0.to_string()
        }
    }

    struct FixedSequence(u64);

    impl SequenceSource for FixedSequence {
        fn next(&self) -> u64 {
            self.0
        }
    }

    struct FakeStateSource {
        states: Mutex<VecDeque<ServiceState>>,
    }

    impl FakeStateSource {
        fn stable(state: ServiceState) -> Self {
            Self::scripted(vec![state.clone(), state])
        }

        fn scripted(states: Vec<ServiceState>) -> Self {
            Self {
                states: Mutex::new(states.into()),
            }
        }
    }

    impl StateSource for FakeStateSource {
        fn snapshot(&self) -> Result<ServiceState, DesktopCaptureError> {
            let mut states = self.states.lock().unwrap();
            if states.len() > 1 {
                Ok(states.pop_front().unwrap())
            } else {
                states.front().cloned().ok_or_else(|| {
                    DesktopCaptureError::new(
                        "desktop_capture_failed",
                        "fake state source exhausted",
                    )
                })
            }
        }
    }

    struct FakeFrameProvider {
        geometries: Mutex<VecDeque<Geometry>>,
        capture: Result<Vec<u8>, DesktopCaptureError>,
        capture_calls: AtomicU64,
    }

    impl FakeFrameProvider {
        fn new(geometry: Geometry, bytes: Vec<u8>) -> Self {
            Self::scripted(vec![geometry, geometry], Ok(bytes))
        }

        fn scripted(
            geometries: Vec<Geometry>,
            capture: Result<Vec<u8>, DesktopCaptureError>,
        ) -> Self {
            Self {
                geometries: Mutex::new(geometries.into()),
                capture,
                capture_calls: AtomicU64::new(0),
            }
        }

        fn capture_calls(&self) -> u64 {
            self.capture_calls.load(Ordering::Relaxed)
        }
    }

    impl FrameProvider for FakeFrameProvider {
        fn provider_version(&self) -> &str {
            "fake-v1"
        }

        fn geometry(&self, _display_name: &str) -> Result<Geometry, DesktopCaptureError> {
            let mut geometries = self.geometries.lock().unwrap();
            if geometries.len() > 1 {
                Ok(geometries.pop_front().unwrap())
            } else {
                geometries.front().copied().ok_or_else(|| {
                    DesktopCaptureError::new(
                        "desktop_geometry_unavailable",
                        "fake geometry source exhausted",
                    )
                })
            }
        }

        fn capture_png(
            &self,
            _display_name: &str,
            _max_bytes: u64,
        ) -> Result<Vec<u8>, DesktopCaptureError> {
            self.capture_calls.fetch_add(1, Ordering::Relaxed);
            self.capture.clone()
        }
    }

    #[test]
    fn capture_returns_a_bound_ephemeral_png_receipt() {
        let source = FakeStateSource::stable(ready_state());
        let provider = FakeFrameProvider::new(Geometry::new(1, 1), one_pixel_png());
        let dependencies = CaptureDependencies::new(
            &source,
            &provider,
            &FixedClock("2026-08-12T12:00:00Z"),
            &FixedSequence(7),
        );

        let result = capture_desktop_frame(
            DesktopCaptureRequest {
                browser_id: "browser-1".to_string(),
                session_name: Some("session-1".to_string()),
                max_bytes: DEFAULT_MAX_BYTES,
            },
            dependencies,
        )
        .expect("ready service-owned desktop must capture");

        assert_eq!(result.context.browser_id, "browser-1");
        assert_eq!(result.context.display_allocation_id, "display-1");
        assert_eq!(result.context.stream_id, "stream-1");
        assert_eq!(result.context.route_id, "route-1");
        assert_eq!(result.context.width, 1);
        assert_eq!(result.context.height, 1);
        assert_eq!(result.frame_receipt.sequence, 7);
        assert_eq!(result.frame_receipt.byte_length, result.image_bytes.len());
        assert_eq!(result.frame_receipt.retention, "ephemeral");
        assert!(!result.frame_receipt.persisted);
        assert_eq!(result.image_bytes, one_pixel_png());
        let expected_geometry_epoch = digest_text("browser-1\0display-1\0route-1\01\01\01000");
        let expected_context_id = format!(
            "desktop-context-{}",
            &digest_text(&format!(
                "browser-1\0stream-1\0route-1\0{expected_geometry_epoch}\02026-08-12T12:00:00Z"
            ))[..24]
        );
        assert_eq!(result.context.geometry_epoch, expected_geometry_epoch);
        assert_eq!(result.context.context_id, expected_context_id);
        let context = serde_json::to_value(&result.context).unwrap();
        let receipt = serde_json::to_value(&result.frame_receipt).unwrap();
        assert_eq!(context["schemaVersion"], "v1");
        assert!(context.get("readinessEvidence").is_some());
        assert!(context.get("readiness").is_none());
        assert_eq!(receipt["schemaVersion"], "v1");
        assert_eq!(receipt["sha256"], digest_bytes(&one_pixel_png()));
        assert!(receipt.get("contentSha256").is_none());
    }

    #[test]
    fn stream_projection_removes_response_only_pixels() {
        let data = json!({
            "context": {"contextId": "desktop-context-1"},
            "frameReceipt": {"frameId": "desktop-frame-1"},
            "imageBase64": "sensitive-pixels"
        });

        let broadcast = redact_desktop_capture_stream_result(&data);

        assert!(broadcast.get("imageBase64").is_none());
        assert_eq!(broadcast["imagePayload"], "response_only");
        assert_eq!(broadcast["frameReceipt"]["frameId"], "desktop-frame-1");
        assert_eq!(data["imageBase64"], "sensitive-pixels");
    }

    #[test]
    fn missing_or_ambiguous_workspace_fails_closed() {
        let mut missing = ready_state();
        missing.browsers.clear();
        assert_capture_code(missing, Some("session-1"), "desktop_workspace_not_found");

        let mut sessions = ready_state();
        sessions
            .browsers
            .get_mut("browser-1")
            .unwrap()
            .active_session_ids = vec!["session-1".to_string(), "session-2".to_string()];
        assert_capture_code(sessions, None, "desktop_workspace_ambiguous");

        let mut streams = ready_state();
        let mut duplicate = streams.browsers["browser-1"].view_streams[0].clone();
        duplicate.id = "stream-2".to_string();
        streams
            .browsers
            .get_mut("browser-1")
            .unwrap()
            .view_streams
            .push(duplicate);
        assert_capture_code(streams, Some("session-1"), "desktop_workspace_ambiguous");
    }

    #[test]
    fn route_and_display_identity_mismatches_fail_closed() {
        let mut route = ready_state();
        route
            .remote_view_routes
            .get_mut("route-1")
            .unwrap()
            .browser_id = Some("browser-2".to_string());
        assert_capture_code(route, Some("session-1"), "desktop_identity_mismatch");

        let mut display = ready_state();
        display.browsers.get_mut("browser-1").unwrap().display_name = Some(":202".to_string());
        assert_capture_code(display, Some("session-1"), "desktop_identity_mismatch");

        let mut route_key = ready_state();
        route_key.remote_view_routes.get_mut("route-1").unwrap().id = "route-2".to_string();
        assert_capture_code(route_key, Some("session-1"), "desktop_identity_mismatch");

        let mut display_key = ready_state();
        display_key
            .display_allocations
            .get_mut("display-1")
            .unwrap()
            .id = "display-2".to_string();
        assert_capture_code(display_key, Some("session-1"), "desktop_identity_mismatch");
    }

    #[test]
    fn route_and_display_readiness_fail_closed() {
        let mut route = ready_state();
        route.remote_view_routes.get_mut("route-1").unwrap().state = "pending".to_string();
        assert_capture_code(route, Some("session-1"), "desktop_route_not_ready");

        let mut display = ready_state();
        display
            .display_allocations
            .get_mut("display-1")
            .unwrap()
            .state = "allocating".to_string();
        assert_capture_code(display, Some("session-1"), "desktop_display_not_ready");

        let mut isolation = ready_state();
        isolation
            .display_allocations
            .get_mut("display-1")
            .unwrap()
            .display_isolation = "unknown".to_string();
        assert_capture_code(isolation, Some("session-1"), "desktop_identity_mismatch");

        let mut no_visible_content = ready_state();
        no_visible_content
            .browsers
            .get_mut("browser-1")
            .unwrap()
            .view_streams[0]
            .readiness = Some(json!({ "state": "ready" }));
        assert_capture_code(
            no_visible_content,
            Some("session-1"),
            "desktop_route_not_ready",
        );

        let mut exited = ready_state();
        exited.browsers.get_mut("browser-1").unwrap().health = BrowserHealth::ProcessExited;
        assert_capture_code(exited, Some("session-1"), "desktop_display_not_ready");
    }

    #[test]
    fn provider_failure_invalid_png_and_oversize_are_typed() {
        let state = ready_state();
        let source = FakeStateSource::stable(state.clone());
        let provider = FakeFrameProvider::scripted(
            vec![Geometry::new(1, 1)],
            Err(DesktopCaptureError::new(
                "desktop_capture_provider_unavailable",
                "provider absent",
            )),
        );
        assert_eq!(
            capture_with(&source, &provider, DEFAULT_MAX_BYTES)
                .unwrap_err()
                .code(),
            "desktop_capture_provider_unavailable"
        );

        let source = FakeStateSource::stable(state.clone());
        let invalid = FakeFrameProvider::new(Geometry::new(1, 1), b"not png".to_vec());
        assert_eq!(
            capture_with(&source, &invalid, DEFAULT_MAX_BYTES)
                .unwrap_err()
                .code(),
            "desktop_capture_failed"
        );

        let source = FakeStateSource::stable(state);
        let oversized = FakeFrameProvider::new(Geometry::new(1, 1), one_pixel_png());
        assert_eq!(
            capture_with(&source, &oversized, 8).unwrap_err().code(),
            "desktop_frame_too_large"
        );
    }

    #[test]
    fn post_capture_binding_or_geometry_drift_discards_frame() {
        let before = ready_state();
        let mut after = ready_state();
        after.remote_view_routes.get_mut("route-1").unwrap().state = "released".to_string();
        let source = FakeStateSource::scripted(vec![before.clone(), after]);
        let provider = FakeFrameProvider::new(Geometry::new(1, 1), one_pixel_png());
        assert_eq!(
            capture_with(&source, &provider, DEFAULT_MAX_BYTES)
                .unwrap_err()
                .code(),
            "desktop_route_not_ready"
        );

        let source = FakeStateSource::stable(before);
        let provider = FakeFrameProvider::scripted(
            vec![Geometry::new(1, 1), Geometry::new(2, 1)],
            Ok(one_pixel_png()),
        );
        assert_eq!(
            capture_with(&source, &provider, DEFAULT_MAX_BYTES)
                .unwrap_err()
                .code(),
            "desktop_geometry_changed"
        );
    }

    #[test]
    fn request_parser_rejects_caller_controlled_display_fields() {
        for key in [
            "displayName",
            "routeId",
            "providerUrl",
            "outputPath",
            "crop",
        ] {
            let mut request = json!({ "browserId": "browser-1" });
            request[key] = Value::String("caller-controlled".to_string());
            assert_eq!(
                parse_request(&request).unwrap_err().code(),
                "desktop_identity_mismatch"
            );
        }
    }

    #[test]
    fn xdpyinfo_geometry_parser_is_bounded_and_deterministic() {
        assert_eq!(
            parse_xdpyinfo_geometry(b"screen #0:\n  dimensions:    1920x1080 pixels\n").unwrap(),
            Geometry::new(1920, 1080)
        );
        assert_eq!(
            parse_xdpyinfo_geometry(b"dimensions: 0x1080 pixels")
                .unwrap_err()
                .code(),
            "desktop_geometry_unavailable"
        );
        assert_eq!(
            validate_geometry(Geometry::new(16_384, 16_384))
                .unwrap_err()
                .code(),
            "desktop_geometry_unavailable"
        );

        let drained = read_bounded_and_drain(Cursor::new(b"abcdefgh"), 4).unwrap();
        assert_eq!(drained.bytes, b"abcd");
        assert!(drained.overflowed);
    }

    #[test]
    fn bounded_provider_processes_report_missing_timeout_overflow_and_redact_stderr() {
        let missing = run_bounded_command(
            "agent-browser-desktop-capture-provider-does-not-exist",
            &[],
            64,
            Duration::from_millis(20),
        )
        .unwrap_err();
        assert_eq!(missing.code(), "desktop_capture_provider_unavailable");

        let executable = std::env::current_exe().unwrap();
        let executable = executable.to_str().unwrap();
        let timeout = run_bounded_command(
            executable,
            &[
                "native::desktop_capture::tests::desktop_capture_timeout_subprocess_fixture",
                "--exact",
            ],
            1024,
            Duration::from_millis(30),
        )
        .unwrap_err();
        assert_eq!(timeout.code(), "desktop_capture_failed");
        assert!(timeout.to_string().contains("timed out"));

        let overflow = run_bounded_command(
            executable,
            &[
                "native::desktop_capture::tests::desktop_capture_overflow_subprocess_fixture",
                "--exact",
            ],
            8,
            Duration::from_secs(2),
        )
        .unwrap_err();
        assert_eq!(overflow.code(), "desktop_frame_too_large");

        let failure = run_bounded_command(
            executable,
            &[
                "native::desktop_capture::tests::desktop_capture_stderr_subprocess_fixture",
                "--exact",
            ],
            1024,
            Duration::from_secs(2),
        )
        .unwrap_err();
        assert_eq!(failure.code(), "desktop_capture_failed");
        assert!(!failure.to_string().contains("sensitive-provider-stderr"));
    }

    #[test]
    fn desktop_capture_timeout_subprocess_fixture() {
        if subprocess_fixture_active("desktop_capture_timeout_subprocess_fixture") {
            thread::sleep(Duration::from_secs(1));
        }
    }

    #[test]
    fn desktop_capture_overflow_subprocess_fixture() {
        if subprocess_fixture_active("desktop_capture_overflow_subprocess_fixture") {
            std::io::stdout().write_all(&vec![b'x'; 128]).unwrap();
            std::io::stdout().flush().unwrap();
        }
    }

    #[test]
    fn desktop_capture_stderr_subprocess_fixture() {
        if subprocess_fixture_active("desktop_capture_stderr_subprocess_fixture") {
            eprintln!("sensitive-provider-stderr");
            panic!("fixture provider failure");
        }
    }

    fn subprocess_fixture_active(name: &str) -> bool {
        let arguments = std::env::args().collect::<Vec<_>>();
        arguments.iter().any(|argument| argument == "--exact")
            && arguments.iter().any(|argument| argument.ends_with(name))
    }

    fn capture_with(
        source: &dyn StateSource,
        provider: &dyn FrameProvider,
        max_bytes: u64,
    ) -> Result<DesktopCaptureResult, DesktopCaptureError> {
        capture_desktop_frame(
            DesktopCaptureRequest {
                browser_id: "browser-1".to_string(),
                session_name: Some("session-1".to_string()),
                max_bytes,
            },
            CaptureDependencies::new(
                source,
                provider,
                &FixedClock("2026-08-12T12:00:00Z"),
                &FixedSequence(7),
            ),
        )
    }

    fn assert_capture_code(state: ServiceState, session: Option<&str>, expected: &str) {
        let source = FakeStateSource::stable(state);
        let provider = FakeFrameProvider::new(Geometry::new(1, 1), one_pixel_png());
        let error = capture_desktop_frame(
            DesktopCaptureRequest {
                browser_id: "browser-1".to_string(),
                session_name: session.map(str::to_string),
                max_bytes: DEFAULT_MAX_BYTES,
            },
            CaptureDependencies::new(
                &source,
                &provider,
                &FixedClock("2026-08-12T12:00:00Z"),
                &FixedSequence(7),
            ),
        )
        .unwrap_err();
        assert_eq!(error.code(), expected, "{error}");
        assert_eq!(
            provider.capture_calls(),
            0,
            "provider ran before {expected}"
        );
    }

    fn ready_state() -> ServiceState {
        let stream = ViewStream {
            id: "stream-1".to_string(),
            provider: ViewStreamProvider::RdpGateway,
            route_id: Some("route-1".to_string()),
            display_allocation_id: Some("display-1".to_string()),
            readiness: Some(json!({
                "state": "ready",
                "displayContent": { "state": "browser_window_visible" }
            })),
            ..ViewStream::default()
        };
        let browser = BrowserProcess {
            id: "browser-1".to_string(),
            profile_id: Some("profile-1".to_string()),
            host: BrowserHost::RemoteHeaded,
            health: BrowserHealth::Ready,
            display_isolation: Some("private_virtual_display".to_string()),
            display_name: Some(":101".to_string()),
            display_allocation_id: Some("display-1".to_string()),
            view_streams: vec![stream],
            active_session_ids: vec!["session-1".to_string()],
            ..BrowserProcess::default()
        };
        let display = DisplayAllocation {
            id: "display-1".to_string(),
            display_name: Some(":101".to_string()),
            display_isolation: "private_virtual_display".to_string(),
            owner_browser_id: Some("browser-1".to_string()),
            owner_session_id: Some("session-1".to_string()),
            profile_id: Some("profile-1".to_string()),
            host: Some(BrowserHost::RemoteHeaded),
            state: "ready".to_string(),
            route_ids: vec!["route-1".to_string()],
            readiness: Some(json!({ "state": "ready" })),
            ..DisplayAllocation::default()
        };
        let route = RemoteViewRoute {
            id: "route-1".to_string(),
            provider: ViewStreamProvider::RdpGateway,
            display_allocation_id: Some("display-1".to_string()),
            browser_id: Some("browser-1".to_string()),
            session_id: Some("session-1".to_string()),
            state: "ready".to_string(),
            readiness: Some(json!({ "state": "ready" })),
            ..RemoteViewRoute::default()
        };
        ServiceState {
            browsers: BTreeMap::from([("browser-1".to_string(), browser)]),
            display_allocations: BTreeMap::from([("display-1".to_string(), display)]),
            remote_view_routes: BTreeMap::from([("route-1".to_string(), route)]),
            ..ServiceState::default()
        }
    }

    fn one_pixel_png() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([42, 84, 126, 255]));
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }
}
