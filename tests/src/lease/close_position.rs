use ::lease::{
    api::{ExecuteMsg, FullClose, PartialClose, PositionClose, StateResponse},
    error::ContractError,
};
use currency::Currency;
use finance::{
    coin::{Amount, Coin},
    price,
    zero::Zero,
};
use sdk::cosmwasm_std::{Addr, Coin as CwCoin, Event, Timestamp};

use crate::{
    common::{
        self,
        leaser::{self, Instantiator},
        test_case::response::ResponseWithInterChainMsgs,
        ADMIN, USER,
    },
    lease::{
        self, dex, LeaseCoin, LeaseCurrency, Lpn, LpnCoin, PaymentCoin, PaymentCurrency,
        DOWNPAYMENT,
    },
};

use super::LeaseTestCase;

#[test]
fn close_by_another_user() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);
    assert_unauthorized(
        &mut test_case,
        lease.clone(),
        ExecuteMsg::ClosePosition(PositionClose::FullClose(FullClose {})),
    );
    assert_unauthorized(
        &mut test_case,
        lease,
        ExecuteMsg::ClosePosition(PositionClose::PartialClose(PartialClose {
            amount: LeaseCoin::from(1234414).into(),
        })),
    );
}

#[test]
fn full_close() {
    let lease_amount: LeaseCoin = lease_amount();
    let customer = Addr::unchecked(USER);
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let exp_loan_close = true;
    let exp_change = price::total(DOWNPAYMENT, lease::price_lpn_of());
    let lease = do_close(
        &mut test_case,
        &customer,
        lease_amount,
        PositionClose::FullClose(FullClose {}),
        exp_loan_close,
        exp_change,
    );
    let state = lease::state_query(&test_case, lease.as_str());
    assert_eq!(StateResponse::Closed(), state);

    assert_eq!(
        lease_balance(&test_case, lease),
        common::cwcoin_as_balance(LeaseCoin::ZERO),
    );

    leaser::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        customer.clone(),
    );

    assert_eq!(exp_change, user_balance::<Lpn>(&customer, &test_case));
}

#[test]
fn partial_close_loan_not_closed() {
    let lease_amount: LeaseCoin = lease_amount();
    let principal: LpnCoin = price::total(lease_amount, lease::price_lpn_of())
        - price::total(DOWNPAYMENT, lease::price_lpn_of());
    let close_amount: LeaseCoin =
        price::total(principal - 1234567.into(), lease::price_lpn_of().inv());
    let repay_principal = price::total(close_amount, lease::price_lpn_of());
    let customer = Addr::unchecked(USER);
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let exp_loan_close = false;
    let exp_change = LpnCoin::ZERO;
    let lease = do_close(
        &mut test_case,
        &customer,
        close_amount,
        PositionClose::PartialClose(PartialClose {
            amount: close_amount.into(),
        }),
        exp_loan_close,
        exp_change,
    );
    let state = lease::state_query(&test_case, lease.as_str());
    assert_eq!(
        lease::expected_open_state(
            &test_case,
            DOWNPAYMENT,
            repay_principal,
            close_amount,
            Timestamp::default(),
            Timestamp::default(),
            Timestamp::default(),
        ),
        state
    );
    assert_eq!(
        lease_balance(&test_case, lease),
        common::cwcoin_as_balance(exp_change),
    );

    assert_eq!(LpnCoin::ZERO, user_balance::<Lpn>(&customer, &test_case));
    assert_eq!(
        LeaseCoin::ZERO,
        user_balance::<LeaseCurrency>(&customer, &test_case)
    );
}

#[test]
fn partial_close_loan_closed() {
    let lease_amount: LeaseCoin = lease_amount();
    let principal: LpnCoin = price::total(lease_amount, lease::price_lpn_of())
        - price::total(DOWNPAYMENT, lease::price_lpn_of());
    let exp_change: LpnCoin = 345.into();

    let repay_principal = principal + exp_change;
    let close_amount: LeaseCoin = price::total(repay_principal, lease::price_lpn_of().inv());

    let customer = Addr::unchecked(USER);
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let exp_loan_close = true;
    let lease = do_close(
        &mut test_case,
        &customer,
        close_amount,
        PositionClose::PartialClose(PartialClose {
            amount: close_amount.into(),
        }),
        exp_loan_close,
        exp_change,
    );
    let state = lease::state_query(&test_case, lease.as_str());
    assert_eq!(
        StateResponse::Paid {
            amount: (lease_amount - close_amount).into(),
            in_progress: None
        },
        state
    );

    assert_eq!(
        lease_balance(&test_case, lease),
        common::cwcoin_as_balance(exp_change),
    );

    assert_eq!(LpnCoin::ZERO, user_balance::<Lpn>(&customer, &test_case));
    assert_eq!(
        LeaseCoin::ZERO,
        user_balance::<LeaseCurrency>(&customer, &test_case)
    );
}

#[test]
fn partial_close_invalid_currency() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);
    let msg = &ExecuteMsg::ClosePosition(PositionClose::PartialClose(PartialClose {
        amount: PaymentCoin::from(12345678).into(),
    }));

    let err = test_case
        .app
        .execute(Addr::unchecked(USER), lease, msg, &[])
        .unwrap_err();
    assert_eq!(
        err.root_cause().downcast_ref::<finance::error::Error>(),
        Some(&finance::error::Error::UnexpectedTicker(
            PaymentCurrency::TICKER.into(),
            LeaseCurrency::TICKER.into(),
        ))
    );
}

#[test]
fn partial_close_min_asset() {
    let min_asset_lpn = Instantiator::position_spec().min_asset.try_into().unwrap();
    let min_asset = price::total(min_asset_lpn, lease::price_lpn_of().inv());
    let lease_amount: LeaseCoin = lease_amount();

    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);
    let msg = &ExecuteMsg::ClosePosition(PositionClose::PartialClose(PartialClose {
        amount: (lease_amount - min_asset + 1.into()).into(),
    }));

    let err = test_case
        .app
        .execute(Addr::unchecked(USER), lease, msg, &[])
        .unwrap_err();
    assert_eq!(
        err.root_cause().downcast_ref::<ContractError>(),
        Some(&ContractError::PositionCloseAmountTooBig(
            min_asset_lpn.into()
        ))
    );
}

#[test]
fn partial_close_min_sell_asset() {
    let min_sell_asset_lpn = Instantiator::position_spec()
        .min_sell_asset
        .try_into()
        .unwrap();
    let min_sell_asset: LeaseCoin = price::total(min_sell_asset_lpn, lease::price_lpn_of().inv());

    let mut test_case = lease::create_test_case::<PaymentCurrency>();

    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);
    let msg = &ExecuteMsg::ClosePosition(PositionClose::PartialClose(PartialClose {
        amount: (min_sell_asset - 1.into()).into(),
    }));

    let err = test_case
        .app
        .execute(Addr::unchecked(USER), lease, msg, &[])
        .unwrap_err();
    assert_eq!(
        err.root_cause().downcast_ref::<ContractError>(),
        Some(&ContractError::PositionCloseAmountTooSmall(
            min_sell_asset_lpn.into()
        ))
    );
}

fn do_close(
    test_case: &mut LeaseTestCase,
    customer: &Addr,
    close_amount: LeaseCoin,
    close_msg: PositionClose,
    exp_loan_close: bool,
    exp_change: LpnCoin,
) -> Addr {
    let user_balance_before: PaymentCoin = user_balance(customer, test_case);
    let lease = lease::open_lease(test_case, DOWNPAYMENT, None);
    let exp_lease_amount = if let StateResponse::Opened {
        amount: lease_amount,
        ..
    } =
        lease::expected_newly_opened_state(test_case, DOWNPAYMENT, Coin::<Lpn>::ZERO)
    {
        TryInto::<LeaseCoin>::try_into(lease_amount).unwrap() - close_amount
    } else {
        panic!();
    };

    let close_amount_in_lpn = price::total(close_amount, lease::price_lpn_of());
    let response_close = send_close(
        test_case,
        lease.clone(),
        &ExecuteMsg::ClosePosition(close_msg),
    );

    dex::expect_swap(response_close);
    let response_swap = dex::do_swap(test_case, lease.clone(), close_amount, close_amount_in_lpn);

    dex::expect_init_transfer_in(response_swap);
    let response_transfer_in = dex::do_transfer_in(
        test_case,
        lease.clone(),
        close_amount_in_lpn,
        exp_lease_amount,
    );

    response_transfer_in.assert_event(
        &Event::new("wasm-ls-close-position")
            .add_attribute("to", lease.clone())
            .add_attribute(
                "payment-amount",
                Amount::from(close_amount_in_lpn).to_string(),
            )
            .add_attribute("payment-symbol", Lpn::TICKER)
            .add_attribute("loan-close", exp_loan_close.to_string())
            .add_attribute(
                "principal",
                Amount::from(close_amount_in_lpn - exp_change).to_string(),
            )
            .add_attribute("change", Amount::from(exp_change).to_string())
            .add_attribute("amount-amount", Amount::from(close_amount).to_string())
            .add_attribute("amount-symbol", LeaseCurrency::TICKER),
    );

    assert_eq!(
        user_balance_before - DOWNPAYMENT,
        user_balance::<PaymentCurrency>(customer, test_case)
    );
    lease
}

fn send_close<'r>(
    test_case: &'r mut LeaseTestCase,
    contract_addr: Addr,
    msg: &ExecuteMsg,
) -> ResponseWithInterChainMsgs<'r, ()> {
    test_case
        .app
        .execute(Addr::unchecked(USER), contract_addr, msg, &[])
        .unwrap()
        .ignore_response()
}

fn assert_unauthorized(test_case: &mut LeaseTestCase, lease: Addr, close_msg: ExecuteMsg) {
    let sender = Addr::unchecked(ADMIN);
    {
        let err = test_case
            .app
            .execute(sender, lease, &close_msg, &[])
            .unwrap_err();
        assert_eq!(
            err.root_cause()
                .downcast_ref::<access_control::error::Error>(),
            Some(&access_control::error::Error::Unauthorized {})
        );
    }
}

fn user_balance<C>(customer: &Addr, test_case: &LeaseTestCase) -> Coin<C>
where
    C: Currency,
{
    platform::bank::balance::<C>(customer, &test_case.app.query()).unwrap()
}

fn lease_balance(test_case: &LeaseTestCase, lease: Addr) -> Vec<CwCoin> {
    test_case.app.query().query_all_balances(lease).unwrap()
}

fn lease_amount() -> LeaseCoin {
    2857142857142.into()
}
