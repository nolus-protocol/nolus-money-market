use std::collections::{HashMap, HashSet};

use currency::{
    lease::{Atom, Cro},
    lpn::Usdc,
    Currency,
};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::Fraction as _,
    interest::InterestPeriod,
    percent::Percent,
    period::Period,
    price::{self, Price},
    zero::Zero,
};
use lease::api::{ExecuteMsg, StateQuery, StateResponse};
use leaser::msg::{QueryMsg, QuoteResponse};
use osmosis_std::types::osmosis::gamm::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse,
};
use sdk::{
    cosmwasm_std::{coin, Addr, Binary, Coin as CwCoin, Event, Timestamp},
    cw_multi_test::AppResponse,
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};

use crate::common::{
    cwcoin, lease as lease_mod,
    leaser::{self as leaser_mod, Instantiator as LeaserInstantiator},
    oracle as oracle_mod,
    test_case::{
        builder::Builder as TestCaseBuilder,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
        TestCase,
    },
    ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
};

mod repay_mod;

mod close_mod;

type Lpn = Usdc;
type LpnCoin = Coin<Lpn>;

type LeaseCurrency = Cro;
type LeaseCoin = Coin<LeaseCurrency>;

type PaymentCurrency = Atom;
type PaymentCoin = Coin<PaymentCurrency>;

const DOWNPAYMENT: u128 = 1_000_000_000_000;

fn create_payment_coin(amount: u128) -> PaymentCoin {
    PaymentCoin::new(amount)
}

fn price_lpn_of<C>() -> Price<C, Lpn>
where
    C: Currency,
{
    Price::identity()
}

fn feed_price<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms>,
) {
    let lease_price = price_lpn_of::<LeaseCurrency>();
    oracle_mod::feed_price_pair(test_case, Addr::unchecked(ADMIN), lease_price);

    let payment_price = price_lpn_of::<PaymentCurrency>();
    oracle_mod::feed_price_pair(test_case, Addr::unchecked(ADMIN), payment_price);
}

fn create_test_case<InitFundsC>() -> TestCase<(), Addr, Addr, Addr, Addr, Addr, Addr>
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

    oracle_mod::add_feeder(&mut test_case, ADMIN);

    feed_price(&mut test_case);

    test_case
}

fn calculate_interest(principal: Coin<Lpn>, interest_rate: Percent, duration: u64) -> Coin<Lpn> {
    InterestPeriod::with_interest(interest_rate)
        .and_period(Period::from_length(
            Timestamp::default(),
            Duration::from_nanos(duration),
        ))
        .interest(principal)
}

fn open_lease<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms, DownpaymentC>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: Coin<DownpaymentC>,
    max_ltd: Option<Percent>,
) -> Addr
where
    DownpaymentC: Currency,
{
    try_init_lease(test_case, downpayment, max_ltd);

    let lease = get_lease_address(test_case);

    let quote = leaser_mod::query_quote::<DownpaymentC, LeaseCurrency>(
        &mut test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        max_ltd,
    );
    let exp_borrow = TryInto::<Coin<Lpn>>::try_into(quote.borrow).unwrap();
    let exp_lease = TryInto::<Coin<LeaseCurrency>>::try_into(quote.total).unwrap();

    lease_mod::complete_initialization::<Lpn, DownpaymentC, LeaseCurrency>(
        &mut test_case.app,
        TestCase::LEASER_CONNECTION_ID,
        lease.clone(),
        downpayment,
        exp_borrow,
        exp_lease,
    );

    lease
}

fn try_init_lease<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms, D>(
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

fn get_lease_address<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms>(
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

fn construct_response(data: Binary) -> NeutronSudoMsg {
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

fn quote_borrow<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: PaymentCoin,
) -> LpnCoin {
    LpnCoin::try_from(quote_query(test_case, downpayment).borrow).unwrap()
}

fn quote_query<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms, DownpaymentC>(
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

fn state_query<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: &str,
) -> StateResponse {
    test_case
        .app
        .query()
        .query_wasm_smart(contract_addr, &StateQuery {})
        .unwrap()
}

fn expected_open_state<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms, DownpaymentC>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: Coin<DownpaymentC>,
    payments: PaymentCoin,
    last_paid: Timestamp,
    current_period_start: Timestamp,
    now: Timestamp,
) -> StateResponse
where
    DownpaymentC: Currency,
{
    let quote_result = quote_query(test_case, downpayment);
    let total: LeaseCoin = quote_result.total.try_into().unwrap();
    let total_lpn: LpnCoin = price::total(total, price_lpn_of::<LeaseCurrency>());
    let expected: LpnCoin = total_lpn
        - price::total(downpayment, price_lpn_of::<DownpaymentC>())
        - price::total(payments, price_lpn_of::<PaymentCurrency>());
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

fn expected_newly_opened_state<
    Dispatcher,
    Treasury,
    Profit,
    Lpp,
    Oracle,
    TimeAlarms,
    DownpaymentC,
>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>,
    downpayment: Coin<DownpaymentC>,
    payments: PaymentCoin,
) -> StateResponse
where
    DownpaymentC: Currency,
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

#[test]
#[should_panic = "[Lease] No payment sent"]
fn open_zero_downpayment() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(0);
    try_init_lease(&mut test_case, downpayment, None);
}

#[test]
fn open_downpayment_lease_currency() {
    let mut test_case = create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(100);
    let lease = open_lease(&mut test_case, downpayment, None);

    let query_result = state_query(&test_case, &lease.into_string());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_payment_coin(0));
    assert_eq!(query_result, expected_result);
}

#[test]
fn state_opened_when_no_payments() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease = open_lease(&mut test_case, downpayment, None);

    let query_result = state_query(&test_case, &lease.into_string());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_payment_coin(0));
    assert_eq!(query_result, expected_result);
}

#[test]
fn state_opened_when_partially_paid() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);

    let quote_result = quote_query(&test_case, downpayment);
    let amount: LpnCoin = quote_result.borrow.try_into().unwrap();
    let partial_payment = create_payment_coin(u128::from(amount) / 2);
    let expected_result = expected_newly_opened_state(&test_case, downpayment, partial_payment);

    let lease_address = open_lease(&mut test_case, downpayment, None);
    repay_mod::repay(&mut test_case, lease_address.clone(), partial_payment);

    let query_result = state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn state_opened_when_partially_paid_after_time() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = create_payment_coin(DOWNPAYMENT);

    let lease_address = open_lease(&mut test_case, downpayment, None);

    test_case.app.time_shift(Duration::from_nanos(
        LeaserInstantiator::REPAYMENT_PERIOD.nanos() >> 1,
    ));

    let query_result = state_query(&test_case, lease_address.as_ref());

    let StateResponse::Opened {
        previous_margin_due,
        previous_interest_due,
        current_margin_due,
        ..
    } = query_result else {
        unreachable!()
    };

    feed_price(&mut test_case);

    let current_margin_to_pay: LpnCoin = LpnCoin::try_from(current_margin_due)
        .unwrap()
        .checked_div(2)
        .unwrap();

    repay_mod::repay(
        &mut test_case,
        lease_address.clone(),
        price::total(
            LpnCoin::try_from(previous_margin_due).unwrap()
                + LpnCoin::try_from(previous_interest_due).unwrap()
                + current_margin_to_pay,
            price_lpn_of::<PaymentCurrency>().inv(),
        ),
    );

    let query_result = state_query(&test_case, lease_address.as_str());

    if let StateResponse::Opened {
        previous_margin_due,
        previous_interest_due,
        ..
    } = query_result
    {
        assert!(
            previous_margin_due.is_zero(),
            "Expected 0 for margin interest due, got {}",
            previous_margin_due.amount()
        );

        assert!(
            previous_interest_due.is_zero(),
            "Expected 0 for interest due, got {}",
            previous_interest_due.amount()
        );
    } else {
        unreachable!()
    }
}

#[test]
fn state_paid() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);
    let borrowed: PaymentCoin =
        price::total(quote_borrow(&test_case, downpayment), price_lpn_of().inv());

    repay_mod::repay(&mut test_case, lease_address.clone(), borrowed);

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ price_lpn_of(),
        ),
        /* LPN -> Lease */ price_lpn_of().inv(),
    );
    let expected_result = StateResponse::Paid {
        amount: LeaseCoin::into(expected_amount),
        in_progress: None,
    };
    let query_result = state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn state_paid_with_max_ltv() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let percent = Percent::from_percent(10);
    let borrowed = Coin::new(percent.of(DOWNPAYMENT));
    let lease_address = open_lease(&mut test_case, downpayment, Some(percent));

    let expected_result = StateResponse::Opened {
        amount: (Percent::HUNDRED + percent)
            .of(price::total(
                downpayment,
                Price::<PaymentCurrency, LeaseCurrency>::identity(),
            ))
            .into(),
        loan_interest_rate: Percent::from_permille(70),
        margin_interest_rate: Percent::from_permille(30),
        principal_due: price::total(percent.of(downpayment), price_lpn_of()).into(),
        previous_margin_due: LpnCoin::ZERO.into(),
        previous_interest_due: LpnCoin::ZERO.into(),
        current_margin_due: LpnCoin::ZERO.into(),
        current_interest_due: LpnCoin::ZERO.into(),
        validity: Timestamp::from_nanos(1537237454879305533),
        in_progress: None,
    };
    let query_result = state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);

    repay_mod::repay(&mut test_case, lease_address.clone(), borrowed);

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ price_lpn_of(),
        ),
        /* LPN -> Lease */ price_lpn_of().inv(),
    );
    let expected_result = StateResponse::Paid {
        amount: LeaseCoin::into(expected_amount),
        in_progress: None,
    };
    let query_result = state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn state_paid_when_overpaid() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);
    let borrowed: PaymentCoin = price::total(
        quote_borrow(&test_case, downpayment),
        /* LPN -> Payment */ price_lpn_of().inv(),
    );

    let overpayment = create_payment_coin(5);
    let payment: PaymentCoin = borrowed + overpayment;

    repay_mod::repay(&mut test_case, lease_address.clone(), payment);

    let query_result = state_query(&test_case, lease_address.as_str());

    assert_eq!(
        test_case
            .app
            .query()
            .query_all_balances(lease_address)
            .unwrap(),
        &[cwcoin::<Lpn, Amount>(overpayment.into())],
    );

    assert_eq!(
        test_case.app.query().query_all_balances("ica0").unwrap(),
        &[cwcoin::<LeaseCurrency, _>(price::total(
            price::total(downpayment + borrowed, price_lpn_of()),
            price_lpn_of().inv()
        ))],
    );

    assert_eq!(
        query_result,
        StateResponse::Paid {
            amount: LeaseCoin::into(price::total(
                price::total(downpayment + borrowed, price_lpn_of()),
                price_lpn_of().inv(),
            )),
            in_progress: None
        }
    );
}

fn liquidation_warning(base: LeaseCoin, quote: LpnCoin, liability: Percent, level: &str) {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);

    oracle_mod::feed_price(&mut test_case, Addr::unchecked(ADMIN), base, quote);

    let response: AppResponse = test_case
        .app
        .execute(
            test_case.address_book.oracle().clone(),
            lease_address,
            &ExecuteMsg::PriceAlarm(),
            // &cwcoin::<LeaseCurrency, _>(10000),
            &[],
        )
        .unwrap()
        .unwrap_response();

    let event = response
        .events
        .iter()
        .find(|event| event.ty == "wasm-ls-liquidation-warning")
        .expect("No liquidation warning emitted!");

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "customer")
        .expect("Customer attribute not present!");

    assert_eq!(attribute.value, USER);

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "ltv")
        .expect("LTV attribute not present!");

    assert_eq!(attribute.value, liability.units().to_string());

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "level")
        .expect("Level attribute not present!");

    assert_eq!(attribute.value, level);

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "lease-asset")
        .expect("Lease Asset attribute not present!");

    assert_eq!(&attribute.value, LeaseCurrency::TICKER);
}

#[test]
#[should_panic = "No liquidation warning emitted!"]
fn liquidation_warning_price_0() {
    liquidation_warning(
        2085713.into(),
        1857159.into(),
        LeaserInstantiator::liability().max(), //not used
        "N/A",
    );
}

#[test]
fn liquidation_warning_price_1() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1827159.into(),
        LeaserInstantiator::liability().first_liq_warn(),
        "1",
    );
}

#[test]
fn liquidation_warning_price_2() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1757159.into(),
        LeaserInstantiator::liability().second_liq_warn(),
        "2",
    );
}

#[test]
fn liquidation_warning_price_3() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1707159.into(),
        LeaserInstantiator::liability().third_liq_warn(),
        "3",
    );
}

fn liquidation_time_alarm(time_pass: Duration, liquidation_amount: Option<LeaseCoin>) {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);

    let StateResponse::Opened {
        amount: lease_amount,
        ..
    } = state_query(&test_case, lease_address.as_ref()) else {
        unreachable!()
    };
    let lease_amount: LeaseCoin = lease_amount.try_into().unwrap();

    test_case.app.time_shift(time_pass);

    feed_price(&mut test_case);

    let mut response: ResponseWithInterChainMsgs<'_, AppResponse> = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            lease_address.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    if liquidation_amount.is_some() {
        response.expect_submit_tx(TestCase::LEASER_CONNECTION_ID, "0", 1);
    }

    let liquidation_start_response: AppResponse = response.unwrap_response();

    let Some(liquidation_amount): Option<LeaseCoin> = liquidation_amount else {
        assert!(!liquidation_start_response.has_event(&Event::new("wasm-ls-liquidation-start")));

        return;
    };

    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            Addr::unchecked(ADMIN),
            &[cwcoin(liquidation_amount)],
        )
        .unwrap();

    let liquidated_in_lpn: LpnCoin = price::total(liquidation_amount, price_lpn_of());

    test_case.send_funds_from_admin(Addr::unchecked("ica0"), &[cwcoin(liquidated_in_lpn)]);

    let mut response: ResponseWithInterChainMsgs<'_, ()> = test_case
        .app
        .sudo(
            lease_address.clone(),
            &sdk::neutron_sdk::sudo::msg::SudoMsg::Response {
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
                data: Binary(platform::trx::encode_msg_responses(
                    [platform::trx::encode_msg_response(
                        MsgSwapExactAmountInResponse {
                            token_out_amount: Amount::from(liquidation_amount).to_string(),
                        },
                        MsgSwapExactAmountIn::TYPE_URL,
                    )]
                    .into_iter(),
                )),
            },
        )
        .unwrap()
        .ignore_response();

    response.expect_submit_tx(TestCase::LEASER_CONNECTION_ID, "0", 1);

    () = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            lease_address.clone(),
            &[cwcoin(liquidated_in_lpn)],
        )
        .unwrap();

    () = test_case
        .app
        .sudo(
            lease_address.clone(),
            &sdk::neutron_sdk::sudo::msg::SudoMsg::Response {
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
                data: Binary::default(),
            },
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let liquidation_attributes: HashMap<String, String> = liquidation_start_response
        .events
        .into_iter()
        .find(|event| event.ty == "wasm-ls-liquidation-start")
        .expect("No liquidation emitted!")
        .attributes
        .into_iter()
        .map(|attribute| (attribute.key, attribute.value))
        .collect();

    let query_result = state_query(&test_case, lease_address.as_str());

    let liquidated_amount: LeaseCoin = liquidation_attributes["amount-amount"]
        .parse::<Amount>()
        .unwrap()
        .into();

    assert_eq!(liquidated_amount, liquidation_amount);

    if let StateResponse::Opened {
        amount,
        previous_margin_due,
        previous_interest_due,
        ..
    } = query_result
    {
        assert_eq!(
            LeaseCoin::try_from(amount).unwrap(),
            lease_amount - liquidated_amount
        );

        assert!(previous_margin_due.is_zero());

        assert!(previous_interest_due.is_zero());
    }
}

#[test]
fn liquidation_time_alarm_0() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD - Duration::from_nanos(1),
        None,
    );
}

#[test]
fn liquidation_time_alarm_1() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::GRACE_PERIOD
            - Duration::from_nanos(1),
        None,
    );
}

#[test]
fn liquidation_time_alarm_2() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::GRACE_PERIOD,
        Some(LeaseCoin::new(45792563600)),
    );
}

#[test]
fn compare_state_with_manual_calculation() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);
    let quote_result = dbg!(quote_query(&test_case, downpayment));

    let query_result = state_query(&test_case, lease_address.as_ref());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_payment_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::REPAYMENT_PERIOD
            - Duration::from_nanos(1),
    );

    let query_result = state_query(&test_case, &lease_address.into_string());
    let expected_result = StateResponse::Opened {
        amount: Coin::<LeaseCurrency>::new(DOWNPAYMENT + 1_857_142_857_142).into(),
        loan_interest_rate: quote_result.annual_interest_rate,
        margin_interest_rate: quote_result.annual_interest_rate_margin,
        principal_due: Coin::<Lpn>::new(1_857_142_857_142).into(),
        previous_margin_due: LpnCoin::new(13_737_769_080).into(),
        previous_interest_due: LpnCoin::new(32_054_794_520).into(),
        current_margin_due: LpnCoin::new(13_737_769_080).into(),
        current_interest_due: LpnCoin::new(32_054_794_520).into(),
        validity: block_time(&test_case),
        in_progress: None,
    };

    assert_eq!(dbg!(query_result), expected_result);
}

#[test]
fn compare_state_with_lpp_state_implicit_time() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);

    let query_result = state_query(&test_case, lease_address.as_ref());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_payment_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::REPAYMENT_PERIOD
            - Duration::from_nanos(1),
    );

    let loan_resp: lpp::msg::LoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &lpp::msg::QueryMsg::Loan {
                lease_addr: lease_address.clone(),
            },
        )
        .unwrap();

    let query_result = if let StateResponse::Opened {
        principal_due,
        previous_interest_due,
        current_interest_due,
        ..
    } = state_query(&test_case, &lease_address.into_string())
    {
        (
            LpnCoin::try_from(principal_due).unwrap(),
            LpnCoin::try_from(previous_interest_due).unwrap()
                + LpnCoin::try_from(current_interest_due).unwrap(),
        )
    } else {
        unreachable!();
    };

    assert_eq!(
        query_result,
        (
            loan_resp.principal_due,
            loan_resp.interest_due(test_case.app.block_info().time)
        )
    );
}

#[test]
fn compare_state_with_lpp_state_explicit_time() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);

    let query_result = state_query(&test_case, lease_address.as_ref());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_payment_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::REPAYMENT_PERIOD
            - Duration::from_nanos(1),
    );

    let loan: lpp::msg::LoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &lpp::msg::QueryMsg::Loan {
                lease_addr: lease_address.clone(),
            },
        )
        .unwrap();

    let query_result = if let StateResponse::Opened {
        previous_interest_due,
        current_interest_due,
        ..
    } = state_query(&test_case, &lease_address.into_string())
    {
        LpnCoin::try_from(previous_interest_due).unwrap()
            + LpnCoin::try_from(current_interest_due).unwrap()
    } else {
        unreachable!();
    };

    assert_eq!(query_result, loan.interest_due(block_time(&test_case)));
}

#[test]
fn state_closed() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);
    let borrowed: PaymentCoin = price::total(
        quote_borrow(&test_case, downpayment),
        price_lpn_of::<PaymentCurrency>().inv(),
    );
    let lease_amount: LeaseCoin = price::total(
        price::total(downpayment, price_lpn_of()) + quote_borrow(&test_case, downpayment),
        price_lpn_of::<LeaseCurrency>().inv(),
    );
    repay_mod::repay(&mut test_case, lease_address.clone(), borrowed);

    let user_balance: LeaseCoin =
        platform::bank::balance(&Addr::unchecked(USER), &test_case.app.query()).unwrap();

    close_mod::close(
        &mut test_case,
        lease_address.clone(),
        &[cwcoin(lease_amount)],
    );

    let query_result = state_query(&test_case, lease_address.as_str());
    let expected_result = StateResponse::Closed();

    assert_eq!(query_result, expected_result);

    assert_eq!(
        platform::bank::balance(&Addr::unchecked(USER), &test_case.app.query()).unwrap(),
        user_balance + lease_amount
    );
}

fn block_time<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
) -> Timestamp {
    test_case.app.block_info().time
}
