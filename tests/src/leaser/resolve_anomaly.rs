use lease::api::authz::AccessGranted;
use leaser::msg::QueryMsg;
use sdk::{cosmwasm_std::Addr, testing};

use crate::{
    common::{
        LEASE_ADMIN, USER,
        leaser::{self as leaser_common},
        test_case::app::App,
    },
    leaser::change_admin,
};

#[test]
fn not_privileged() {
    let mut test_case = leaser_common::test_case();

    let user = testing::user(USER);
    let lease_admin = testing::user(LEASE_ADMIN);
    let leaser = test_case.address_book.leaser().clone();

    assert_eq!(
        AccessGranted::No,
        check_permission(&test_case.app, leaser.clone(), user.clone())
    );

    assert!(
        change_admin::change_admin(
            &mut test_case.app,
            leaser.clone(),
            lease_admin,
            user.clone()
        )
        .is_ok()
    );

    assert_eq!(
        AccessGranted::Yes,
        check_permission(&test_case.app, leaser, user)
    );
}

#[test]
fn privileged() {
    let test_case = leaser_common::test_case();

    let lease_admin = testing::user(LEASE_ADMIN);
    let leaser = test_case.address_book.leaser().clone();

    assert_eq!(
        AccessGranted::Yes,
        check_permission(&test_case.app, leaser, lease_admin)
    );
}

fn check_permission(app: &App, leaser: Addr, subject: Addr) -> AccessGranted {
    app.query()
        .query_wasm_smart(
            leaser,
            &QueryMsg::CheckAnomalyResolutionPermission { by: subject },
        )
        .unwrap()
}
