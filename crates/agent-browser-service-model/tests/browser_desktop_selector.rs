use agent_browser_service_model::{select_least_crowded_browser_desktop, BrowserDesktopRoute};

fn route(id: &str, display_name: &str, healthy: bool) -> BrowserDesktopRoute {
    BrowserDesktopRoute {
        id: id.to_string(),
        display_name: display_name.to_string(),
        healthy,
    }
}

#[test]
fn selects_the_healthy_desktop_with_fewer_live_browsers() {
    let routes = vec![route("guac-a", ":10", true), route("guac-b", ":11", true)];
    let live_displays = vec![":10".to_string()];

    let selected = select_least_crowded_browser_desktop(&routes, &live_displays).unwrap();

    assert_eq!(selected.route_id, "guac-b");
    assert_eq!(selected.display_name, ":11");
    assert_eq!(selected.live_browser_count, 0);
}

#[test]
fn configured_route_order_breaks_equal_load_ties() {
    let routes = vec![route("guac-a", ":10", true), route("guac-b", ":11", true)];
    let live_displays = Vec::new();

    let selected = select_least_crowded_browser_desktop(&routes, &live_displays).unwrap();

    assert_eq!(selected.route_id, "guac-a");
}

#[test]
fn automatic_remote_selection_excludes_local_screen_and_unhealthy_routes() {
    let routes = vec![
        route("local", ":0", true),
        route("guac-a", ":10", false),
        route("guac-b", ":11", true),
    ];
    let live_displays = vec![":11".to_string(), ":11".to_string()];

    let selected = select_least_crowded_browser_desktop(&routes, &live_displays).unwrap();

    assert_eq!(selected.route_id, "guac-b");
    assert_eq!(selected.live_browser_count, 2);
}

#[test]
fn duplicate_active_route_or_display_identity_is_rejected() {
    let duplicate_route = vec![route("guac-a", ":10", true), route("guac-a", ":11", true)];
    assert_eq!(
        select_least_crowded_browser_desktop(&duplicate_route, &[]).unwrap_err(),
        "browser_desktop_route_id_duplicate:guac-a"
    );

    let duplicate_display = vec![route("guac-a", ":10", true), route("guac-b", ":10", true)];
    assert_eq!(
        select_least_crowded_browser_desktop(&duplicate_display, &[]).unwrap_err(),
        "browser_desktop_display_duplicate::10"
    );
}
