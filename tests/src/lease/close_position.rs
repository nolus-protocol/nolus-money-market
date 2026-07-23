use currencies::{PaymentGroup, testing::PaymentC5};
use currency::CurrencyDef;
use finance::{
    coin::{Coin, CoinDTO},
    price,
    zero::Zero,
};
use lease::{
    api::{
        ExecuteMsg, LpnCoinDTO,
        position::{FullClose, PartialClose, PositionClose},
        query::{StateResponse, paid::ClosingTrx},
    },
    error::{ContractError, PositionError},
};
use platform::coin_legacy;
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
    testing,
};

use crate::common::{
    self, ADMIN, CwCoin, USER, ibc, lease as common_lease,
    leaser::{self, Instantiator},
    remote_lease_controller_stub::SwapFill,
    swap,
    test_case::{TestCase, response::ResponseWithInterChainMsgs},
};

use super::{
    DOWNPAYMENT, LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCoin, LpnCurrency, PaymentCoin,
    PaymentCurrency,
};

#[test]
fn close_by_another_user() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let lease = super::open_lease(&mut test_case, DOWNPAYMENT, None);
    assert_unauthorized(
        &mut test_case,
        lease.clone(),
        ExecuteMsg::ClosePosition(PositionClose::FullClose(FullClose {})),
    );
    assert_unauthorized(
        &mut test_case,
        lease,
        ExecuteMsg::ClosePosition(PositionClose::PartialClose(PartialClose {
            amount: LeaseCoin::new(1234414).into(),
        })),
    );
}

#[test]
fn full_close() {
    let lease_amount: LeaseCoin = lease_amount();
    let customer = testing::user(USER);
    let mut test_case = super::create_test_case::<PaymentCurrency>();

    let exp_loan_close = true;
    let exp_change = price::total(DOWNPAYMENT, super::price_lpn_of()).unwrap();
    let lease = do_close(
        &mut test_case,
        &customer,
        lease_amount,
        PositionClose::FullClose(FullClose {}),
        exp_loan_close,
        exp_change,
        LeaseCoin::ZERO,
    );
    let state = super::state_query(&test_case, lease.clone());
    assert_eq!(StateResponse::Closed(), state);

    common_lease::assert_lease_balance_eq(&test_case.app, &lease, common::cwcoin(LeaseCoin::ZERO));

    leaser::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        customer.clone(),
    );

    assert_eq!(
        exp_change,
        user_balance::<LpnCurrency>(&customer, &test_case)
    );
}

#[test]
fn partial_close_loan_not_closed() {
    let lease_amount: LeaseCoin = lease_amount();
    let principal: LpnCoin = price::total(lease_amount, super::price_lpn_of()).unwrap()
        - price::total(DOWNPAYMENT, super::price_lpn_of()).unwrap();
    let close_amount: LeaseCoin = price::total(
        principal - common::coin(1234567),
        super::price_lpn_of().inv(),
    )
    .unwrap();
    let repay_principal = price::total(close_amount, super::price_lpn_of()).unwrap();
    let customer = testing::user(USER);
    let mut test_case = super::create_test_case::<PaymentCurrency>();

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
        lease_amount - close_amount,
    );
    let state = super::state_query(&test_case, lease.clone());
    assert_eq!(
        super::expected_open_state(
            &test_case,
            DOWNPAYMENT,
            repay_principal,
            close_amount,
            Instantiator::REPAYMENT_PERIOD,
        ),
        state
    );
    common_lease::assert_lease_balance_eq(&test_case.app, &lease, common::cwcoin(exp_change));

    assert_eq!(
        user_balance::<LpnCurrency>(&customer, &test_case),
        LpnCoin::ZERO,
    );
    assert_eq!(
        user_balance::<LeaseCurrency>(&customer, &test_case),
        LeaseCoin::ZERO,
    );
}

#[test]
fn partial_close_loan_closed() {
    let lease_amount: LeaseCoin = lease_amount();
    let principal: LpnCoin = price::total(lease_amount, super::price_lpn_of()).unwrap()
        - price::total(DOWNPAYMENT, super::price_lpn_of()).unwrap();
    let exp_change: LpnCoin = common::coin(345);

    let repay_principal = principal + exp_change;
    let close_amount: LeaseCoin =
        price::total(repay_principal, super::price_lpn_of().inv()).unwrap();

    let customer = testing::user(USER);
    let mut test_case = super::create_test_case::<PaymentCurrency>();

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
        lease_amount - close_amount,
    );
    let state = super::state_query(&test_case, lease.clone());
    assert_eq!(
        StateResponse::Closing {
            amount: (lease_amount - close_amount).into(),
            in_progress: ClosingTrx::TransferInInit
        },
        state
    );

    common_lease::assert_lease_balance_eq(&test_case.app, &lease, common::cwcoin(exp_change));

    assert_eq!(
        LpnCoin::ZERO,
        user_balance::<LpnCurrency>(&customer, &test_case)
    );
    assert_eq!(
        LeaseCoin::ZERO,
        user_balance::<LeaseCurrency>(&customer, &test_case)
    );
}

#[test]
fn partial_close_invalid_currency() {
    let mut test_case: LeaseTestCase = super::create_test_case::<PaymentCurrency>();

    let lease: Addr = super::open_lease(&mut test_case, DOWNPAYMENT, None);

    let err = test_case
        .app
        .execute(
            testing::user(USER),
            lease,
            &(&ExecuteMsg::ClosePosition(PositionClose::PartialClose(PartialClose {
                amount: common::coin::<PaymentC5>(12345678).into(),
            }))),
            &[],
        )
        .unwrap_err();

    assert!(matches!(
        err.downcast_ref::<ContractError>().unwrap(),
        ContractError::FinanceError(finance::error::Error::CurrencyError(
            currency::error::Error::CurrencyMismatch {
                expected: _,
                found: _
            }
        ))
    ));
}

#[test]
fn partial_close_min_asset() {
    let min_asset_lpn = Instantiator::min_asset().try_into().unwrap();
    let min_asset = price::total(min_asset_lpn, super::price_lpn_of().inv()).unwrap();
    let lease_amount: LeaseCoin = lease_amount();

    let mut test_case = super::create_test_case::<PaymentCurrency>();

    let lease = super::open_lease(&mut test_case, DOWNPAYMENT, None);
    let msg = &ExecuteMsg::ClosePosition(PositionClose::PartialClose(PartialClose {
        amount: (lease_amount - min_asset + common::coin(1)).into(),
    }));

    let err = test_case
        .app
        .execute(testing::user(USER), lease, msg, &[])
        .unwrap_err();
    assert!(matches!(
        err.downcast_ref::<ContractError>().unwrap(),
        &ContractError::PositionError(
            PositionError::PositionCloseAmountTooBig(coin)
        ) if coin == LpnCoinDTO::from(min_asset_lpn)));
}

#[test]
fn partial_close_min_transaction() {
    let min_transaction_lpn = Instantiator::min_transaction().try_into().unwrap();
    let min_transaction: LeaseCoin =
        price::total(min_transaction_lpn, super::price_lpn_of().inv()).unwrap();

    let mut test_case = super::create_test_case::<PaymentCurrency>();

    let lease = super::open_lease(&mut test_case, DOWNPAYMENT, None);
    let msg = &ExecuteMsg::ClosePosition(PositionClose::PartialClose(PartialClose {
        amount: (min_transaction - common::coin(1)).into(),
    }));

    let err = test_case
        .app
        .execute(testing::user(USER), lease, msg, &[])
        .unwrap_err();
    assert!(matches!(
        err.downcast_ref::<ContractError>().unwrap(),
        &ContractError::PositionError(PositionError::PositionCloseAmountTooSmall(coin
       )) if coin == LpnCoinDTO::from(min_transaction_lpn)
    ));
}

fn do_close(
    test_case: &mut LeaseTestCase,
    customer_addr: &Addr,
    close_amount: LeaseCoin,
    close_msg: PositionClose,
    exp_loan_close: bool,
    exp_change: LpnCoin,
    exp_lease_amount_after: LeaseCoin,
) -> Addr {
    let user_balance_before: PaymentCoin = user_balance(customer_addr, test_case);
    let lease_addr: Addr = super::open_lease(test_case, DOWNPAYMENT, None);
    let lease_ica = TestCase::stub_pda(1);

    assert!(matches!(
        super::expected_newly_opened_state(test_case, DOWNPAYMENT, Coin::<LpnCurrency>::ZERO),
        StateResponse::Opened { .. }
    ));

    let controller = test_case.address_book.remote_lease_controller().clone();
    let close_amount_in_lpn: LpnCoin = price::total(close_amount, super::price_lpn_of()).unwrap();

    // Identity DEX fill: the sold collateral yields its LPN value.
    swap::set_fill(&mut test_case.app, &controller, SwapFill::InputAmount);

    let mut response_close = send_close(
        test_case,
        lease_addr.clone(),
        &ExecuteMsg::ClosePosition(close_msg),
    );

    // The sell-asset swap fired inline on the ClosePosition execute; the lease
    // (local-output) has emitted the transfer-in of the LPN proceeds.
    let transfer_amount: CwCoin = ibc::expect_remote_transfer(
        &mut response_close,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );
    assert_eq!(
        transfer_amount,
        coin_legacy::to_cosmwasm_on_dex(close_amount_in_lpn)
    );
    let _ = response_close.unwrap_response();

    // Fidelity: the swap input is exactly the close amount.
    let captured = swap::captured(&test_case.app, &controller);
    assert_eq!(
        <Coin<LeaseCurrency> as Into<CoinDTO<PaymentGroup>>>::into(close_amount),
        swap::token_in(&captured),
    );

    let mut response_transfer_in = swap::deliver_transfer_in(
        &mut test_case.app,
        lease_ica.clone(),
        lease_addr.clone(),
        &transfer_amount,
    );

    if exp_loan_close && !exp_lease_amount_after.is_zero() {
        let lease_amount_after = ibc::expect_remote_transfer(
            &mut response_transfer_in,
            TestCase::DEX_CONNECTION_ID,
            TestCase::LEASE_ICA_ID,
        );

        assert_eq!(
            coin_legacy::to_cosmwasm_on_dex(exp_lease_amount_after),
            lease_amount_after
        );
    }

    response_transfer_in.unwrap_response().assert_event(
        &Event::new("wasm-ls-close-position")
            .add_attribute("to", lease_addr.clone())
            .add_attribute("payment-amount", close_amount_in_lpn.display_primitive())
            .add_attribute("payment-symbol", LpnCurrency::ticker())
            .add_attribute("loan-close", exp_loan_close.to_string())
            .add_attribute(
                "principal",
                (close_amount_in_lpn - exp_change).display_primitive(),
            )
            .add_attribute("change", exp_change.display_primitive())
            .add_attribute("amount-amount", close_amount.display_primitive())
            .add_attribute("amount-symbol", LeaseCurrency::ticker()),
    );

    assert_eq!(
        user_balance::<PaymentCurrency>(customer_addr, test_case),
        user_balance_before - DOWNPAYMENT,
    );

    // The residual collateral left on the remote (StubPda) is `exp_lease_amount_after`.
    // The StubPda is not a bech32 address, so its balance cannot be queried; the residual
    // is pinned by the caller's `StateResponse` `amount` assertion together with the
    // intercepted close `submit_tx` transfers asserted above.

    lease_addr
}

fn send_close<'r>(
    test_case: &'r mut LeaseTestCase,
    contract_addr: Addr,
    msg: &ExecuteMsg,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    test_case
        .app
        .execute(testing::user(USER), contract_addr, msg, &[])
        .unwrap()
}

fn assert_unauthorized(test_case: &mut LeaseTestCase, lease: Addr, close_msg: ExecuteMsg) {
    let sender = testing::user(ADMIN);
    {
        use access_control::error::Error;

        let err = test_case
            .app
            .execute(sender, lease, &close_msg, &[])
            .unwrap_err();
        assert!(matches!(
            err.downcast_ref::<ContractError>().unwrap(),
            &ContractError::Unauthorized(Error::Unauthorized {})
        ));
    }
}

fn user_balance<C>(customer: &Addr, test_case: &LeaseTestCase) -> Coin<C>
where
    C: CurrencyDef,
{
    platform::bank::balance::<C>(customer, test_case.app.query()).unwrap()
}

fn lease_amount() -> LeaseCoin {
    common::coin(2857142857142)
}
