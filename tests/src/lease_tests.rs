use std::collections::{HashMap, HashSet};

use currency::{lease::Osmo, lpn::Usdc};
use finance::{
    coin::{Amount, Coin},
    currency::Currency as _,
    duration::Duration,
    fraction::Fraction,
    interest::InterestPeriod,
    percent::{NonZeroPercent, NonZeroUnits, Percent},
    price::{self, Price},
};
use lease::api::{ExecuteMsg, StateQuery, StateResponse};
use leaser::msg::{QueryMsg, QuoteResponse};
use sdk::{
    cosmwasm_std::{coin, Addr, Timestamp},
    cw_multi_test::{AppResponse, Executor},
    testing::{new_custom_msg_queue, CustomMessageReceiver},
};

use crate::common::{
    cwcoin, cwcoins,
    lease_wrapper::complete_lease_initialization,
    leaser_wrapper::LeaserWrapper,
    oracle_wrapper::{add_feeder, feed_price as oracle_feed_price},
    test_case::TestCase,
    AppExt, ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
};

type Lpn = Usdc;
type LpnCoin = Coin<Lpn>;

// FIXME; change to `CRO` instead of using `OSMO` after ref. TODO is fixed
//  ref: contracts/lease/src/contract/state/buy_asset.rs:109 @ 5ff50b0302ba07a68b00440d670cdf8135fb1f8b
type LeaseCurrency = Osmo;
type LeaseCoin = Coin<LeaseCurrency>;

const DOWNPAYMENT: u128 = 1_000_000_000_000;

fn create_lease_coin(amount: u128) -> LeaseCoin {
    LeaseCoin::new(amount)
}

fn feed_price(test_case: &mut TestCase<Lpn>) {
    oracle_feed_price(
        test_case,
        &Addr::unchecked(ADMIN),
        LeaseCoin::new(1),
        LpnCoin::new(1),
    );
}

fn create_test_case() -> (TestCase<Lpn>, CustomMessageReceiver) {
    let (neutron_message_sender, neutron_message_receiver) = new_custom_msg_queue();

    let mut test_case = TestCase::with_reserve(
        Some(neutron_message_sender),
        &[
            cwcoin::<LeaseCurrency, _>(10_000_000_000_000_000_000_000_000_000),
            cwcoin::<Lpn, _>(10_000_000_000_000_000_000_000_000_000),
        ],
    );
    test_case.init(
        &Addr::unchecked("user"),
        cwcoins::<LeaseCurrency, _>(1_000_000_000_000_000_000_000_000),
    );
    test_case.init_lpp_with_funds(
        None,
        vec![coin(
            5_000_000_000_000_000_000_000_000_000,
            Lpn::BANK_SYMBOL,
        )],
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
    );
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    add_feeder(&mut test_case, ADMIN);

    feed_price(&mut test_case);

    (test_case, neutron_message_receiver)
}

fn calculate_interest(principal: Coin<Lpn>, interest_rate: Percent, duration: u64) -> Coin<Lpn> {
    InterestPeriod::with_interest(interest_rate)
        .from(Timestamp::from_nanos(0))
        .spanning(Duration::from_nanos(duration))
        .interest(principal)
}

fn open_lease(
    test_case: &mut TestCase<Lpn>,
    neutron_message_receiver: &CustomMessageReceiver,
    value: LeaseCoin,
    max_loan: Option<NonZeroPercent>,
) -> Addr {
    try_init_lease(test_case, value, max_loan);

    let lease = get_lease_address(test_case);

    complete_lease_initialization::<Lpn>(
        &mut test_case.app,
        neutron_message_receiver,
        &lease,
        cwcoin(value),
    );

    lease
}

fn try_init_lease(
    test_case: &mut TestCase<Lpn>,
    value: LeaseCoin,
    max_ltv: Option<NonZeroPercent>,
) {
    test_case
        .app
        .execute_contract(
            Addr::unchecked(USER),
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: LeaseCurrency::TICKER.into(),
                max_ltv,
            },
            &if value.is_zero() {
                vec![]
            } else {
                cwcoins::<LeaseCurrency, _>(value)
            },
        )
        .unwrap();
}

fn get_lease_address(test_case: &TestCase<Lpn>) -> Addr {
    let query_response: HashSet<Addr> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Leases {
                owner: Addr::unchecked(USER),
            },
        )
        .unwrap();
    assert_eq!(query_response.len(), 1);
    query_response.iter().next().unwrap().clone()
}

fn repay(test_case: &mut TestCase<Lpn>, contract_addr: &Addr, value: LeaseCoin) -> AppResponse {
    test_case
        .app
        .execute_contract(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Repay {},
            &cwcoins::<LeaseCurrency, _>(value),
        )
        .unwrap()
}

fn close(test_case: &mut TestCase<Lpn>, contract_addr: &Addr) -> AppResponse {
    test_case
        .app
        .execute_contract(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Close {},
            &[],
        )
        .unwrap()
}

fn quote_borrow(test_case: &TestCase<Lpn>, amount: LeaseCoin) -> LpnCoin {
    LpnCoin::try_from(quote_query(test_case, amount).borrow).unwrap()
}

fn quote_query(test_case: &TestCase<Lpn>, amount: LeaseCoin) -> QuoteResponse {
    test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: amount.into(),
                lease_asset: LeaseCurrency::TICKER.into(),
            },
        )
        .unwrap()
}

fn state_query(test_case: &TestCase<Lpn>, contract_addr: &String) -> StateResponse {
    test_case
        .app
        .wrap()
        .query_wasm_smart(contract_addr, &StateQuery {})
        .unwrap()
}

fn expected_open_state(
    test_case: &TestCase<Lpn>,
    downpayment: LeaseCoin,
    payments: LeaseCoin,
    last_paid: Timestamp,
    current_period_start: Timestamp,
    now: Timestamp,
) -> StateResponse {
    let quote_result = quote_query(test_case, downpayment);
    let total: LeaseCoin = quote_result.total.try_into().unwrap();
    let expected: LpnCoin = price::total(total - downpayment - payments, Price::identity());
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

fn expected_newly_opened_state(
    test_case: &TestCase<Lpn>,
    downpayment: LeaseCoin,
    payments: LeaseCoin,
) -> StateResponse {
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
    let (mut test_case, _) = create_test_case();
    let downpayment = create_lease_coin(0);
    try_init_lease(&mut test_case, downpayment, None);
}

#[test]
fn state_opened_when_no_payments() {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_lease_coin(0));
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);

    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(query_result, expected_result);
}

#[test]
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_opened_when_partially_paid() {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);

    let quote_result = quote_query(&test_case, downpayment);
    let amount: LpnCoin = quote_result.borrow.try_into().unwrap();
    let partial_payment = create_lease_coin(u128::from(amount) / 2);
    let expected_result = expected_newly_opened_state(&test_case, downpayment, partial_payment);

    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);
    repay(&mut test_case, &lease_address, partial_payment);

    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

#[test]
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_opened_when_partially_paid_after_time() {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);

    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);

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
                Price::identity(),
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
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);
    let borrowed = price::total(quote_borrow(&test_case, downpayment), Price::identity());

    repay(&mut test_case, &lease_address, borrowed);

    let expected_amount = downpayment + borrowed;
    let expected_result = StateResponse::Paid {
        amount: expected_amount.into(),
        in_progress: None,
    };
    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

#[test]
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_paid_with_max_loan() {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let percent = NonZeroPercent::from_permille(NonZeroUnits::new(10).unwrap());
    let borrowed = Coin::new(percent.percent().of(DOWNPAYMENT));
    let lease_address = open_lease(
        &mut test_case,
        &neutron_message_receiver,
        downpayment,
        Some(percent),
    );

    repay(&mut test_case, &lease_address, borrowed);

    let expected_amount = downpayment + borrowed;
    let expected_result = StateResponse::Paid {
        amount: expected_amount.into(),
        in_progress: None,
    };
    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

#[test]
#[ignore = "not yet implemented: proceed with TransferOut - Swap - TransferIn before landing to the same Lease::repay call"]
fn state_paid_when_overpaid() {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);
    let borrowed = price::total(quote_borrow(&test_case, downpayment), Price::identity());

    let overpayment = create_lease_coin(5);
    let payment = borrowed + overpayment;

    repay(&mut test_case, &lease_address, payment);

    let query_result = state_query(&test_case, &lease_address.clone().into_string());

    let balance = test_case
        .app
        .wrap()
        .query_all_balances(lease_address)
        .unwrap();
    assert_eq!(cwcoins::<LeaseCurrency, _>(downpayment + payment), balance);

    assert_eq!(
        query_result,
        StateResponse::Paid {
            amount: (downpayment + borrowed).into(),
            in_progress: None
        }
    );
}

#[test]
#[should_panic = "Unauthorized"]
fn price_alarm_unauthorized() {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);

    println!(
        "{:?}",
        test_case
            .app
            .execute_contract(
                Addr::unchecked(ADMIN),
                lease_address,
                &ExecuteMsg::PriceAlarm(),
                &cwcoins::<LeaseCurrency, _>(10000),
            )
            .unwrap()
    );
}

fn liquidation_warning(base: LeaseCoin, quote: LpnCoin, percent: Percent, level: &str) {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);

    oracle_feed_price(&mut test_case, &Addr::unchecked(ADMIN), base, quote);

    let response = test_case
        .app
        .execute_contract(
            test_case.oracle.unwrap(),
            lease_address,
            &ExecuteMsg::PriceAlarm(),
            // &cwcoins::<LeaseCurrency, _>(10000),
            &[],
        )
        .unwrap();

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

    assert_eq!(attribute.value, percent.units().to_string());

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
        LeaserWrapper::liability().healthy_percent(),
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
        LeaserWrapper::liability().first_liq_warn_percent(),
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
        LeaserWrapper::liability().second_liq_warn_percent(),
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
        LeaserWrapper::liability().third_liq_warn_percent(),
        "3",
    );
}

fn liquidation_time_alarm(time_pass: Duration) {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);

    let base_amount = if let StateResponse::Opened { amount, .. } =
        state_query(&test_case, &lease_address.to_string())
    {
        LeaseCoin::try_from(amount).unwrap()
    } else {
        unreachable!()
    };

    test_case.app.time_shift(time_pass);

    feed_price(&mut test_case);

    let response = test_case
        .app
        .execute_contract(
            test_case.timealarms.clone().unwrap(),
            lease_address.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

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
            base_amount
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
fn liquidation_time_alarm_2() {
    liquidation_time_alarm(LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::GRACE_PERIOD);
}

#[test]
fn compare_state_with_manual_calculation() {
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);
    let quote_result = dbg!(quote_query(&test_case, downpayment));

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_lease_coin(0));

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
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_lease_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1),
    );

    let loan_resp: lpp::msg::LoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.lpp_addr.clone().unwrap(),
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
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result =
        expected_newly_opened_state(&test_case, downpayment, create_lease_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1),
    );

    let loan: lpp::msg::LoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.lpp_addr.clone().unwrap(),
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
    let (mut test_case, neutron_message_receiver) = create_test_case();
    let downpayment = create_lease_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, &neutron_message_receiver, downpayment, None);
    let borrowed = price::total(quote_borrow(&test_case, downpayment), Price::identity());
    repay(&mut test_case, &lease_address, borrowed);
    close(&mut test_case, &lease_address);

    let query_result = state_query(&test_case, &lease_address.into_string());
    let expected_result = StateResponse::Closed();

    assert_eq!(query_result, expected_result);
}

fn block_time(test_case: &TestCase<Lpn>) -> Timestamp {
    test_case.app.block_info().time
}
