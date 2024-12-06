use ::lease::{
    api::{
        position::{ChangeCmd, ClosePolicyChange},
        ExecuteMsg,
    },
    error::ContractError,
};
use anyhow::Error;
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse, testing};

use crate::{
    common::{test_case::response::ResponseWithInterChainMsgs, ADMIN, USER},
    lease::LeaseTestCase,
};

mod change;
mod trigger;

fn change_ok(
    test_case: &mut LeaseTestCase,
    lease: Addr,
    take_profit: Option<ChangeCmd>,
    stop_loss: Option<ChangeCmd>,
) {
    send_change(
        test_case,
        USER,
        lease,
        ClosePolicyChange {
            stop_loss,
            take_profit,
        },
    )
    .unwrap()
    .ignore_response()
    .unwrap_response()
}

fn change_err(
    test_case: &mut LeaseTestCase,
    lease: Addr,
    take_profit: Option<ChangeCmd>,
    stop_loss: Option<ChangeCmd>,
) -> Error {
    send_change(
        test_case,
        USER,
        lease,
        ClosePolicyChange {
            stop_loss,
            take_profit,
        },
    )
    .unwrap_err()
}

fn change_unauthorized(test_case: &mut LeaseTestCase, lease: Addr) {
    use access_control::error::Error;

    let err = send_change(
        test_case,
        ADMIN,
        lease,
        ClosePolicyChange {
            stop_loss: None,
            take_profit: Some(ChangeCmd::Reset),
        },
    )
    .unwrap_err();

    assert_eq!(
        err.downcast_ref::<ContractError>(),
        Some(&ContractError::Unauthorized(Error::Unauthorized {}))
    );
}

fn send_change<'r>(
    test_case: &'r mut LeaseTestCase,
    sender: &str,
    lease: Addr,
    change: ClosePolicyChange,
) -> anyhow::Result<ResponseWithInterChainMsgs<'r, AppResponse>> {
    test_case.app.execute(
        testing::user(sender),
        lease,
        &ExecuteMsg::ChangeClosePolicy(change),
        &[],
    )
}
