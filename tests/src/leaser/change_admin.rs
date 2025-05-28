use leaser::ContractError;
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse, testing};

use crate::common::{
    ADMIN, LEASE_ADMIN, USER,
    leaser::{self as leaser_common},
    test_case::{app::App, response::ResponseWithInterChainMsgs},
};

#[test]
fn not_privileged() {
    let mut test_case = leaser_common::test_case();

    let user = testing::user(USER);
    let admin = testing::user(ADMIN);
    let leaser = test_case.address_book.leaser().clone();

    assert!(matches!(
        change_admin(&mut test_case.app, leaser, user, admin)
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

    assert!(change_admin(&mut test_case.app, leaser, lease_admin, admin).is_ok());
}

pub(super) fn change_admin(
    app: &mut App,
    leaser: Addr,
    caller: Addr,
    new_admin: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    app.execute(
        caller,
        leaser,
        &leaser::msg::ExecuteMsg::ChangeLeaseAdmin { new: new_admin },
        &[],
    )
}
