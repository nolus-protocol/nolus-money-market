use currency::{
    lease::{Atom, Cro},
    lpn::Usdc,
    Currency,
};
use finance::{
    coin::Coin,
    duration::Duration,
    interest::InterestPeriod,
    percent::Percent,
    period::Period,
    price::{self, Price},
};
use lease::api::{StateQuery, StateResponse};
use leaser::msg::{QueryMsg, QuoteResponse};
use sdk::{
    cosmwasm_std::{coin, Addr, Binary, Timestamp},
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};
use std::collections::HashSet;

use crate::common::{
    self, cwcoin,
    test_case::{builder::Builder as TestCaseBuilder, response::RemoteChain, TestCase},
    ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
};

mod close;
mod compare_with_lpp;
mod heal;
mod liquidation;
mod open;
mod repay;

type Lpn = Usdc;
type LpnCoin = Coin<Lpn>;

type LeaseCurrency = Cro;
type LeaseCoin = Coin<LeaseCurrency>;

type PaymentCurrency = Atom;
type PaymentCoin = Coin<PaymentCurrency>;

const DOWNPAYMENT: u128 = 1_000_000_000_000;

pub(super) fn create_payment_coin(amount: u128) -> PaymentCoin {
    PaymentCoin::new(amount)
}

pub(super) fn price_lpn_of<C>() -> Price<C, Lpn>
where
    C: Currency,
{
    Price::identity()
}

pub(super) fn feed_price<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms>,
) {
    let lease_price = price_lpn_of::<LeaseCurrency>();
    common::oracle::feed_price_pair(test_case, Addr::unchecked(ADMIN), lease_price);

    let payment_price = price_lpn_of::<PaymentCurrency>();
    common::oracle::feed_price_pair(test_case, Addr::unchecked(ADMIN), payment_price);
}

pub(super) fn create_test_case<InitFundsC>() -> TestCase<(), Addr, Addr, Addr, Addr, Addr, Addr>
where
    InitFundsC: Currency,
{
    let mut test_case: TestCase<_, _, _, _, _, _, _> =
        TestCaseBuilder::<Lpn, _, _, _, _, _, _, _>::with_reserve(&[
            cwcoin::<PaymentCurrency, _>(10_000_000_000_000_000_000_000_000_000),
            cwcoin::<Lpn, _>(10_000_000_000_000_000_000_000_000_000),
            cwcoin::<LeaseCurrency, _>(10_000_000_000_000_000_000_000_000_000),
            cwcoin::<InitFundsC, _>(10_000_000_000_000_000_000_000_000_000),
        ])
        .init_lpp_with_funds(
            None,
            &[coin(
                5_000_000_000_000_000_000_000_000_000,
                Lpn::BANK_SYMBOL,
            )],
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_time_alarms()
        .init_oracle(None)
        .init_treasury_without_dispatcher()
        .init_profit(24)
        .init_leaser()
        .into_generic();

    test_case.send_funds_from_admin(
        Addr::unchecked(USER),
        &[cwcoin::<InitFundsC, _>(1_000_000_000_000_000_000_000_000)],
    );

    common::oracle::add_feeder(&mut test_case, ADMIN);

    feed_price(&mut test_case);

    test_case
}

pub(super) fn calculate_interest(
    principal: Coin<Lpn>,
    interest_rate: Percent,
    duration: u64,
) -> Coin<Lpn> {
    InterestPeriod::with_interest(interest_rate)
        .and_period(Period::from_length(
            Timestamp::default(),
            Duration::from_nanos(duration),
        ))
        .interest(principal)
}

pub(super) fn open_lease<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms, DownpaymentC>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: Coin<DownpaymentC>,
    max_ltd: Option<Percent>,
) -> Addr
where
    DownpaymentC: Currency,
{
    try_init_lease(test_case, downpayment, max_ltd);

    let lease = get_lease_address(test_case);

    let quote = common::leaser::query_quote::<DownpaymentC, LeaseCurrency>(
        &mut test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        max_ltd,
    );
    let exp_borrow = TryInto::<Coin<Lpn>>::try_into(quote.borrow).unwrap();
    let exp_lease = TryInto::<Coin<LeaseCurrency>>::try_into(quote.total).unwrap();

    common::lease::complete_initialization::<Lpn, DownpaymentC, LeaseCurrency>(
        &mut test_case.app,
        TestCase::LEASER_CONNECTION_ID,
        lease.clone(),
        downpayment,
        exp_borrow,
        exp_lease,
    );

    lease
}

pub(super) fn try_init_lease<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms, D>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: Coin<D>,
    max_ltd: Option<Percent>,
) where
    D: Currency,
{
    let downpayment = (!downpayment.is_zero()).then(|| cwcoin::<D, _>(downpayment));

    let mut response = test_case
        .app
        .execute(
            Addr::unchecked(USER),
            test_case.address_book.leaser().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: LeaseCurrency::TICKER.into(),
                max_ltd,
            },
            downpayment.as_ref().map_or(&[], std::slice::from_ref),
        )
        .unwrap();

    response.expect_register_ica(TestCase::LEASER_CONNECTION_ID, "0");

    () = response.ignore_response().unwrap_response();
}

pub(super) fn get_lease_address<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
) -> Addr {
    let query_response: HashSet<Addr> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.leaser().clone(),
            &QueryMsg::Leases {
                owner: Addr::unchecked(USER),
            },
        )
        .unwrap();
    assert_eq!(query_response.len(), 1);
    query_response.iter().next().unwrap().clone()
}

pub(super) fn construct_response(data: Binary) -> NeutronSudoMsg {
    NeutronSudoMsg::Response {
        request: sdk::neutron_sdk::sudo::msg::RequestPacket {
            sequence: None,
            source_port: None,
            source_channel: None,
            destination_port: None,
            destination_channel: None,
            data: None,
            timeout_height: None,
            timeout_timestamp: None,
        },
        data,
    }
}

pub(super) fn quote_borrow<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: PaymentCoin,
) -> LpnCoin {
    LpnCoin::try_from(quote_query(test_case, downpayment).borrow).unwrap()
}

pub(super) fn quote_query<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms, DownpaymentC>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: Coin<DownpaymentC>,
) -> QuoteResponse
where
    DownpaymentC: Currency,
{
    test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.leaser().clone(),
            &QueryMsg::Quote {
                downpayment: downpayment.into(),
                lease_asset: LeaseCurrency::TICKER.into(),
                max_ltd: None,
            },
        )
        .unwrap()
}

pub(super) fn state_query<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: &str,
) -> StateResponse {
    test_case
        .app
        .query()
        .query_wasm_smart(contract_addr, &StateQuery {})
        .unwrap()
}

pub(super) fn expected_open_state<
    Dispatcher,
    Treasury,
    Profit,
    Lpp,
    Oracle,
    TimeAlarms,
    DownpaymentC,
    PaymentC,
>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: Coin<DownpaymentC>,
    payments: Coin<PaymentC>,
    last_paid: Timestamp,
    current_period_start: Timestamp,
    now: Timestamp,
) -> StateResponse
where
    DownpaymentC: Currency,
    PaymentC: Currency,
{
    let quote_result = quote_query(test_case, downpayment);
    let total: LeaseCoin = quote_result.total.try_into().unwrap();
    let total_lpn: LpnCoin = price::total(total, price_lpn_of::<LeaseCurrency>());
    let expected: LpnCoin = total_lpn
        - price::total(downpayment, price_lpn_of::<DownpaymentC>())
        - price::total(payments, price_lpn_of::<PaymentC>());
    let (overdue, due) = (
        current_period_start
            .nanos()
            .saturating_sub(last_paid.nanos()),
        now.nanos().saturating_sub(current_period_start.nanos()),
    );
    StateResponse::Opened {
        amount: total.into(),
        loan_interest_rate: quote_result.annual_interest_rate,
        margin_interest_rate: quote_result.annual_interest_rate_margin,
        principal_due: expected.into(),
        previous_margin_due: calculate_interest(
            expected,
            quote_result.annual_interest_rate_margin,
            overdue,
        )
        .into(),
        previous_interest_due: calculate_interest(
            expected,
            quote_result.annual_interest_rate,
            overdue,
        )
        .into(),
        current_margin_due: calculate_interest(
            expected,
            quote_result.annual_interest_rate_margin,
            due,
        )
        .into(),
        current_interest_due: calculate_interest(expected, quote_result.annual_interest_rate, due)
            .into(),
        validity: block_time(test_case),
        in_progress: None,
    }
}

pub(super) fn expected_newly_opened_state<
    Dispatcher,
    Treasury,
    Profit,
    Lpp,
    Oracle,
    TimeAlarms,
    DownpaymentC,
    PaymentC,
>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: Coin<DownpaymentC>,
    payments: Coin<PaymentC>,
) -> StateResponse
where
    DownpaymentC: Currency,
    PaymentC: Currency,
{
    expected_open_state(
        test_case,
        downpayment,
        payments,
        Timestamp::default(),
        Timestamp::default(),
        Timestamp::default(),
    )
}

pub(super) fn block_time<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
) -> Timestamp {
    test_case.app.block_info().time
}
