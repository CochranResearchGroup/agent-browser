//! Native XTEST adapter for the controlled desktop input provider.
//!
//! The adapter dynamically loads libX11 and libXtst so unsupported platforms
//! continue to compile and fail closed at runtime. It accepts only the closed
//! event schema owned by the parent module.

use super::{ClosedX11Sink, ControlledX11Event, DesktopInputProviderError};

#[derive(Debug)]
pub(crate) struct XTestSink {
    display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct XTestSceneProbe {
    pub(crate) pointer_x: i32,
    pub(crate) pointer_y: i32,
    pub(crate) controlled_fixture_focused: bool,
}

impl XTestSink {
    pub(crate) fn new(display_name: impl Into<String>) -> Result<Self, DesktopInputProviderError> {
        let display_name = display_name.into();
        if display_name.trim().is_empty() || display_name.as_bytes().contains(&0) {
            return Err(DesktopInputProviderError::new(
                "desktop_input_display_authority_invalid",
            ));
        }
        Ok(Self { display_name })
    }

    pub(crate) fn probe(&self) -> Result<XTestSceneProbe, DesktopInputProviderError> {
        platform::probe(&self.display_name)
    }
}

impl ClosedX11Sink for XTestSink {
    fn emit(&mut self, event: &ControlledX11Event) -> Result<String, DesktopInputProviderError> {
        platform::emit(&self.display_name, event)?;
        Ok(format!("x11-acknowledgement:{}", uuid::Uuid::new_v4()))
    }
}

fn registered_keysym_name(key: char) -> Result<String, DesktopInputProviderError> {
    match key {
        'a'..='z' => Ok(key.to_string()),
        '-' => Ok("minus".to_string()),
        _ => Err(DesktopInputProviderError::new(
            "desktop_input_key_not_registered",
        )),
    }
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void, CString};

    type Display = c_void;
    type Window = c_ulong;
    type XOpenDisplayFn = unsafe extern "C" fn(*const c_char) -> *mut Display;
    type XCloseDisplayFn = unsafe extern "C" fn(*mut Display) -> c_int;
    type XSyncFn = unsafe extern "C" fn(*mut Display, c_int) -> c_int;
    type XStringToKeysymFn = unsafe extern "C" fn(*const c_char) -> c_ulong;
    type XKeysymToKeycodeFn = unsafe extern "C" fn(*mut Display, c_ulong) -> c_uint;
    type XTestFakeMotionEventFn =
        unsafe extern "C" fn(*mut Display, c_int, c_int, c_int, c_ulong) -> c_int;
    type XTestFakeButtonEventFn =
        unsafe extern "C" fn(*mut Display, c_uint, c_int, c_ulong) -> c_int;
    type XTestFakeKeyEventFn = unsafe extern "C" fn(*mut Display, c_uint, c_int, c_ulong) -> c_int;
    type XDefaultRootWindowFn = unsafe extern "C" fn(*mut Display) -> Window;
    type XGetInputFocusFn = unsafe extern "C" fn(*mut Display, *mut Window, *mut c_int) -> c_int;
    type XFetchNameFn = unsafe extern "C" fn(*mut Display, Window, *mut *mut c_char) -> c_int;
    type XQueryTreeFn = unsafe extern "C" fn(
        *mut Display,
        Window,
        *mut Window,
        *mut Window,
        *mut *mut Window,
        *mut c_uint,
    ) -> c_int;
    type XQueryPointerFn = unsafe extern "C" fn(
        *mut Display,
        Window,
        *mut Window,
        *mut Window,
        *mut c_int,
        *mut c_int,
        *mut c_int,
        *mut c_int,
        *mut c_uint,
    ) -> c_int;
    type XFreeFn = unsafe extern "C" fn(*mut c_void) -> c_int;

    struct XTestLibraries {
        x11_handle: *mut c_void,
        xtst_handle: *mut c_void,
        open_display: XOpenDisplayFn,
        close_display: XCloseDisplayFn,
        sync: XSyncFn,
        string_to_keysym: XStringToKeysymFn,
        keysym_to_keycode: XKeysymToKeycodeFn,
        default_root_window: XDefaultRootWindowFn,
        get_input_focus: XGetInputFocusFn,
        fetch_name: XFetchNameFn,
        query_tree: XQueryTreeFn,
        query_pointer: XQueryPointerFn,
        free: XFreeFn,
        fake_motion: XTestFakeMotionEventFn,
        fake_button: XTestFakeButtonEventFn,
        fake_key: XTestFakeKeyEventFn,
    }

    impl XTestLibraries {
        fn load() -> Result<Self, DesktopInputProviderError> {
            let x11_handle = open_library(&["libX11.so.6", "libX11.so"])?;
            let xtst_handle = match open_library(&["libXtst.so.6", "libXtst.so"]) {
                Ok(handle) => handle,
                Err(error) => {
                    // SAFETY: `x11_handle` came from a successful `dlopen`.
                    unsafe { libc::dlclose(x11_handle) };
                    return Err(error);
                }
            };
            // SAFETY: Each requested symbol has the matching Xlib or XTEST C
            // function type. Missing symbols fail closed before use.
            let loaded = unsafe {
                (|| {
                    Ok(Self {
                        x11_handle,
                        xtst_handle,
                        open_display: symbol(x11_handle, "XOpenDisplay")?,
                        close_display: symbol(x11_handle, "XCloseDisplay")?,
                        sync: symbol(x11_handle, "XSync")?,
                        string_to_keysym: symbol(x11_handle, "XStringToKeysym")?,
                        keysym_to_keycode: symbol(x11_handle, "XKeysymToKeycode")?,
                        default_root_window: symbol(x11_handle, "XDefaultRootWindow")?,
                        get_input_focus: symbol(x11_handle, "XGetInputFocus")?,
                        fetch_name: symbol(x11_handle, "XFetchName")?,
                        query_tree: symbol(x11_handle, "XQueryTree")?,
                        query_pointer: symbol(x11_handle, "XQueryPointer")?,
                        free: symbol(x11_handle, "XFree")?,
                        fake_motion: symbol(xtst_handle, "XTestFakeMotionEvent")?,
                        fake_button: symbol(xtst_handle, "XTestFakeButtonEvent")?,
                        fake_key: symbol(xtst_handle, "XTestFakeKeyEvent")?,
                    })
                })()
            };
            if loaded.is_err() {
                // SAFETY: Both handles came from successful `dlopen` calls.
                unsafe {
                    libc::dlclose(xtst_handle);
                    libc::dlclose(x11_handle);
                }
            }
            loaded
        }
    }

    impl Drop for XTestLibraries {
        fn drop(&mut self) {
            // SAFETY: Handles remain owned by this value and are closed once.
            unsafe {
                libc::dlclose(self.xtst_handle);
                libc::dlclose(self.x11_handle);
            }
        }
    }

    pub(super) fn emit(
        display_name: &str,
        event: &ControlledX11Event,
    ) -> Result<(), DesktopInputProviderError> {
        if unsafe { libc::geteuid() } == 0 {
            return Err(DesktopInputProviderError::new(
                "desktop_input_root_execution_forbidden",
            ));
        }
        let display_name = CString::new(display_name).map_err(|_| {
            DesktopInputProviderError::new("desktop_input_display_authority_invalid")
        })?;
        let libraries = XTestLibraries::load()?;
        // SAFETY: The display pointer is checked before use and closed exactly
        // once after the bounded XTEST call and XSync acknowledgement.
        unsafe {
            let display = (libraries.open_display)(display_name.as_ptr());
            if display.is_null() {
                return Err(DesktopInputProviderError::new(
                    "desktop_input_display_unavailable",
                ));
            }
            let result = emit_on_display(&libraries, display, event);
            if result.is_ok() {
                (libraries.sync)(display, 0);
            }
            (libraries.close_display)(display);
            result
        }
    }

    pub(super) fn probe(display_name: &str) -> Result<XTestSceneProbe, DesktopInputProviderError> {
        let display_name = CString::new(display_name).map_err(|_| {
            DesktopInputProviderError::new("desktop_input_display_authority_invalid")
        })?;
        let libraries = XTestLibraries::load()?;
        unsafe {
            let display = (libraries.open_display)(display_name.as_ptr());
            if display.is_null() {
                return Err(DesktopInputProviderError::new(
                    "desktop_input_display_unavailable",
                ));
            }
            let result = probe_on_display(&libraries, display);
            (libraries.close_display)(display);
            result
        }
    }

    unsafe fn probe_on_display(
        libraries: &XTestLibraries,
        display: *mut Display,
    ) -> Result<XTestSceneProbe, DesktopInputProviderError> {
        let root = (libraries.default_root_window)(display);
        let mut root_return = 0;
        let mut child_return = 0;
        let mut root_x = 0;
        let mut root_y = 0;
        let mut window_x = 0;
        let mut window_y = 0;
        let mut mask = 0;
        if (libraries.query_pointer)(
            display,
            root,
            &mut root_return,
            &mut child_return,
            &mut root_x,
            &mut root_y,
            &mut window_x,
            &mut window_y,
            &mut mask,
        ) == 0
        {
            return Err(DesktopInputProviderError::new(
                "desktop_input_focus_probe_unavailable",
            ));
        }
        let mut focused = 0;
        let mut revert = 0;
        (libraries.get_input_focus)(display, &mut focused, &mut revert);
        let mut controlled_fixture_focused = false;
        for _ in 0..16 {
            let mut name = std::ptr::null_mut();
            if (libraries.fetch_name)(display, focused, &mut name) != 0 && !name.is_null() {
                let title = std::ffi::CStr::from_ptr(name).to_bytes();
                controlled_fixture_focused = title == b"Agent Browser Controlled X11 Fixture";
                (libraries.free)(name.cast());
                if controlled_fixture_focused {
                    break;
                }
            }
            let mut root_window = 0;
            let mut parent = 0;
            let mut children = std::ptr::null_mut();
            let mut child_count = 0;
            if (libraries.query_tree)(
                display,
                focused,
                &mut root_window,
                &mut parent,
                &mut children,
                &mut child_count,
            ) == 0
                || parent == 0
                || parent == focused
            {
                break;
            }
            if !children.is_null() {
                (libraries.free)(children.cast());
            }
            focused = parent;
        }
        Ok(XTestSceneProbe {
            pointer_x: root_x,
            pointer_y: root_y,
            controlled_fixture_focused,
        })
    }

    unsafe fn emit_on_display(
        libraries: &XTestLibraries,
        display: *mut Display,
        event: &ControlledX11Event,
    ) -> Result<(), DesktopInputProviderError> {
        let emitted = match event {
            ControlledX11Event::PointerMove { x, y } => {
                let x = c_int::try_from(*x).map_err(|_| {
                    DesktopInputProviderError::new("desktop_input_event_out_of_bounds")
                })?;
                let y = c_int::try_from(*y).map_err(|_| {
                    DesktopInputProviderError::new("desktop_input_event_out_of_bounds")
                })?;
                (libraries.fake_motion)(display, -1, x, y, 0)
            }
            ControlledX11Event::LeftDown => (libraries.fake_button)(display, 1, 1, 0),
            ControlledX11Event::LeftUp => (libraries.fake_button)(display, 1, 0, 0),
            ControlledX11Event::KeyDown { key } => {
                let keycode = keycode(libraries, display, *key)?;
                (libraries.fake_key)(display, keycode, 1, 0)
            }
            ControlledX11Event::KeyUp { key } => {
                let keycode = keycode(libraries, display, *key)?;
                (libraries.fake_key)(display, keycode, 0, 0)
            }
        };
        if emitted == 0 {
            return Err(DesktopInputProviderError::new(
                "desktop_input_xtest_effect_uncertain",
            ));
        }
        Ok(())
    }

    unsafe fn keycode(
        libraries: &XTestLibraries,
        display: *mut Display,
        key: char,
    ) -> Result<c_uint, DesktopInputProviderError> {
        let keysym_name = registered_keysym_name(key)?;
        let keysym_name =
            CString::new(keysym_name).expect("registered keysym name contains no NUL");
        let keysym = (libraries.string_to_keysym)(keysym_name.as_ptr());
        if keysym == 0 {
            return Err(DesktopInputProviderError::new(
                "desktop_input_key_not_registered",
            ));
        }
        let keycode = (libraries.keysym_to_keycode)(display, keysym);
        if keycode == 0 {
            return Err(DesktopInputProviderError::new(
                "desktop_input_key_not_registered",
            ));
        }
        Ok(keycode)
    }

    fn open_library(names: &[&str]) -> Result<*mut c_void, DesktopInputProviderError> {
        for name in names {
            let name = CString::new(*name).expect("static library name");
            // SAFETY: `name` is NUL terminated and the returned handle is
            // checked before it is retained.
            let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
            if !handle.is_null() {
                return Ok(handle);
            }
        }
        Err(DesktopInputProviderError::new(
            "desktop_input_xtest_provider_unavailable",
        ))
    }

    unsafe fn symbol<T: Copy>(
        handle: *mut c_void,
        name: &str,
    ) -> Result<T, DesktopInputProviderError> {
        let name = CString::new(name).expect("static symbol name");
        let pointer = libc::dlsym(handle, name.as_ptr());
        if pointer.is_null() {
            return Err(DesktopInputProviderError::new(
                "desktop_input_xtest_provider_unavailable",
            ));
        }
        Ok(std::mem::transmute_copy(&pointer))
    }
}

#[cfg(not(unix))]
mod platform {
    use super::*;

    pub(super) fn emit(
        _display_name: &str,
        _event: &ControlledX11Event,
    ) -> Result<(), DesktopInputProviderError> {
        Err(DesktopInputProviderError::new(
            "desktop_input_provider_unsupported",
        ))
    }

    pub(super) fn probe(_display_name: &str) -> Result<XTestSceneProbe, DesktopInputProviderError> {
        Err(DesktopInputProviderError::new(
            "desktop_input_xtest_provider_unavailable",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_rejects_missing_or_ambiguous_display_authority() {
        for display_name in ["", "  ", "\0"] {
            assert_eq!(
                XTestSink::new(display_name).unwrap_err().code(),
                "desktop_input_display_authority_invalid"
            );
        }
    }

    #[test]
    fn registered_hyphen_uses_the_x11_minus_keysym_name() {
        assert_eq!(registered_keysym_name('-').unwrap(), "minus");
        assert_eq!(registered_keysym_name('f').unwrap(), "f");
        assert_eq!(
            registered_keysym_name('_').unwrap_err().code(),
            "desktop_input_key_not_registered"
        );
    }
}
