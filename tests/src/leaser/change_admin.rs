use leaser::ContractError;
use sdk::testing;

use crate::common::{
    ADMIN, LEASE_ADMIN, USER,
    leaser::{self as leaser_common},
};

#[test]
fn not_privileged() {
    let mut test_case = leaser_common::test_case();

    let user = testing::user(USER);
    let admin = testing::user(ADMIN);
    let leaser = test_case.address_book.leaser().clone();

    assert!(matches!(
        test_case
            .app
            .execute(
                user,
                leaser,
                &leaser::msg::ExecuteMsg::ChangeLeaseAdmin { new: admin },
                &[],
            )
            .expect_err("change lease admin by non authorized user should fail")
            .downcast_ref::<ContractError>(),
        Some(&ContractError::Unauthorized(_))
    ));
}
#[test]
fn privileged() {
    let mut test_case = leaser_common::test_case();

    let admin = testing::user(ADMIN);
    let lease_admin = testing::user(LEASE_ADMIN);
    let leaser = test_case.address_book.leaser().clone();

    assert!(
        test_case
            .app
            .execute(
                lease_admin,
                leaser,
                &leaser::msg::ExecuteMsg::ChangeLeaseAdmin { new: admin },
                &[],
            )
            .is_ok()
    );
}
