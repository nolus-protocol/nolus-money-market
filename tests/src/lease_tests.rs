use std::collections::{HashMap, HashSet};

use currency::lpn::Usdc;
use finance::{
    coin::{Amount, Coin},
    currency::Currency as _,
    duration::Duration,
    interest::InterestPeriod,
    percent::Percent,
    price::dto::PriceDTO,
};
use lease::api::{ExecuteMsg, StateQuery, StateResponse};
use leaser::msg::{QueryMsg, QuoteResponse};
use marketprice::SpotPrice;
use sdk::{cosmwasm_std::coin, neutron_sdk::sudo::msg::SudoMsg};
use sdk::{
    cosmwasm_std::{Addr, Timestamp},
    cw_multi_test::{AppResponse, Executor},
};

use crate::common::{
    cwcoin, cwcoins, leaser_wrapper::LeaserWrapper, test_case::TestCase, AppExt,
    ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
};

type Lpn = Usdc;
type LpnCoin = Coin<Lpn>;

type LeaseCurrency = Lpn;
type LeaseCoin = Coin<LeaseCurrency>;

const DOWNPAYMENT: u128 = 1_000_000_000_000;

fn create_coin(amount: u128) -> LeaseCoin {
    Coin::<LeaseCurrency>::new(amount)
}

fn create_test_case() -> TestCase<Lpn> {
    let mut test_case = TestCase::with_reserve(&[
        cwcoin::<LeaseCurrency, _>(10_000_000_000_000_000_000_000_000_000),
        cwcoin::<Lpn, _>(10_000_000_000_000_000_000_000_000_000),
    ]);
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

    test_case
}

fn calculate_interest(principal: LeaseCoin, interest_rate: Percent, duration: u64) -> LeaseCoin {
    InterestPeriod::with_interest(interest_rate)
        .from(Timestamp::from_nanos(0))
        .spanning(Duration::from_nanos(duration))
        .interest(principal)
}

fn open_lease(test_case: &mut TestCase<Lpn>, value: LeaseCoin) -> Addr {
    try_init_lease(test_case, value).unwrap();

    let lease = get_lease_address(test_case);

    try_transfer_out(test_case, &lease).unwrap();

    lease
}

fn try_init_lease(
    test_case: &mut TestCase<Lpn>,
    value: LeaseCoin,
) -> Result<AppResponse, anyhow::Error> {
    test_case.app.execute_contract(
        Addr::unchecked(USER),
        test_case.leaser_addr.clone().unwrap(),
        &leaser::msg::ExecuteMsg::OpenLease {
            currency: LeaseCurrency::TICKER.into(),
        },
        &cwcoins::<LeaseCurrency, _>(value),
    )
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

fn try_transfer_out(
    test_case: &mut TestCase<Lpn>,
    lease: &Addr,
) -> Result<AppResponse, anyhow::Error> {
    let dex_address = "dex_addr_lease";
    let version = format!(
        "{{\"version\":\"\", \"controller_connection_id\":\"\", 
                                \"host_connection_id\":\"\", \"address\":\"{}\", \"encoding\":\"\",
                                \"tx_type\":\"\" }}",
        dex_address
    );
    test_case.app.wasm_sudo(
        lease.to_owned(),
        &SudoMsg::OpenAck {
            port_id: "".into(),
            channel_id: "".into(),
            counterparty_channel_id: "".into(),
            counterparty_version: version,
        },
    )
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

fn quote_borrow(test_case: &TestCase<Lpn>, amount: LeaseCoin) -> LeaseCoin {
    quote_query(test_case, amount).borrow.try_into().unwrap()
}

fn quote_query(test_case: &TestCase<Lpn>, amount: LeaseCoin) -> QuoteResponse {
    test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: amount.into(),
                lease_asset: Lpn::TICKER.into(),
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
    let expected = total - downpayment - payments;
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
fn open_zero_downpayment() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(0);
    let res = try_init_lease(&mut test_case, downpayment);
    assert!(res.is_err());
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn state_opened_when_no_payments() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let expected_result = expected_newly_opened_state(&test_case, downpayment, create_coin(0));
    let lease_address = open_lease(&mut test_case, downpayment);

    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(query_result, expected_result);
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn state_opened_when_partially_paid() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);

    let quote_result = quote_query(&test_case, downpayment);
    let amount: Coin<Usdc> = quote_result.borrow.try_into().unwrap();
    let partial_payment = create_coin(u128::from(amount) / 2);
    let expected_result = expected_newly_opened_state(&test_case, downpayment, partial_payment);

    let lease_address = open_lease(&mut test_case, downpayment);
    repay(&mut test_case, &lease_address, partial_payment);

    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn state_opened_when_partially_paid_after_time() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);

    let lease_address = open_lease(&mut test_case, downpayment);

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
            LpnCoin::try_from(previous_margin_due).unwrap()
                + previous_interest_due.try_into().unwrap()
                + current_margin_to_pay,
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
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn state_paid() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);
    let borrowed = quote_borrow(&test_case, downpayment);

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
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn state_paid_when_overpaid() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);
    let borrowed = quote_borrow(&test_case, downpayment);

    let overpayment = create_coin(5);
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
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn price_alarm_unauthorized() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);

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

fn liquidation_warning(price: SpotPrice, percent: Percent, level: &str) {
    const DOWNPAYMENT: u128 = 1_000_000;

    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);

    let response = test_case
        .app
        .execute_contract(
            test_case.oracle.unwrap(),
            lease_address,
            &ExecuteMsg::PriceAlarm(),
            &cwcoins::<LeaseCurrency, _>(10000),
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

    assert_eq!(&attribute.value, price.quote().ticker());
}

#[test]
#[should_panic = "No liquidation warning emitted!"]
#[ignore = "No support for currencies different than LPN"]
fn liquidation_warning_price_0() {
    liquidation_warning(
        PriceDTO::new(create_coin(2085713).into(), create_coin(1857159).into()),
        LeaserWrapper::liability().healthy_percent(),
        "N/A",
    );
}

#[test]
#[ignore = "No support for currencies different than LPN"]
fn liquidation_warning_price_1() {
    liquidation_warning(
        PriceDTO::new(
            create_coin(2085713).into(), // ref: 2085713
            create_coin(137159).into(),  // ref: 1857159
        ),
        LeaserWrapper::liability().first_liq_warn_percent(),
        "1",
    );
}

#[test]
#[ignore = "No support for currencies different than LPN"]
fn liquidation_warning_price_2() {
    liquidation_warning(
        PriceDTO::new(
            create_coin(2085713).into(), // ref: 2085713
            create_coin(1757159).into(), // ref: 1857159
        ),
        LeaserWrapper::liability().second_liq_warn_percent(),
        "2",
    );
}

#[test]
#[ignore = "No support for currencies different than LPN"]
fn liquidation_warning_price_3() {
    liquidation_warning(
        PriceDTO::new(
            create_coin(2085713).into(), // ref: 2085713
            create_coin(1707159).into(), // ref: 1857159
        ),
        LeaserWrapper::liability().third_liq_warn_percent(),
        "3",
    );
}

fn liquidation_time_alarm(time_pass: Duration) {
    const DOWNPAYMENT: u128 = 1_000_000;

    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);

    let base_amount = if let StateResponse::Opened { amount, .. } =
        state_query(&test_case, &lease_address.to_string())
    {
        LeaseCoin::try_from(amount).unwrap()
    } else {
        unreachable!()
    };

    test_case.app.time_shift(time_pass);

    let response = test_case
        .app
        .execute_contract(
            test_case.timealarms.clone().unwrap(),
            lease_address.clone(),
            &ExecuteMsg::TimeAlarm(test_case.app.block_info().time),
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
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn liquidation_time_alarm_0() {
    liquidation_time_alarm(LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1));
}

#[test]
#[should_panic = "No liquidation emitted!"]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn liquidation_time_alarm_1() {
    liquidation_time_alarm(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::GRACE_PERIOD - Duration::from_nanos(1),
    );
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn liquidation_time_alarm_2() {
    liquidation_time_alarm(LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::GRACE_PERIOD);
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn compare_state_with_manual_calculation() {
    const DOWNPAYMENT: u128 = 1_000_000;

    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);
    let quote_result = dbg!(quote_query(&test_case, downpayment));

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result = expected_newly_opened_state(&test_case, downpayment, create_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1),
    );

    let query_result = state_query(&test_case, &lease_address.into_string());
    let expected_result = StateResponse::Opened {
        amount: Coin::<LeaseCurrency>::new(1_000_000 + 1_857_142).into(),
        loan_interest_rate: quote_result.annual_interest_rate,
        margin_interest_rate: quote_result.annual_interest_rate_margin,
        principal_due: Coin::<Lpn>::new(1_857_142).into(),
        previous_margin_due: create_coin(13_737).into(),
        previous_interest_due: create_coin(32_054).into(),
        current_margin_due: create_coin(13_737).into(),
        current_interest_due: create_coin(32_055).into(),
        validity: block_time(&test_case),
        in_progress: None,
    };

    assert_eq!(dbg!(query_result), expected_result);
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn compare_state_with_lpp_state_implicit_time() {
    const DOWNPAYMENT: u128 = 1_000_000;

    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result = expected_newly_opened_state(&test_case, downpayment, create_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1),
    );

    let loan_resp: lpp::msg::LoanResponse<LeaseCurrency> = test_case
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
        (loan_resp.principal_due, loan_resp.interest_due)
    );
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn compare_state_with_lpp_state_explicit_time() {
    const DOWNPAYMENT: u128 = 1_000_000;

    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);

    let query_result = state_query(&test_case, &lease_address.to_string());
    let expected_result = expected_newly_opened_state(&test_case, downpayment, create_coin(0));

    assert_eq!(dbg!(query_result), expected_result);

    test_case.app.time_shift(
        LeaserWrapper::REPAYMENT_PERIOD + LeaserWrapper::REPAYMENT_PERIOD - Duration::from_nanos(1),
    );

    let lpp::msg::OutstandingInterest::<LeaseCurrency>(loan_resp) = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.lpp_addr.clone().unwrap(),
            &lpp::msg::QueryMsg::LoanOutstandingInterest {
                lease_addr: lease_address.clone(),
                outstanding_time: block_time(&test_case),
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

    assert_eq!(query_result, loan_resp);
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn state_closed() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);
    let borrowed = quote_borrow(&test_case, downpayment);
    repay(&mut test_case, &lease_address, borrowed);
    close(&mut test_case, &lease_address);

    let expected_result = StateResponse::Closed();
    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

fn block_time(test_case: &TestCase<Lpn>) -> Timestamp {
    test_case.app.block_info().time
}
