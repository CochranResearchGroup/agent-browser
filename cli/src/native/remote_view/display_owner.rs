//! Live X-server ownership proof. X authorization permits access; it does not
//! establish that a display belongs to the selected provider route.
use serde_json::{json, Value};

pub(crate) fn route_display_owner(display: Option<&str>, user: Option<&str>) -> Value {
    #[cfg(target_os = "linux")]
    {
        let expected = user.and_then(linux::user_uid);
        linux::observe(display, expected)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (display, user);
        json!({"required": false, "verified": true, "code": "not_applicable"})
    }
}

pub(crate) fn browser_display_owner(
    state: &super::ServiceState,
    browser: &super::super::service_model::BrowserProcess,
) -> Value {
    if browser.host != super::BrowserHost::RemoteHeaded {
        return json!({"required": false, "verified": true, "code": "not_applicable"});
    }
    let entries = state
        .route_pool
        .values()
        .filter(|entry| {
            state.remote_view_routes.values().any(|route| {
                route.id == entry.route_id
                    && route.browser_id.as_deref() == Some(browser.id.as_str())
                    && route.display_allocation_id == browser.display_allocation_id
            })
        })
        .collect::<Vec<_>>();
    if let [entry] = entries.as_slice() {
        let user = ["routeUser", "username", "user"]
            .iter()
            .find_map(|key| super::route_pool_target_string(entry, key));
        return route_display_owner(browser.display_name.as_deref(), user.as_deref());
    }
    // A standalone private display has no provider route. Its socket must still
    // belong to this runtime's OS account. Route-bound allocations cannot use
    // this fallback when their provider identity is missing or ambiguous.
    #[cfg(target_os = "linux")]
    if entries.is_empty()
        && browser.display_allocation_id.is_none()
        && browser.display_isolation.as_deref() == Some("private_virtual_display")
    {
        // SAFETY: geteuid has no pointer arguments or preconditions.
        return linux::observe(
            browser.display_name.as_deref(),
            Some(unsafe { libc::geteuid() }),
        );
    }
    route_display_owner(browser.display_name.as_deref(), None)
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    pub(super) fn user_uid(user: &str) -> Option<u32> {
        let name = CString::new(user).ok()?;
        let mut passwd = std::mem::MaybeUninit::<libc::passwd>::uninit();
        let mut result = std::ptr::null_mut();
        let mut buffer = vec![0u8; 16 * 1024];
        // SAFETY: all output pointers and the backing buffer remain valid for
        // the call; passwd is read only after success and a non-null result.
        let code = unsafe {
            libc::getpwnam_r(
                name.as_ptr(),
                passwd.as_mut_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut result,
            )
        };
        if code != 0 || result.is_null() {
            return None;
        }
        Some(unsafe { passwd.assume_init() }.pw_uid)
    }

    fn peer(path: &str, abstract_socket: bool) -> Option<libc::ucred> {
        // SAFETY: socket returns a new descriptor transferred once to OwnedFd.
        let raw = unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
            )
        };
        if raw < 0 {
            return None;
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        // SAFETY: zero is a valid initial representation of sockaddr_un.
        let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        address.sun_family = libc::AF_UNIX as _;
        let offset = usize::from(abstract_socket);
        if path.len() + 1 > address.sun_path.len() {
            return None;
        }
        for (slot, byte) in address.sun_path[offset..].iter_mut().zip(path.bytes()) {
            *slot = byte as _;
        }
        let length = std::mem::offset_of!(libc::sockaddr_un, sun_path) + path.len() + 1;
        // SAFETY: the address and length describe the initialized local socket
        // address. Nonblocking connect cannot stall on a full listen backlog.
        if unsafe {
            libc::connect(
                fd.as_raw_fd(),
                (&address as *const libc::sockaddr_un).cast(),
                length as _,
            )
        } != 0
        {
            return None;
        }
        let mut credentials = std::mem::MaybeUninit::<libc::ucred>::uninit();
        let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: the output pointer and size reference valid ucred storage.
        if unsafe {
            libc::getsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                credentials.as_mut_ptr().cast(),
                &mut size,
            )
        } != 0
            || size as usize != std::mem::size_of::<libc::ucred>()
        {
            return None;
        }
        Some(unsafe { credentials.assume_init() })
    }

    pub(super) fn observe(display: Option<&str>, expected_uid: Option<u32>) -> Value {
        let number = display
            .and_then(|s| s.strip_prefix(':'))
            .and_then(|s| s.strip_suffix(".0").or(Some(s)))
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse::<u32>().ok());
        let peers = number
            .map(|n| {
                let path = format!("/tmp/.X11-unix/X{n}");
                [peer(&path, true), peer(&path, false)]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let code = if expected_uid.is_none() {
            "route_display_owner_unproven"
        } else if number.is_none() || peers.is_empty() {
            "route_display_server_unavailable"
        } else if peers.iter().any(|p| Some(p.uid) != expected_uid) {
            "route_display_owner_mismatch"
        } else if peers.iter().any(|p| p.pid != peers[0].pid) {
            "route_display_server_ambiguous"
        } else {
            "route_display_owner_verified"
        };
        json!({"required": true, "verified": code == "route_display_owner_verified",
            "code": code, "displayName": display, "expectedUid": expected_uid,
            "observedPeers": peers.iter().map(|p| json!({"pid":p.pid,"uid":p.uid})).collect::<Vec<_>>(),
            "source": "native/remote_view/display_owner.rs::observe",
            "nextAction": if code == "route_display_owner_verified" { "none" } else { "repair_route_display_binding" }})
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::linux::net::SocketAddrExt;
        use std::os::unix::net::{SocketAddr, UnixListener};

        #[test]
        fn accessible_socket_requires_exact_route_uid() {
            let number = uuid::Uuid::new_v4().as_u128() as u32;
            let display = format!(":{number}");
            let address =
                SocketAddr::from_abstract_name(format!("/tmp/.X11-unix/X{number}")).unwrap();
            let _listener = UnixListener::bind_addr(&address).unwrap();
            let uid = unsafe { libc::geteuid() };
            assert_eq!(observe(Some(&display), Some(uid))["verified"], true);
            let denied = observe(Some(&display), Some(uid.wrapping_add(1)));
            assert_eq!(denied["verified"], false);
            assert_eq!(denied["code"], "route_display_owner_mismatch");
            assert_eq!(observe(Some(&display), None)["verified"], false);
            assert_eq!(
                observe(Some("remote.example:10"), Some(uid))["verified"],
                false
            );
        }
    }
}
