//! Read-only X11 window-semantic evidence for a service-owned browser process.
//!
//! This module deliberately exposes no window handles and performs no focus,
//! stacking, geometry, or input mutation. It converts an exact browser PID and
//! display binding into capture-readiness facts for the desktop evidence
//! coordinator.

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

#[cfg(not(target_os = "linux"))]
pub(crate) fn observe_browser_scene(
    _pid: u32,
    _display_name: &str,
) -> Result<X11SceneEvidence, String> {
    Err("read-only X11 scene observation is unavailable on this platform".to_string())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{evaluate_scene, Rect, WindowSnapshot, X11SceneEvidence};
    use libc::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_void};
    use std::ffi::CString;
    use std::ptr;

    type Display = c_void;
    type Visual = c_void;
    type Screen = c_void;
    type Window = c_ulong;
    type Atom = c_ulong;
    type Colormap = c_ulong;

    const FALSE: c_int = 0;
    const ANY_PROPERTY_TYPE: Atom = 0;
    const IS_VIEWABLE: c_int = 2;

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
