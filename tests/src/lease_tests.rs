use std::collections::{HashMap, HashSet, VecDeque};

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
};
use lease::api::{ExecuteMsg, StateQuery, StateResponse};
use leaser::msg::{QueryMsg, QuoteResponse};
use sdk::{
    cosmwasm_ext::CustomMsg,
    cosmwasm_std::{coin, Addr, Timestamp},
    cw_multi_test::AppResponse,
};

use crate::common::{
    cwcoin,
    lease_wrapper::complete_lease_initialization,
    leaser_wrapper::{self, LeaserWrapper},
    oracle_wrapper::{
        add_feeder, feed_a_price as oracle_feed_a_price, feed_price as oracle_feed_price,
    },
    test_case::{Builder as TestCaseBuilder, TestCase},
    ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
};

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
    oracle_feed_a_price(test_case, Addr::unchecked(ADMIN), lease_price);

    let payment_price = price_lpn_of::<PaymentCurrency>();
    oracle_feed_a_price(test_case, Addr::unchecked(ADMIN), payment_price);
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

    add_feeder(&mut test_case, ADMIN);

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
    let messages = try_init_lease(test_case, downpayment, max_ltd);

    let lease = get_lease_address(test_case);

    let quote = leaser_wrapper::query_quote::<DownpaymentC, LeaseCurrency>(
        &mut test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
    );
    let exp_borrow = TryInto::<Coin<Lpn>>::try_into(quote.borrow).unwrap();
    let exp_lease = TryInto::<Coin<LeaseCurrency>>::try_into(quote.total).unwrap();

    complete_lease_initialization::<Lpn, DownpaymentC, LeaseCurrency>(
        &mut test_case.app,
        &lease,
        messages,
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
) -> VecDeque<CustomMsg>
where
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

    let messages: VecDeque<CustomMsg> = response.iter().collect();

    let _: AppResponse = response.unwrap_response();

    messages
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

fn repay<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: &Addr,
    payment: PaymentCoin,
) -> AppResponse {
    test_case
        .app
        .execute(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Repay {},
            &[cwcoin::<PaymentCurrency, _>(payment)],
        )
        .unwrap()
        .unwrap_response()
}

fn close<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: &Addr,
) -> AppResponse {
    test_case
        .app
        .execute(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Close {},
            &[],
        )
        .unwrap()
        .unwrap_response()
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
    contract_addr: &String,
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
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_opened_when_partially_paid() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);

    let quote_result = quote_query(&test_case, downpayment);
    let amount: LpnCoin = quote_result.borrow.try_into().unwrap();
    let partial_payment = create_payment_coin(u128::from(amount) / 2);
    let expected_result = expected_newly_opened_state(&test_case, downpayment, partial_payment);

    let lease_address = open_lease(&mut test_case, downpayment, None);
    repay(&mut test_case, &lease_address, partial_payment);

    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(query_result, expected_result);
}

#[test]
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_opened_when_partially_paid_after_time() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);

    let lease_address = open_lease(&mut test_case, downpayment, None);

    test_case.app.time_shift(Duration::from_nanos(
        LeaserWrapper::REPAYMENT_PERIOD.nanos() >> 1,
    ));

    let query_result = state_query(&test_case, &lease_address.to_string());

    if let StateResponse::Opened {
        previous_margin_due,
        previous_interest_due,
        current_margin_due,
        ..
    } = query_result
    {
        let current_margin_to_pay: LpnCoin = LpnCoin::try_from(current_margin_due)
            .unwrap()
            .checked_div(2)
            .unwrap();
        repay(
            &mut test_case,
            &lease_address,
            price::total(
                LpnCoin::try_from(previous_margin_due).unwrap()
                    + LpnCoin::try_from(previous_interest_due).unwrap()
                    + current_margin_to_pay,
                price_lpn_of::<PaymentCurrency>().inv(),
            ),
        );
    } else {
        unreachable!();
    }

    let query_result = state_query(&test_case, &lease_address.into_string());

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
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_paid() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);
    let borrowed = price::total(
        quote_borrow(&test_case, downpayment),
        price_lpn_of::<PaymentCurrency>().inv(),
    );

    repay(&mut test_case, &lease_address, borrowed);

    let expected_amount = downpayment + borrowed;
    let expected_result = StateResponse::Paid {
        amount: expected_amount.into(),
        in_progress: None,
    };
    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(query_result, expected_result);
}

#[test]
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_paid_with_max_ltv() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let percent = Percent::from_percent(10);
    let borrowed = Coin::new(percent.of(DOWNPAYMENT));
    let lease_address = open_lease(&mut test_case, downpayment, Some(percent));

    repay(&mut test_case, &lease_address, borrowed);

    let expected_amount = downpayment + borrowed;
    let expected_result = StateResponse::Paid {
        amount: expected_amount.into(),
        in_progress: None,
    };
    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(query_result, expected_result);
}

#[test]
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_paid_when_overpaid() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);
    let borrowed = price::total(
        quote_borrow(&test_case, downpayment),
        price_lpn_of::<PaymentCurrency>().inv(),
    );

    let overpayment = create_payment_coin(5);
    let payment = borrowed + overpayment;

    repay(&mut test_case, &lease_address, payment);

    let query_result = state_query(&test_case, &lease_address.clone().into_string());

    let balance = test_case
        .app
        .query()
        .query_all_balances(lease_address)
        .unwrap();
    assert_eq!(
        balance,
        &[cwcoin::<PaymentCurrency, _>(downpayment + payment)],
    );

    assert_eq!(
        query_result,
        StateResponse::Paid {
            amount: (downpayment + borrowed).into(),
            in_progress: None
        }
    );
}

fn liquidation_warning(base: LeaseCoin, quote: LpnCoin, liability: Percent, level: &str) {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);

    oracle_feed_price(&mut test_case, Addr::unchecked(ADMIN), base, quote);

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
        LeaserWrapper::liability().max(), //not used
        "N/A",
    );
}

#[test]
#[ignore = "liquidations on price have been disabled until https://github.com/nolus-protocol/nolus-money-market/issues/49 gets implemented"]
fn liquidation_warning_price_1() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1827159.into(),
        LeaserWrapper::liability().first_liq_warn(),
        "1",
    );
}

#[test]
#[ignore = "liquidations on price have been disabled until https://github.com/nolus-protocol/nolus-money-market/issues/49 gets implemented"]
fn liquidation_warning_price_2() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1757159.into(),
        LeaserWrapper::liability().second_liq_warn(),
        "2",
    );
}

#[test]
#[ignore = "liquidations on price have been disabled until https://github.com/nolus-protocol/nolus-money-market/issues/49 gets implemented"]
fn liquidation_warning_price_3() {
    liquidation_warning(
        // ref: 2085713
        2085713.into(),
        // ref: 1857159
        1707159.into(),
        LeaserWrapper::liability().third_liq_warn(),
        "3",
    );
}

fn liquidation_time_alarm(time_pass: Duration) {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);

    let lease_amount = if let StateResponse::Opened {
        amount: lease_amount,
        ..
    } = state_query(&test_case, &lease_address.to_string())
    {
        LeaseCoin::try_from(lease_amount).unwrap()
    } else {
        unreachable!()
    };

    test_case.app.time_shift(time_pass);

    feed_price(&mut test_case);

    let response: AppResponse = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            lease_address.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .unwrap_response();

    let liquidation_attributes: HashMap<String, String> = response
        .events
        .into_iter()
        .find(|event| event.ty == "wasm-ls-liquidation")
        .expect("No liquidation emitted!")
        .attributes
        .into_iter()
        .map(|attribute| (attribute.key, attribute.value))
        .collect();

    let query_result = state_query(&test_case, &lease_address.into_string());

    if let StateResponse::Opened {
        amount,
        previous_margin_due,
        previous_interest_due,
        ..
    } = query_result
    {
        assert_eq!(
            LeaseCoin::try_from(amount).unwrap(),
            lease_amount
                - liquidation_attributes["liquidation-amount"]
                    .parse::<Amount>()
                    .unwrap()
                    .into()
        );

        assert!(previous_margin_due.is_zero());

        assert!(previous_interest_due.is_zero());
    }
}

#[test]
#[should_panic = "No liquidation emitted!"]
fn liquidation_time_alarm_0() {
    liquidation_time_alarm(LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1));
}

#[test]
#[should_panic = "No liquidation emitted!"]
fn liquidation_time_alarm_1() {
    liquidation_time_alarm(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::GRACE_PERIOD - Duration::from_nanos(1),
    );
}

#[test]
#[ignore = "liquidations on time have been disabled until https://github.com/nolus-protocol/nolus-money-market/issues/49 gets implemented"]
fn liquidation_time_alarm_2() {
    liquidation_time_alarm(LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::GRACE_PERIOD);
}

#[test]
fn compare_state_with_manual_calculation() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);
    let quote_result = dbg!(quote_query(&test_case, downpayment));

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_payment_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1),
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

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_payment_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1),
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

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_payment_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1),
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
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_closed() {
    let mut test_case = create_test_case::<PaymentCurrency>();
    let downpayment = create_payment_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment, None);
    let borrowed = price::total(
        quote_borrow(&test_case, downpayment),
        price_lpn_of::<PaymentCurrency>().inv(),
    );
    repay(&mut test_case, &lease_address, borrowed);
    close(&mut test_case, &lease_address);

    let query_result = state_query(&test_case, &lease_address.into_string());
    let expected_result = StateResponse::Closed();

    assert_eq!(query_result, expected_result);
}

fn block_time<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
) -> Timestamp {
    test_case.app.block_info().time
}
