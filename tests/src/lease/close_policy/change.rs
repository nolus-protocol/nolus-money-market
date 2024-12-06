use ::lease::{
    api::{
        position::{ChangeCmd, ClosePolicyChange},
        query::{ClosePolicy, StateResponse},
        ExecuteMsg,
    },
    error::{CloseStrategy, ContractError, PositionError},
};
use anyhow::Error;
use finance::{coin::Coin, percent::Percent};
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse, testing};

use crate::{
    common::{oracle, test_case::response::ResponseWithInterChainMsgs, ADMIN, USER},
    lease::{
        self, LeaseCurrency, LeaseTestCase, LeaserInstantiator, LpnCurrency, PaymentCurrency,
        DOWNPAYMENT,
    },
};

#[test]
fn by_another_user() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);
    change_unauthorized(&mut test_case, lease.clone());
}

#[test]
fn tp_zero() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);

    let err = change_err(
        &mut test_case,
        lease,
        Some(ChangeCmd::Set(Percent::ZERO)),
        Some(ChangeCmd::Reset),
    );

    assert_eq!(
        err.downcast_ref::<ContractError>(),
        Some(&ContractError::PositionError(
            PositionError::ZeroClosePolicy("take profit")
        ))
    );
}

#[test]
fn sl_zero() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);

    let err = change_err(
        &mut test_case,
        lease,
        Some(ChangeCmd::Reset),
        Some(ChangeCmd::Set(Percent::ZERO)),
    );

    assert_eq!(
        err.downcast_ref::<ContractError>(),
        Some(&ContractError::PositionError(
            PositionError::ZeroClosePolicy("stop loss")
        ))
    );
}

#[test]
fn tp_trigger() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);

    let init_ltv = LeaserInstantiator::INITIAL_LTV;
    let tp_new = init_ltv + Percent::from_permille(1);
    let err = change_err(
        &mut test_case,
        lease,
        Some(ChangeCmd::Set(tp_new)),
        Some(ChangeCmd::Reset),
    );

    let Some(ContractError::PositionError(PositionError::TriggerClose {
        lease_ltv: _,
        strategy: CloseStrategy::TakeProfit(tp_trigger),
    })) = err.downcast_ref::<ContractError>()
    else {
        unreachable!()
    };
    assert_eq!(&tp_new, tp_trigger);
}

#[test]
fn sl_trigger() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);

    let init_ltv = LeaserInstantiator::INITIAL_LTV;
    let sl_new = init_ltv;
    let err = change_err(
        &mut test_case,
        lease,
        Some(ChangeCmd::Reset),
        Some(ChangeCmd::Set(sl_new)),
    );

    let Some(ContractError::PositionError(PositionError::TriggerClose {
        lease_ltv: _,
        strategy: CloseStrategy::StopLoss(sl_trigger),
    })) = err.downcast_ref::<ContractError>()
    else {
        unreachable!()
    };
    assert_eq!(&sl_new, sl_trigger);
}

#[test]
fn liquidation_conflict() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);

    let max_ltv = LeaserInstantiator::MAX_LTV;
    let sl_new = max_ltv;
    let err = change_err(
        &mut test_case,
        lease.clone(),
        Some(ChangeCmd::Set(sl_new - Percent::from_permille(1))),
        Some(ChangeCmd::Set(sl_new)),
    );

    let Some(ContractError::PositionError(PositionError::LiquidationConflict {
        strategy: CloseStrategy::StopLoss(sl),
        top_bound,
    })) = err.downcast_ref::<ContractError>()
    else {
        unreachable!()
    };
    assert_eq!(&sl_new, sl);
    assert_eq!(&max_ltv, top_bound);

    assert!(matches!(
        change_err(
            &mut test_case,
            lease,
            Some(ChangeCmd::Set(sl_new)),
            Some(ChangeCmd::Reset),
        )
        .downcast_ref::<ContractError>(),
        Some(ContractError::PositionError(
            PositionError::LiquidationConflict {
                strategy: CloseStrategy::TakeProfit(_),
                top_bound: _,
            }
        ))
    ));
}

#[test]
fn tp_set() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    // LeaseCLpnC = 1:1
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);
    assert_eq!(ClosePolicy::default(), query_policy(&test_case, &lease));

    let tp = Percent::from_percent(28);
    change_ok(
        &mut test_case,
        lease.clone(),
        Some(ChangeCmd::Set(tp)),
        Some(ChangeCmd::Reset),
    );
    assert_eq!(
        ClosePolicy::new(Some(tp), None),
        query_policy(&test_case, &lease)
    );

    // LeaseC/LpnC = 2/1
    oracle::feed_price(
        &mut test_case,
        testing::user(ADMIN),
        Coin::<LeaseCurrency>::from(1),
        Coin::<LpnCurrency>::from(45),
    );
}

fn query_policy(test_case: &LeaseTestCase, lease: &Addr) -> ClosePolicy {
    let StateResponse::Opened { close_policy, .. } = lease::state_query(test_case, lease) else {
        unreachable!()
    };
    close_policy
}

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
