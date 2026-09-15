use agent_browser_candidate::{
    allocate_development_namespace, coordinate_test_run, DevelopmentNamespace, NamespaceAllocation,
    TestResourceClass, TestRunDecision, TestRunIdentity, TestRunRecord, TestRunState,
};

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn identity(selection: &[&str], resource_class: TestResourceClass) -> TestRunIdentity {
    TestRunIdentity::new(
        digest('a'),
        digest('b'),
        "suite-revision-1",
        selection.iter().map(|value| (*value).to_string()).collect(),
        digest('c'),
        "x86_64-unknown-linux-gnu",
        digest('d'),
        digest('e'),
        resource_class,
    )
    .expect("test identity")
}

#[test]
fn lane_namespace_is_stable_and_disjoint() {
    let first = allocate_development_namespace("P190", &[]).expect("first namespace");
    let NamespaceAllocation::Allocated(first) = first else {
        panic!("first allocation unexpectedly reused a namespace")
    };

    let replay = allocate_development_namespace("P190", std::slice::from_ref(&first))
        .expect("namespace replay");
    assert_eq!(replay, NamespaceAllocation::Existing(first.clone()));

    let second = allocate_development_namespace("P191", std::slice::from_ref(&first))
        .expect("second namespace");
    let NamespaceAllocation::Allocated(second) = second else {
        panic!("second allocation unexpectedly reused a namespace")
    };
    assert_namespace_disjoint(&first, &second);
}

fn assert_namespace_disjoint(first: &DevelopmentNamespace, second: &DevelopmentNamespace) {
    assert_ne!(first.install_root, second.install_root);
    assert_ne!(first.pseudo_home, second.pseudo_home);
    assert_ne!(first.runtime_directory, second.runtime_directory);
    assert_ne!(first.socket_directory, second.socket_directory);
    assert_ne!(first.profile_root, second.profile_root);
    assert_ne!(first.browser_state_root, second.browser_state_root);
    assert_ne!(first.output_root, second.output_root);
    assert!(first
        .ports
        .values()
        .all(|port| !second.ports.values().any(|other| other == port)));
}

#[test]
fn exact_active_test_is_joined_and_selection_order_is_canonical() {
    let first = identity(
        &["candidate::digest", "candidate::promotion"],
        TestResourceClass::IsolatedProviderFree,
    );
    let reordered = identity(
        &["candidate::promotion", "candidate::digest"],
        TestResourceClass::IsolatedProviderFree,
    );
    assert_eq!(first.digest(), reordered.digest());

    let active = TestRunRecord::active("run-1", first.clone());
    assert_eq!(
        coordinate_test_run(&reordered, &[active], &[]).expect("test advice"),
        TestRunDecision::JoinActive {
            run_id: "run-1".to_string(),
        }
    );
}

#[test]
fn only_exact_hermetic_clean_receipt_is_reused() {
    let requested = identity(
        &["candidate::digest"],
        TestResourceClass::IsolatedProviderFree,
    );
    let completed = TestRunRecord {
        run_id: "run-1".to_string(),
        identity: requested.clone(),
        state: TestRunState::Passed,
        hermetic: true,
        terminal_cleanup_proven: true,
        receipt_locator: Some("receipt://run-1".to_string()),
    };
    assert_eq!(
        coordinate_test_run(&requested, &[], std::slice::from_ref(&completed))
            .expect("receipt reuse"),
        TestRunDecision::ReuseReceipt {
            run_id: "run-1".to_string(),
            receipt_locator: "receipt://run-1".to_string(),
        }
    );

    let mut dirty = completed;
    dirty.terminal_cleanup_proven = false;
    assert!(matches!(
        coordinate_test_run(&requested, &[], &[dirty]).expect("new run"),
        TestRunDecision::StartIsolated { .. }
    ));
}

#[test]
fn shared_resource_conflict_waits_but_isolated_work_can_overlap() {
    let active_shared = TestRunRecord::active(
        "run-1",
        identity(
            &["workstation::acceptance"],
            TestResourceClass::Shared("chrome-main".to_string()),
        ),
    );
    let competing_shared = identity(
        &["workstation::other"],
        TestResourceClass::Shared("chrome-main".to_string()),
    );
    assert_eq!(
        coordinate_test_run(&competing_shared, std::slice::from_ref(&active_shared), &[])
            .expect("shared-resource advice"),
        TestRunDecision::WaitForSharedResource {
            resource_key: "chrome-main".to_string(),
            active_run_id: "run-1".to_string(),
        }
    );

    let isolated = identity(
        &["candidate::promotion"],
        TestResourceClass::IsolatedProviderFree,
    );
    assert!(matches!(
        coordinate_test_run(&isolated, &[active_shared], &[]).expect("isolated advice"),
        TestRunDecision::StartIsolated { .. }
    ));
}
