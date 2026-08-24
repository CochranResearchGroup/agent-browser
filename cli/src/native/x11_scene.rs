//! X11 window-semantic evidence and reversible scene staging for one exact
//! service-owned browser process. Native window handles remain private to this
//! module and never enter service contracts, receipts, logs, or durable state.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct X11SceneEvidence {
    pub(crate) active_window_owned: bool,
    pub(crate) topmost_window_owned: bool,
    pub(crate) authorized_geometry: bool,
    pub(crate) capture_region_unoccluded: bool,
    pub(crate) frame_width: u32,
    pub(crate) frame_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    fn area(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    fn contains(self, other: Self) -> bool {
        let self_right = i64::from(self.x) + i64::from(self.width);
        let self_bottom = i64::from(self.y) + i64::from(self.height);
        let other_right = i64::from(other.x) + i64::from(other.width);
        let other_bottom = i64::from(other.y) + i64::from(other.height);
        i64::from(self.x) <= i64::from(other.x)
            && i64::from(self.y) <= i64::from(other.y)
            && self_right >= other_right
            && self_bottom >= other_bottom
    }

    fn intersects(self, other: Self) -> bool {
        let self_right = i64::from(self.x) + i64::from(self.width);
        let self_bottom = i64::from(self.y) + i64::from(self.height);
        let other_right = i64::from(other.x) + i64::from(other.width);
        let other_bottom = i64::from(other.y) + i64::from(other.height);
        i64::from(self.x) < other_right
            && i64::from(other.x) < self_right
            && i64::from(self.y) < other_bottom
            && i64::from(other.y) < self_bottom
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowSnapshot {
    owned: bool,
    rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OwnedWindowSnapshot {
    window: u64,
    rect: Rect,
    maximized_horizontal: bool,
    maximized_vertical: bool,
}

/// Ephemeral native state required to reverse one scene-staging transaction.
/// This value must remain process-local because it contains provider handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct X11SceneSnapshot {
    pid: u32,
    display_name: String,
    target_window: u64,
    active_window: Option<u64>,
    stacking_bottom_to_top: Vec<u64>,
    owned_windows: Vec<OwnedWindowSnapshot>,
}

fn evaluate_scene(
    active_window_owned: bool,
    stacking_authoritative: bool,
    capture_region: Rect,
    windows_bottom_to_top: &[WindowSnapshot],
) -> Result<X11SceneEvidence, String> {
    if capture_region.width == 0 || capture_region.height == 0 {
        return Err("X11 capture region has zero area".to_string());
    }
    let (base_index, base) = windows_bottom_to_top
        .iter()
        .enumerate()
        .filter(|(_, window)| window.owned && window.rect.intersects(capture_region))
        .max_by_key(|(_, window)| window.rect.area())
        .ok_or_else(|| "No viewable X11 window belongs to the browser PID".to_string())?;
    let topmost_window_owned = stacking_authoritative
        && windows_bottom_to_top
            .iter()
            .rev()
            .find(|window| window.rect.intersects(capture_region))
            .is_some_and(|window| window.owned);
    let capture_region_unoccluded = stacking_authoritative
        && windows_bottom_to_top
            .iter()
            .skip(base_index + 1)
            .filter(|window| window.rect.intersects(capture_region))
            .all(|window| window.owned);

    Ok(X11SceneEvidence {
        active_window_owned,
        topmost_window_owned,
        authorized_geometry: base.rect.contains(capture_region),
        capture_region_unoccluded,
        frame_width: capture_region.width,
        frame_height: capture_region.height,
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn observe_browser_scene(
    pid: u32,
    display_name: &str,
) -> Result<X11SceneEvidence, String> {
    linux::observe_browser_scene(pid, display_name)
}

#[cfg(target_os = "linux")]
pub(crate) fn snapshot_browser_scene(
    pid: u32,
    display_name: &str,
) -> Result<X11SceneSnapshot, String> {
    linux::snapshot_browser_scene(pid, display_name)
}

#[cfg(target_os = "linux")]
pub(crate) fn stage_browser_scene(snapshot: &X11SceneSnapshot) -> Result<(), String> {
    linux::stage_browser_scene(snapshot)
}

#[cfg(target_os = "linux")]
pub(crate) fn restore_browser_scene(snapshot: &X11SceneSnapshot) -> Result<(), String> {
    linux::restore_browser_scene(snapshot)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn observe_browser_scene(
    _pid: u32,
    _display_name: &str,
) -> Result<X11SceneEvidence, String> {
    Err("read-only X11 scene observation is unavailable on this platform".to_string())
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn snapshot_browser_scene(
    _pid: u32,
    _display_name: &str,
) -> Result<X11SceneSnapshot, String> {
    Err("X11 scene snapshot is unavailable on this platform".to_string())
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn stage_browser_scene(_snapshot: &X11SceneSnapshot) -> Result<(), String> {
    Err("X11 scene staging is unavailable on this platform".to_string())
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn restore_browser_scene(_snapshot: &X11SceneSnapshot) -> Result<(), String> {
    Err("X11 scene restoration is unavailable on this platform".to_string())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{
        evaluate_scene, OwnedWindowSnapshot, Rect, WindowSnapshot, X11SceneEvidence,
        X11SceneSnapshot,
    };
    use libc::{c_char, c_int, c_long, c_short, c_uchar, c_uint, c_ulong, c_void};
    use std::ffi::CString;
    use std::ptr;
    use std::thread;
    use std::time::Duration;

    type Display = c_void;
    type Visual = c_void;
    type Screen = c_void;
    type Window = c_ulong;
    type Atom = c_ulong;
    type Colormap = c_ulong;

    const FALSE: c_int = 0;
    const ANY_PROPERTY_TYPE: Atom = 0;
    const IS_VIEWABLE: c_int = 2;
    const CLIENT_MESSAGE: c_int = 33;
    const SUBSTRUCTURE_NOTIFY_MASK: c_long = 1 << 19;
    const SUBSTRUCTURE_REDIRECT_MASK: c_long = 1 << 20;
    const NET_WM_STATE_REMOVE: c_long = 0;
    const NET_WM_STATE_ADD: c_long = 1;
    const CURRENT_TIME: c_long = 0;
    const STAGE_VERIFY_ATTEMPTS: usize = 50;
    const RESTORE_VERIFY_ATTEMPTS: usize = 50;
    const VERIFY_INTERVAL_MS: u64 = 10;

    #[repr(C)]
    struct XWindowAttributes {
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
        border_width: c_int,
        depth: c_int,
        visual: *mut Visual,
        root: Window,
        class: c_int,
        bit_gravity: c_int,
        win_gravity: c_int,
        backing_store: c_int,
        backing_planes: c_ulong,
        backing_pixel: c_ulong,
        save_under: c_int,
        colormap: Colormap,
        map_installed: c_int,
        map_state: c_int,
        all_event_masks: c_long,
        your_event_mask: c_long,
        do_not_propagate_mask: c_long,
        override_redirect: c_int,
        screen: *mut Screen,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    union ClientMessageData {
        b: [c_char; 20],
        s: [c_short; 10],
        l: [c_long; 5],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct XClientMessageEvent {
        type_: c_int,
        serial: c_ulong,
        send_event: c_int,
        display: *mut Display,
        window: Window,
        message_type: Atom,
        format: c_int,
        data: ClientMessageData,
    }

    #[repr(C)]
    union XEvent {
        xclient: XClientMessageEvent,
        pad: [c_long; 24],
    }

    type XOpenDisplayFn = unsafe extern "C" fn(*const c_char) -> *mut Display;
    type XCloseDisplayFn = unsafe extern "C" fn(*mut Display) -> c_int;
    type XDefaultRootWindowFn = unsafe extern "C" fn(*mut Display) -> Window;
    type XInternAtomFn = unsafe extern "C" fn(*mut Display, *const c_char, c_int) -> Atom;
    type XGetWindowPropertyFn = unsafe extern "C" fn(
        *mut Display,
        Window,
        Atom,
        c_long,
        c_long,
        c_int,
        Atom,
        *mut Atom,
        *mut c_int,
        *mut c_ulong,
        *mut c_ulong,
        *mut *mut c_uchar,
    ) -> c_int;
    type XGetWindowAttributesFn =
        unsafe extern "C" fn(*mut Display, Window, *mut XWindowAttributes) -> c_int;
    type XTranslateCoordinatesFn = unsafe extern "C" fn(
        *mut Display,
        Window,
        Window,
        c_int,
        c_int,
        *mut c_int,
        *mut c_int,
        *mut Window,
    ) -> c_int;
    type XQueryTreeFn = unsafe extern "C" fn(
        *mut Display,
        Window,
        *mut Window,
        *mut Window,
        *mut *mut Window,
        *mut c_uint,
    ) -> c_int;
    type XSendEventFn =
        unsafe extern "C" fn(*mut Display, Window, c_int, c_long, *mut XEvent) -> c_int;
    type XMoveResizeWindowFn =
        unsafe extern "C" fn(*mut Display, Window, c_int, c_int, c_uint, c_uint) -> c_int;
    type XRaiseWindowFn = unsafe extern "C" fn(*mut Display, Window) -> c_int;
    type XRestackWindowsFn = unsafe extern "C" fn(*mut Display, *mut Window, c_int) -> c_int;
    type XSyncFn = unsafe extern "C" fn(*mut Display, c_int) -> c_int;
    type XFreeFn = unsafe extern "C" fn(*mut c_void) -> c_int;

    struct X11 {
        handle: *mut c_void,
        open_display: XOpenDisplayFn,
        close_display: XCloseDisplayFn,
        default_root_window: XDefaultRootWindowFn,
        intern_atom: XInternAtomFn,
        get_window_property: XGetWindowPropertyFn,
        get_window_attributes: XGetWindowAttributesFn,
        translate_coordinates: XTranslateCoordinatesFn,
        query_tree: XQueryTreeFn,
        send_event: XSendEventFn,
        move_resize_window: XMoveResizeWindowFn,
        raise_window: XRaiseWindowFn,
        restack_windows: XRestackWindowsFn,
        sync: XSyncFn,
        free: XFreeFn,
    }

    impl X11 {
        fn load() -> Result<Self, String> {
            let mut last_error = None;
            for name in ["libX11.so.6", "libX11.so"] {
                let library_name = CString::new(name).expect("static library name");
                let handle = unsafe {
                    libc::dlopen(library_name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL)
                };
                if handle.is_null() {
                    last_error = Some(dl_error());
                    continue;
                }
                return unsafe {
                    Ok(Self {
                        handle,
                        open_display: symbol(handle, "XOpenDisplay")?,
                        close_display: symbol(handle, "XCloseDisplay")?,
                        default_root_window: symbol(handle, "XDefaultRootWindow")?,
                        intern_atom: symbol(handle, "XInternAtom")?,
                        get_window_property: symbol(handle, "XGetWindowProperty")?,
                        get_window_attributes: symbol(handle, "XGetWindowAttributes")?,
                        translate_coordinates: symbol(handle, "XTranslateCoordinates")?,
                        query_tree: symbol(handle, "XQueryTree")?,
                        send_event: symbol(handle, "XSendEvent")?,
                        move_resize_window: symbol(handle, "XMoveResizeWindow")?,
                        raise_window: symbol(handle, "XRaiseWindow")?,
                        restack_windows: symbol(handle, "XRestackWindows")?,
                        sync: symbol(handle, "XSync")?,
                        free: symbol(handle, "XFree")?,
                    })
                };
            }
            Err(format!(
                "X11 scene observation requires libX11 at runtime{}",
                last_error
                    .map(|error| format!(": {error}"))
                    .unwrap_or_default()
            ))
        }
    }

    impl Drop for X11 {
        fn drop(&mut self) {
            unsafe {
                libc::dlclose(self.handle);
            }
        }
    }

    pub(super) fn observe_browser_scene(
        pid: u32,
        display_name: &str,
    ) -> Result<X11SceneEvidence, String> {
        let display_c = CString::new(display_name)
            .map_err(|_| "X11 display contains an interior NUL byte".to_string())?;
        let x11 = X11::load()?;
        unsafe {
            let display = (x11.open_display)(display_c.as_ptr());
            if display.is_null() {
                return Err(format!("Failed to open X11 display {display_name}"));
            }
            let result = observe_on_display(&x11, display, pid);
            (x11.close_display)(display);
            result
        }
    }

    pub(super) fn snapshot_browser_scene(
        pid: u32,
        display_name: &str,
    ) -> Result<X11SceneSnapshot, String> {
        with_display(display_name, |x11, display| unsafe {
            snapshot_on_display(x11, display, pid, display_name)
        })
    }

    pub(super) fn stage_browser_scene(snapshot: &X11SceneSnapshot) -> Result<(), String> {
        with_display(&snapshot.display_name, |x11, display| unsafe {
            let root = (x11.default_root_window)(display);
            let pid_atom = intern_atom(x11, display, "_NET_WM_PID")?;
            let active_atom = intern_atom(x11, display, "_NET_ACTIVE_WINDOW")?;
            let workarea_atom = intern_atom(x11, display, "_NET_WORKAREA")?;
            let wm_state_atom = intern_atom(x11, display, "_NET_WM_STATE")?;
            let maximized_horizontal_atom =
                intern_atom(x11, display, "_NET_WM_STATE_MAXIMIZED_HORZ")?;
            let maximized_vertical_atom =
                intern_atom(x11, display, "_NET_WM_STATE_MAXIMIZED_VERT")?;
            let target = Window::try_from(snapshot.target_window)
                .map_err(|_| "X11 staged target window identity is invalid".to_string())?;
            if window_pid(x11, display, target, pid_atom) != Some(snapshot.pid) {
                return Err("X11 staged target no longer belongs to the browser PID".to_string());
            }
            let root_rect = window_rect(x11, display, root, root)
                .ok_or_else(|| "X11 root window geometry is unavailable".to_string())?;
            let workarea = property_values(x11, display, root, workarea_atom, 4)
                .and_then(|values| rect_from_workarea(&values))
                .unwrap_or(root_rect);

            (x11.move_resize_window)(
                display,
                target,
                workarea.x,
                workarea.y,
                workarea.width,
                workarea.height,
            );
            send_wm_state(
                x11,
                display,
                root,
                target,
                wm_state_atom,
                NET_WM_STATE_ADD,
                maximized_horizontal_atom,
                maximized_vertical_atom,
            )?;
            (x11.raise_window)(display, target);
            send_active_window(x11, display, root, target, active_atom)?;
            (x11.sync)(display, FALSE);

            for _ in 0..STAGE_VERIFY_ATTEMPTS {
                let evidence = observe_on_display(x11, display, snapshot.pid)?;
                let active = property_values(x11, display, root, active_atom, 1)
                    .and_then(|values| values.first().copied());
                if evidence.active_window_owned
                    && evidence.topmost_window_owned
                    && evidence.authorized_geometry
                    && evidence.capture_region_unoccluded
                    && active == Some(target)
                {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(VERIFY_INTERVAL_MS));
                (x11.sync)(display, FALSE);
            }
            Err("X11 scene did not reach capture-ready staging before the deadline".to_string())
        })
    }

    pub(super) fn restore_browser_scene(snapshot: &X11SceneSnapshot) -> Result<(), String> {
        with_display(&snapshot.display_name, |x11, display| unsafe {
            let root = (x11.default_root_window)(display);
            let pid_atom = intern_atom(x11, display, "_NET_WM_PID")?;
            let active_atom = intern_atom(x11, display, "_NET_ACTIVE_WINDOW")?;
            let stacking_atom = intern_atom(x11, display, "_NET_CLIENT_LIST_STACKING")?;
            let wm_state_atom = intern_atom(x11, display, "_NET_WM_STATE")?;
            let maximized_horizontal_atom =
                intern_atom(x11, display, "_NET_WM_STATE_MAXIMIZED_HORZ")?;
            let maximized_vertical_atom =
                intern_atom(x11, display, "_NET_WM_STATE_MAXIMIZED_VERT")?;

            for owned in &snapshot.owned_windows {
                let window = Window::try_from(owned.window)
                    .map_err(|_| "X11 restoration window identity is invalid".to_string())?;
                if window_pid(x11, display, window, pid_atom) != Some(snapshot.pid) {
                    return Err(
                        "X11 restoration window no longer belongs to the browser PID".to_string(),
                    );
                }
                set_wm_state_flag(
                    x11,
                    display,
                    root,
                    window,
                    wm_state_atom,
                    maximized_horizontal_atom,
                    owned.maximized_horizontal,
                )?;
                set_wm_state_flag(
                    x11,
                    display,
                    root,
                    window,
                    wm_state_atom,
                    maximized_vertical_atom,
                    owned.maximized_vertical,
                )?;
                (x11.move_resize_window)(
                    display,
                    window,
                    owned.rect.x,
                    owned.rect.y,
                    owned.rect.width,
                    owned.rect.height,
                );
            }

            let mut top_to_bottom = snapshot
                .stacking_bottom_to_top
                .iter()
                .rev()
                .map(|window| Window::try_from(*window))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "X11 restoration stacking identity is invalid".to_string())?;
            if !top_to_bottom.is_empty() {
                let count = c_int::try_from(top_to_bottom.len())
                    .map_err(|_| "X11 restoration stacking inventory is too large".to_string())?;
                (x11.restack_windows)(display, top_to_bottom.as_mut_ptr(), count);
            }
            let active = snapshot
                .active_window
                .ok_or_else(|| "X11 restoration has no recorded active window".to_string())?;
            send_active_window(
                x11,
                display,
                root,
                Window::try_from(active)
                    .map_err(|_| "X11 restoration active window is invalid".to_string())?,
                active_atom,
            )?;
            (x11.sync)(display, FALSE);

            for _ in 0..RESTORE_VERIFY_ATTEMPTS {
                if restoration_matches(
                    x11,
                    display,
                    root,
                    stacking_atom,
                    active_atom,
                    wm_state_atom,
                    maximized_horizontal_atom,
                    maximized_vertical_atom,
                    snapshot,
                ) {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(VERIFY_INTERVAL_MS));
                (x11.sync)(display, FALSE);
            }
            Err("X11 scene restoration could not be verified exactly".to_string())
        })
    }

    fn with_display<T>(
        display_name: &str,
        operation: impl FnOnce(&X11, *mut Display) -> Result<T, String>,
    ) -> Result<T, String> {
        let display_c = CString::new(display_name)
            .map_err(|_| "X11 display contains an interior NUL byte".to_string())?;
        let x11 = X11::load()?;
        unsafe {
            let display = (x11.open_display)(display_c.as_ptr());
            if display.is_null() {
                return Err(format!("Failed to open X11 display {display_name}"));
            }
            let result = operation(&x11, display);
            (x11.close_display)(display);
            result
        }
    }

    unsafe fn snapshot_on_display(
        x11: &X11,
        display: *mut Display,
        pid: u32,
        display_name: &str,
    ) -> Result<X11SceneSnapshot, String> {
        let root = (x11.default_root_window)(display);
        let pid_atom = intern_atom(x11, display, "_NET_WM_PID")?;
        let stacking_atom = intern_atom(x11, display, "_NET_CLIENT_LIST_STACKING")?;
        let active_atom = intern_atom(x11, display, "_NET_ACTIVE_WINDOW")?;
        let workarea_atom = intern_atom(x11, display, "_NET_WORKAREA")?;
        let wm_state_atom = intern_atom(x11, display, "_NET_WM_STATE")?;
        let maximized_horizontal_atom = intern_atom(x11, display, "_NET_WM_STATE_MAXIMIZED_HORZ")?;
        let maximized_vertical_atom = intern_atom(x11, display, "_NET_WM_STATE_MAXIMIZED_VERT")?;
        let root_rect = window_rect(x11, display, root, root)
            .ok_or_else(|| "X11 root window geometry is unavailable".to_string())?;
        let capture_region = property_values(x11, display, root, workarea_atom, 4)
            .and_then(|values| rect_from_workarea(&values))
            .unwrap_or(root_rect);
        let stacking = property_values(x11, display, root, stacking_atom, 4096)
            .ok_or_else(|| "X11 authoritative stacking inventory is unavailable".to_string())?;
        let active_window = property_values(x11, display, root, active_atom, 1)
            .and_then(|values| values.first().copied());
        if active_window.is_none() {
            return Err("X11 active window is unavailable for reversible staging".to_string());
        }
        let mut owned_windows = Vec::new();
        for window in &stacking {
            if window_pid(x11, display, *window, pid_atom) != Some(pid) {
                continue;
            }
            let Some(rect) = window_rect(x11, display, *window, root) else {
                continue;
            };
            let states =
                property_values(x11, display, *window, wm_state_atom, 64).unwrap_or_default();
            owned_windows.push(OwnedWindowSnapshot {
                window: *window,
                rect,
                maximized_horizontal: states.contains(&maximized_horizontal_atom),
                maximized_vertical: states.contains(&maximized_vertical_atom),
            });
        }
        let target_window = owned_windows
            .iter()
            .filter(|window| window.rect.intersects(capture_region))
            .max_by_key(|window| window.rect.area())
            .map(|window| window.window)
            .ok_or_else(|| "No viewable X11 window belongs to the browser PID".to_string())?;
        Ok(X11SceneSnapshot {
            pid,
            display_name: display_name.to_string(),
            target_window,
            active_window,
            stacking_bottom_to_top: stacking,
            owned_windows,
        })
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn restoration_matches(
        x11: &X11,
        display: *mut Display,
        root: Window,
        stacking_atom: Atom,
        active_atom: Atom,
        wm_state_atom: Atom,
        maximized_horizontal_atom: Atom,
        maximized_vertical_atom: Atom,
        snapshot: &X11SceneSnapshot,
    ) -> bool {
        let active = property_values(x11, display, root, active_atom, 1)
            .and_then(|values| values.first().copied());
        if active != snapshot.active_window {
            return false;
        }
        let Some(stacking) = property_values(x11, display, root, stacking_atom, 4096) else {
            return false;
        };
        let recorded = snapshot
            .stacking_bottom_to_top
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let relative = stacking
            .into_iter()
            .filter(|window| recorded.contains(window))
            .collect::<Vec<_>>();
        if relative != snapshot.stacking_bottom_to_top {
            return false;
        }
        snapshot.owned_windows.iter().all(|owned| {
            let window = owned.window as Window;
            let Some(rect) = window_rect(x11, display, window, root) else {
                return false;
            };
            let states =
                property_values(x11, display, window, wm_state_atom, 64).unwrap_or_default();
            rect == owned.rect
                && states.contains(&maximized_horizontal_atom) == owned.maximized_horizontal
                && states.contains(&maximized_vertical_atom) == owned.maximized_vertical
        })
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn send_wm_state(
        x11: &X11,
        display: *mut Display,
        root: Window,
        window: Window,
        wm_state_atom: Atom,
        action: c_long,
        first_state: Atom,
        second_state: Atom,
    ) -> Result<(), String> {
        let mut event = XEvent {
            xclient: XClientMessageEvent {
                type_: CLIENT_MESSAGE,
                serial: 0,
                send_event: 1,
                display,
                window,
                message_type: wm_state_atom,
                format: 32,
                data: ClientMessageData {
                    l: [action, first_state as c_long, second_state as c_long, 2, 0],
                },
            },
        };
        let sent = (x11.send_event)(
            display,
            root,
            FALSE,
            SUBSTRUCTURE_REDIRECT_MASK | SUBSTRUCTURE_NOTIFY_MASK,
            &mut event,
        );
        (sent != 0)
            .then_some(())
            .ok_or_else(|| "X11 window manager rejected the state request".to_string())
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn set_wm_state_flag(
        x11: &X11,
        display: *mut Display,
        root: Window,
        window: Window,
        wm_state_atom: Atom,
        state_atom: Atom,
        enabled: bool,
    ) -> Result<(), String> {
        send_wm_state(
            x11,
            display,
            root,
            window,
            wm_state_atom,
            if enabled {
                NET_WM_STATE_ADD
            } else {
                NET_WM_STATE_REMOVE
            },
            state_atom,
            0,
        )
    }

    unsafe fn send_active_window(
        x11: &X11,
        display: *mut Display,
        root: Window,
        window: Window,
        active_atom: Atom,
    ) -> Result<(), String> {
        let mut event = XEvent {
            xclient: XClientMessageEvent {
                type_: CLIENT_MESSAGE,
                serial: 0,
                send_event: 1,
                display,
                window,
                message_type: active_atom,
                format: 32,
                data: ClientMessageData {
                    l: [2, CURRENT_TIME, 0, 0, 0],
                },
            },
        };
        let sent = (x11.send_event)(
            display,
            root,
            FALSE,
            SUBSTRUCTURE_REDIRECT_MASK | SUBSTRUCTURE_NOTIFY_MASK,
            &mut event,
        );
        (sent != 0)
            .then_some(())
            .ok_or_else(|| "X11 window manager rejected the activation request".to_string())
    }

    unsafe fn observe_on_display(
        x11: &X11,
        display: *mut Display,
        pid: u32,
    ) -> Result<X11SceneEvidence, String> {
        let root = (x11.default_root_window)(display);
        let pid_atom = intern_atom(x11, display, "_NET_WM_PID")?;
        let stacking_atom = intern_atom(x11, display, "_NET_CLIENT_LIST_STACKING")?;
        let active_atom = intern_atom(x11, display, "_NET_ACTIVE_WINDOW")?;
        let workarea_atom = intern_atom(x11, display, "_NET_WORKAREA")?;
        let root_rect = window_rect(x11, display, root, root)
            .ok_or_else(|| "X11 root window geometry is unavailable".to_string())?;
        let capture_region = property_values(x11, display, root, workarea_atom, 4)
            .and_then(|values| rect_from_workarea(&values))
            .unwrap_or(root_rect);
        let active_window_owned = property_values(x11, display, root, active_atom, 1)
            .and_then(|values| values.first().copied())
            .is_some_and(|window| window_pid(x11, display, window, pid_atom) == Some(pid));

        let stacking = property_values(x11, display, root, stacking_atom, 4096);
        let stacking_authoritative = stacking.is_some();
        let candidates = stacking.unwrap_or_else(|| collect_window_tree(x11, display, root));
        let windows = candidates
            .into_iter()
            .filter_map(|window| {
                let rect = window_rect(x11, display, window, root)?;
                Some(WindowSnapshot {
                    owned: window_pid(x11, display, window, pid_atom) == Some(pid),
                    rect,
                })
            })
            .collect::<Vec<_>>();
        evaluate_scene(
            active_window_owned,
            stacking_authoritative,
            capture_region,
            &windows,
        )
    }

    unsafe fn intern_atom(x11: &X11, display: *mut Display, name: &str) -> Result<Atom, String> {
        let c_name = CString::new(name).map_err(|_| format!("Invalid X11 atom name {name}"))?;
        let atom = (x11.intern_atom)(display, c_name.as_ptr(), FALSE);
        (atom != 0)
            .then_some(atom)
            .ok_or_else(|| format!("X11 atom {name} is unavailable"))
    }

    unsafe fn property_values(
        x11: &X11,
        display: *mut Display,
        window: Window,
        property: Atom,
        maximum_items: c_long,
    ) -> Option<Vec<c_ulong>> {
        let mut actual_type: Atom = 0;
        let mut actual_format: c_int = 0;
        let mut item_count: c_ulong = 0;
        let mut bytes_after: c_ulong = 0;
        let mut property_data: *mut c_uchar = ptr::null_mut();
        let status = (x11.get_window_property)(
            display,
            window,
            property,
            0,
            maximum_items,
            FALSE,
            ANY_PROPERTY_TYPE,
            &mut actual_type,
            &mut actual_format,
            &mut item_count,
            &mut bytes_after,
            &mut property_data,
        );
        if status != 0 || property_data.is_null() || actual_format != 32 || item_count == 0 {
            if !property_data.is_null() {
                (x11.free)(property_data as *mut c_void);
            }
            return None;
        }
        let values =
            std::slice::from_raw_parts(property_data as *const c_ulong, item_count as usize)
                .to_vec();
        (x11.free)(property_data as *mut c_void);
        Some(values)
    }

    unsafe fn window_pid(
        x11: &X11,
        display: *mut Display,
        window: Window,
        pid_atom: Atom,
    ) -> Option<u32> {
        property_values(x11, display, window, pid_atom, 1)
            .and_then(|values| values.first().copied())
            .and_then(|value| u32::try_from(value).ok())
    }

    unsafe fn window_rect(
        x11: &X11,
        display: *mut Display,
        window: Window,
        root: Window,
    ) -> Option<Rect> {
        let mut attributes: XWindowAttributes = std::mem::zeroed();
        if (x11.get_window_attributes)(display, window, &mut attributes) == 0
            || attributes.map_state != IS_VIEWABLE
            || attributes.width <= 0
            || attributes.height <= 0
        {
            return None;
        }
        let mut root_x = 0;
        let mut root_y = 0;
        let mut child = 0;
        if (x11.translate_coordinates)(
            display,
            window,
            root,
            0,
            0,
            &mut root_x,
            &mut root_y,
            &mut child,
        ) == 0
        {
            return None;
        }
        Some(Rect {
            x: root_x,
            y: root_y,
            width: u32::try_from(attributes.width).ok()?,
            height: u32::try_from(attributes.height).ok()?,
        })
    }

    fn rect_from_workarea(values: &[c_ulong]) -> Option<Rect> {
        let [x, y, width, height, ..] = values else {
            return None;
        };
        Some(Rect {
            x: i32::try_from(*x).ok()?,
            y: i32::try_from(*y).ok()?,
            width: u32::try_from(*width).ok()?,
            height: u32::try_from(*height).ok()?,
        })
    }

    unsafe fn collect_window_tree(x11: &X11, display: *mut Display, root: Window) -> Vec<Window> {
        let mut windows = Vec::new();
        collect_window_tree_into(x11, display, root, &mut windows);
        windows
    }

    unsafe fn collect_window_tree_into(
        x11: &X11,
        display: *mut Display,
        window: Window,
        windows: &mut Vec<Window>,
    ) {
        let mut root_return = 0;
        let mut parent_return = 0;
        let mut children: *mut Window = ptr::null_mut();
        let mut child_count = 0;
        let ok = (x11.query_tree)(
            display,
            window,
            &mut root_return,
            &mut parent_return,
            &mut children,
            &mut child_count,
        );
        if ok == 0 || children.is_null() {
            return;
        }
        let child_slice = std::slice::from_raw_parts(children, child_count as usize);
        for child in child_slice {
            windows.push(*child);
            collect_window_tree_into(x11, display, *child, windows);
        }
        (x11.free)(children as *mut c_void);
    }

    unsafe fn symbol<T: Copy>(handle: *mut c_void, name: &str) -> Result<T, String> {
        let symbol_name = CString::new(name).expect("static symbol name");
        let pointer = libc::dlsym(handle, symbol_name.as_ptr());
        if pointer.is_null() {
            return Err(format!("libX11 symbol {name} is unavailable"));
        }
        Ok(std::mem::transmute_copy(&pointer))
    }

    fn dl_error() -> String {
        let error = unsafe { libc::dlerror() };
        if error.is_null() {
            return "unknown dlopen error".to_string();
        }
        unsafe { std::ffi::CStr::from_ptr(error) }
            .to_string_lossy()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISPLAY: Rect = Rect {
        x: 0,
        y: 0,
        width: 1280,
        height: 720,
    };

    fn window(owned: bool, rect: Rect) -> WindowSnapshot {
        WindowSnapshot { owned, rect }
    }

    #[test]
    fn exact_owned_full_region_scene_is_capture_ready() {
        let evidence = evaluate_scene(true, true, DISPLAY, &[window(true, DISPLAY)]).unwrap();

        assert!(evidence.active_window_owned);
        assert!(evidence.topmost_window_owned);
        assert!(evidence.authorized_geometry);
        assert!(evidence.capture_region_unoccluded);
        assert_eq!(evidence.frame_width, 1280);
        assert_eq!(evidence.frame_height, 720);
    }

    #[test]
    fn owned_popup_above_maximized_browser_is_allowed() {
        let popup = Rect {
            x: 900,
            y: 50,
            width: 300,
            height: 400,
        };
        let evidence = evaluate_scene(
            true,
            true,
            DISPLAY,
            &[window(true, DISPLAY), window(true, popup)],
        )
        .unwrap();

        assert!(evidence.topmost_window_owned);
        assert!(evidence.authorized_geometry);
        assert!(evidence.capture_region_unoccluded);
    }

    #[test]
    fn unowned_occluder_fails_closed() {
        let occluder = Rect {
            x: 10,
            y: 10,
            width: 100,
            height: 100,
        };
        let evidence = evaluate_scene(
            false,
            true,
            DISPLAY,
            &[window(true, DISPLAY), window(false, occluder)],
        )
        .unwrap();

        assert!(!evidence.active_window_owned);
        assert!(!evidence.topmost_window_owned);
        assert!(!evidence.capture_region_unoccluded);
    }

    #[test]
    fn non_authoritative_stacking_fails_topmost_and_occlusion_proofs() {
        let evidence = evaluate_scene(true, false, DISPLAY, &[window(true, DISPLAY)]).unwrap();

        assert!(evidence.active_window_owned);
        assert!(!evidence.topmost_window_owned);
        assert!(!evidence.capture_region_unoccluded);
    }

    #[test]
    fn non_maximized_browser_fails_geometry_proof() {
        let browser = Rect {
            x: 0,
            y: 0,
            width: 1000,
            height: 700,
        };
        let evidence = evaluate_scene(true, true, DISPLAY, &[window(true, browser)]).unwrap();

        assert!(!evidence.authorized_geometry);
    }
}
