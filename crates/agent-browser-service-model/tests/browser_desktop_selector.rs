use agent_browser_service_model::{
    select_least_crowded_remote_view_desktop, RemoteViewDesktopCandidate, RemoteViewFixedDesktop,
};

fn candidate(id: &str, route: &str, ready: bool) -> RemoteViewDesktopCandidate {
    RemoteViewDesktopCandidate {
        desktop: RemoteViewFixedDesktop {
            desktop_id: id.to_string(),
            friendly_route_label: route.to_string(),
            generation: 1,
        },
        ready,
    }
}

#[test]
fn selects_the_ready_desktop_with_fewer_agent_browser_browsers() {
    let candidates = vec![
        candidate("11111111-1111-1111-1111-111111111111", "desktop-a", true),
        candidate("22222222-2222-2222-2222-222222222222", "desktop-b", true),
    ];
    let selected = select_least_crowded_remote_view_desktop(
        &candidates,
        &["11111111-1111-1111-1111-111111111111".to_string()],
    )
    .unwrap();

    assert_eq!(
        selected.desktop.desktop_id,
        "22222222-2222-2222-2222-222222222222"
    );
    assert_eq!(selected.live_browser_count, 0);
}

#[test]
fn observation_order_breaks_equal_load_ties_and_unready_desktops_are_excluded() {
    let candidates = vec![
        candidate("11111111-1111-1111-1111-111111111111", "desktop-a", false),
        candidate("22222222-2222-2222-2222-222222222222", "desktop-b", true),
        candidate("33333333-3333-3333-3333-333333333333", "desktop-c", true),
    ];
    let selected = select_least_crowded_remote_view_desktop(&candidates, &[]).unwrap();
    assert_eq!(
        selected.desktop.desktop_id,
        "22222222-2222-2222-2222-222222222222"
    );
}

#[test]
fn duplicate_ready_desktop_or_route_identity_is_rejected() {
    let duplicate_desktop = vec![
        candidate("11111111-1111-1111-1111-111111111111", "desktop-a", true),
        candidate("11111111-1111-1111-1111-111111111111", "desktop-b", true),
    ];
    assert_eq!(
        select_least_crowded_remote_view_desktop(&duplicate_desktop, &[]).unwrap_err(),
        "remote_view_desktop_id_duplicate:11111111-1111-1111-1111-111111111111"
    );

    let duplicate_route = vec![
        candidate("11111111-1111-1111-1111-111111111111", "desktop-a", true),
        candidate("22222222-2222-2222-2222-222222222222", "desktop-a", true),
    ];
    assert_eq!(
        select_least_crowded_remote_view_desktop(&duplicate_route, &[]).unwrap_err(),
        "remote_view_route_label_duplicate:desktop-a"
    );
}
