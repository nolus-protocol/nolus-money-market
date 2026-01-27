use currencies::{Lpn, Lpns, Nls, testing::LeaseC1};
use currency::CurrencyDef;
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::{Fraction, Unit},
    percent::{Percent, Percent100, permilles::Permilles},
    price,
    ratio::SimpleFraction,
    rational::Rational,
    zero::Zero,
};
use lpp::{
    borrow::InterestRate,
    contract::ContractError,
    msg::{
        BalanceResponse, ConfigResponse, LppBalanceResponse, PriceResponse, QueryLoanResponse,
        QueryQuoteResponse, RewardsResponse, SudoMsg,
    },
};
use platform::{bank, coin_legacy};
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
    testing,
};

use crate::{
    common::{
        self, ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, CwCoin, UTILIZATION_OPTIMAL,
        lease::{
            InitConfig as LeaseInitConfig, Instantiator as LeaseInstantiator,
            InstantiatorAddresses as LeaseInstantiatorAddresses,
            InstantiatorConfig as LeaseInstantiatorConfig,
        },
        leaser::Instantiator as LeaserInstantiator,
        lpp::{LppExecuteMsg, LppQueryMsg},
        protocols::Registry,
        test_case::{
            TestCase, app::App, builder::BlankBuilder as TestCaseBuilder,
            response::ResponseWithInterChainMsgs,
        },
    },
    lease::LeaseTestCase,
};

type LeaseCurrency = LeaseC1;
const HALF_YEAR: Duration = Duration::from_nanos(Duration::YEAR.nanos() / 2);

#[test]
fn config_update_parameters() {
    let app_balance = 10_000_000_000;

    let base_interest_rate = Percent100::from_permille(210);
    let addon_optimal_interest_rate = Percent100::from_permille(200);
    let utilization_optimal = Percent100::from_permille(550);
    let min_utilization = Percent100::from_permille(500);

    assert_ne!(base_interest_rate, BASE_INTEREST_RATE);
    assert_ne!(addon_optimal_interest_rate, ADDON_OPTIMAL_INTEREST_RATE);
    assert_ne!(utilization_optimal, UTILIZATION_OPTIMAL);
    assert_ne!(min_utilization, TestCase::DEFAULT_LPP_MIN_UTILIZATION);

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        lpn_cwcoin(app_balance),
        common::cwcoin_from_amount::<Nls>(app_balance),
    ])
    .init_lpp(
        None,
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
        TestCase::DEFAULT_LPP_MIN_UTILIZATION,
    )
    .into_generic();

    let response = new_borrow_rate(
        &mut test_case,
        base_interest_rate,
        utilization_optimal,
        addon_optimal_interest_rate,
    );
    assert!(response.data.is_none());
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_address", contract_address_ref(&test_case)),]
    );

    let response: AppResponse = test_case
        .app
        .sudo(
            contract_address(&test_case),
            &SudoMsg::MinUtilization { min_utilization },
        )
        .unwrap()
        .unwrap_response();

    assert!(response.data.is_none());
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_address", contract_address_ref(&test_case)),]
    );

    let quote: ConfigResponse = test_case
        .app
        .query()
        .query_wasm_smart(contract_address(&test_case), &LppQueryMsg::Config())
        .unwrap();

    assert_eq!(quote.borrow_rate().base_interest_rate(), base_interest_rate);
    assert_eq!(
        quote.borrow_rate().utilization_optimal(),
        utilization_optimal
    );
    assert_eq!(
        quote.borrow_rate().addon_optimal_interest_rate(),
        addon_optimal_interest_rate
    );
    assert_eq!(quote.min_utilization(), min_utilization);
}

#[test]
fn open_loan_unauthorized_contract_id() {
    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .into_generic();

    let lender_addr = contract_address(&test_case);
    let err = try_open_loan(&mut test_case, lender_addr, 100, &[lpn_cwcoin(200)]).unwrap_err();

    assert!(matches!(
        err.downcast_ref::<ContractError>(),
        Some(&ContractError::Platform(
            platform::error::Error::UnexpectedCode(_, _)
        ))
    ))
}

#[test]
#[should_panic(expected = "No liquidity")]
fn open_loan_no_liquidity() {
    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .init_reserve()
        .init_leaser()
        .into_generic();

    let lease_addr: Addr = test_case.open_lease::<Lpn>(currency::dto::<LeaseCurrency, _>());

    try_open_loan(&mut test_case, lease_addr, 2500, &[lpn_cwcoin(200)])
        .unwrap()
        .ignore_response()
        .unwrap_response()
}

#[test]
fn deposit_and_withdraw() {
    let app_balance = 10_000_000_000;
    let init_deposit = 20_000;
    let lpp_balance_push = 80_000;
    let pushed_price = (lpp_balance_push + init_deposit) / init_deposit;
    let test_deposit = 10_004;
    let rounding_error = test_deposit % pushed_price; // should be 4 for this setup
    let post_deposit = 1_000_000;
    let loan = 1_000_000;
    let overdraft = 5_000;
    let withdraw_amount_nlpn = 1000;
    let rest_nlpn = test_deposit / pushed_price - withdraw_amount_nlpn;

    let admin = testing::user(ADMIN);

    let lender1 = testing::user("lender1");
    let lender2 = testing::user("lender2");
    let lender3 = testing::user("lender3");

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[lpn_cwcoin(app_balance)])
        .init_lpp_with_funds(
            None,
            &[],
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .init_reserve()
        .init_leaser()
        .into_generic();

    test_case
        .send_funds_from_admin(lender1.clone(), &[lpn_cwcoin(init_deposit)])
        .send_funds_from_admin(
            lender2.clone(),
            &[lpn_cwcoin(init_deposit.max(test_deposit))],
        )
        .send_funds_from_admin(
            lender3.clone(),
            &[lpn_cwcoin(init_deposit.max(post_deposit))],
        );

    // initial deposit
    deposit(&mut test_case, lender1.clone(), init_deposit);

    // push the price from 1, should be allowed as an interest from previous leases for example.
    () = test_case
        .app
        .send_tokens(
            admin,
            contract_address(&test_case),
            &[lpn_cwcoin(lpp_balance_push)],
        )
        .unwrap();

    let price = query_price(&test_case);

    let amount: Amount = 1_000;
    assert_eq!(
        price::total(Coin::new(amount), price.0).unwrap(),
        common::lpn_coin(amount * pushed_price)
    );

    // deposit to check,
    deposit(&mut test_case, lender2.clone(), test_deposit);

    // got rounding error
    let balance_nlpn = balance(&mut test_case, lender2.clone());

    let price = query_price(&test_case);
    assert_eq!(
        price::total(balance_nlpn.balance, price.0).unwrap(),
        common::lpn_coin(test_deposit - rounding_error)
    );

    // other deposits should not change asserts for lender2
    deposit(&mut test_case, lender3.clone(), post_deposit);

    let balance_nlpn = balance(&mut test_case, lender2.clone());

    let price = query_price(&test_case);
    assert_eq!(
        price::total(balance_nlpn.balance, price.0).unwrap(),
        common::lpn_coin(test_deposit - rounding_error)
    );

    // loans should not change asserts for lender2, the default loan
    instantiate_lease(&mut test_case, loan, Percent100::from_percent(50));

    let balance_nlpn2 = balance(&mut test_case, lender2.clone());

    let price = query_price(&test_case);
    assert_eq!(
        price::total(balance_nlpn2.balance, price.0).unwrap(),
        common::lpn_coin(test_deposit - rounding_error)
    );

    // try to withdraw with overdraft
    let to_burn = Amount::from(balance_nlpn.balance) - rounding_error + overdraft;
    try_burn(&mut test_case, lender2.clone(), to_burn).unwrap_err();

    // partial withdraw
    burn(&mut test_case, lender2.clone(), withdraw_amount_nlpn);

    let balance_nlpn = balance(&mut test_case, lender2.clone());
    assert_eq!(balance_nlpn.balance, Coin::new(rest_nlpn));

    // full withdraw, should close lender's account
    burn(&mut test_case, lender2.clone(), rest_nlpn);

    let balance_nlpn = balance(&mut test_case, lender2);
    assert_eq!(balance_nlpn.balance, Coin::ZERO);
}

#[test]
fn loan_open_wrong_id() {
    let _admin = testing::user(ADMIN);
    let lender = testing::user("lender");
    let hacker = testing::user("Mallory");

    let app_balance = 10_000_000_000;
    let hacker_balance = 10_000_000;
    let init_deposit = 20_000_000;
    let loan = 10_000;

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[lpn_cwcoin(app_balance)])
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .into_generic();

    test_case
        .send_funds_from_admin(lender, &[lpn_cwcoin(init_deposit)])
        .send_funds_from_admin(hacker.clone(), &[lpn_cwcoin(hacker_balance)]);

    try_open_loan(&mut test_case, hacker, loan, &[]).unwrap_err();
}

#[test]
fn loan_open_and_repay() {
    const LOCAL_BASE_INTEREST_RATE: Percent100 = Percent100::from_permille(210);
    const LOCAL_ADDON_OPTIMAL_INTEREST_RATE: Percent100 = Percent100::from_permille(200);
    const LOCAL_UTILIZATION_OPTIMAL_RATE: Percent100 = Percent100::from_permille(550);

    fn interest_rate(loan: Amount, balance: Amount) -> Percent100 {
        general_interest_rate(
            loan,
            balance,
            LOCAL_BASE_INTEREST_RATE,
            LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
            LOCAL_UTILIZATION_OPTIMAL_RATE,
        )
    }

    let admin = testing::user(ADMIN);
    let lender = testing::user("lender");
    let hacker = testing::user("Mallory");

    let app_balance = 10_000_000_000;
    let hacker_balance = 10_000_000;
    let init_deposit = 20_000_000;
    let loan1 = 10_000_000;
    let balance1 = init_deposit - loan1;
    let loan2 = 5_000_000;
    let repay_interest_part = 1_000_000;
    let repay_due_part = 1_000_000;
    let repay_excess = 1_000_000;

    let interest1 = interest_rate(loan1, balance1);
    let loan1_coin = common::lpn_coin(loan1);
    let interest_due1 = interest1.of(loan1_coin);

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        lpn_cwcoin(app_balance),
        common::cwcoin_from_amount::<Nls>(app_balance),
    ])
    .init_lpp_with_funds(
        None,
        &[],
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
        TestCase::DEFAULT_LPP_MIN_UTILIZATION,
    )
    .init_time_alarms()
    .init_protocols_registry(Registry::NoProtocol)
    .init_oracle(None)
    .init_treasury()
    .init_profit(24)
    .init_reserve()
    .init_leaser()
    .into_generic();

    test_case
        .send_funds_from_admin(lender.clone(), &[lpn_cwcoin(init_deposit)])
        .send_funds_from_admin(hacker.clone(), &[lpn_cwcoin(hacker_balance)]);

    // initial deposit
    deposit(&mut test_case, lender, init_deposit);

    new_borrow_rate(
        &mut test_case,
        LOCAL_BASE_INTEREST_RATE,
        LOCAL_UTILIZATION_OPTIMAL_RATE,
        LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
    );

    match quote(&mut test_case, loan1) {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest1),
        _ => panic!("no liquidity"),
    }

    // borrow
    let loan_addr1 = instantiate_lease(&mut test_case, loan1, Percent100::from_percent(50));

    // double borrow
    try_open_loan(&mut test_case, loan_addr1.clone(), loan1, &[]).unwrap_err();

    test_case.app.time_shift(HALF_YEAR);

    let total_interest_due = interest_due1.checked_div(2).unwrap();

    let resp: LppBalanceResponse<Lpns> = test_case
        .app
        .query()
        .query_wasm_smart(contract_address(&test_case), &LppQueryMsg::LppBalance())
        .unwrap();

    assert_eq!(resp.total_interest_due, total_interest_due.into());

    let interest2 = interest_rate(loan1 + loan2 + total_interest_due.to_primitive(), balance1);
    let loan2_coin = common::lpn_coin(loan2);

    match quote(&mut test_case, loan2) {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest2),
        _ => panic!("no liquidity"),
    }

    // borrow 2
    let loan_addr2 = instantiate_lease(&mut test_case, loan2, Percent100::from_percent(50));

    test_case.app.time_shift(HALF_YEAR);

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1_coin);
    assert_eq!(loan1_resp.annual_interest_rate, interest1);
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        interest_due1
    );

    // repay from other addr
    _ = repay_loan::<Lpn>(loan1, &mut test_case, hacker).unwrap_err();

    // repay zero
    _ = repay_loan::<Lpn>(0, &mut test_case, loan_addr1.clone()).unwrap_err();

    // repay wrong currency
    () = test_case
        .app
        .send_tokens(
            admin,
            loan_addr2.clone(),
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(common::coin(
                repay_interest_part,
            ))],
        )
        .unwrap();

    _ = repay_loan::<Nls>(repay_interest_part, &mut test_case, loan_addr2).unwrap_err();

    // repay interest part
    () = repay_loan::<Lpn>(repay_interest_part, &mut test_case, loan_addr1.clone())
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1_coin);
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        interest_due1 - common::lpn_coin(repay_interest_part)
    );

    // repay interest + due part
    () = repay_loan::<Lpn>(
        interest_due1.to_primitive() - repay_interest_part + repay_due_part,
        &mut test_case,
        loan_addr1.clone(),
    )
    .unwrap()
    .ignore_response()
    .unwrap_response();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(
        loan1_resp.principal_due,
        common::lpn_coin(loan1 - repay_due_part)
    );

    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        Some(Coin::ZERO)
    );

    // repay interest + due part, close the loan
    () = repay_loan::<Lpn>(
        loan1 - repay_due_part + repay_excess,
        &mut test_case,
        loan_addr1.clone(),
    )
    .unwrap()
    .ignore_response()
    .unwrap_response();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    assert!(maybe_loan1.is_none());

    // repay excess is returned
    let balance = bank::balance(&loan_addr1, test_case.app.query()).unwrap();
    assert_eq!(balance, loan1_coin - interest_due1);

    let resp: LppBalanceResponse<Lpns> = test_case
        .app
        .query()
        .query_wasm_smart(contract_address(&test_case), &LppQueryMsg::LppBalance())
        .unwrap();

    // total unpaid interest
    assert_eq!(
        resp.total_interest_due,
        interest2.of(loan2_coin).checked_div(2).unwrap().into()
    );
    assert_eq!(resp.total_principal_due, loan2_coin.into());
    assert_eq!(
        resp.balance,
        (common::lpn_coin(init_deposit) + interest_due1 - loan2_coin).into()
    );
}

#[test]
fn compare_lpp_states() {
    const LOCAL_BASE_INTEREST_RATE: Percent100 = Percent100::from_permille(210);
    const LOCAL_ADDON_OPTIMAL_INTEREST_RATE: Percent100 = Percent100::from_permille(200);
    const LOCAL_UTILIZATION_OPTIMAL_RATE: Percent100 = Percent100::from_permille(550);

    fn interest_rate(loan: Amount, balance: Amount) -> Percent100 {
        general_interest_rate(
            loan,
            balance,
            LOCAL_BASE_INTEREST_RATE,
            LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
            LOCAL_UTILIZATION_OPTIMAL_RATE,
        )
    }

    let admin = testing::user(ADMIN);
    let lender = testing::user("lender");
    let hacker = testing::user("Mallory");

    let app_balance = 10_000_000_000;
    let hacker_balance = 10_000_000;
    let init_deposit = 20_000_000;
    let loan1 = 10_000_000;
    let balance1 = init_deposit - loan1;
    let loan2 = 5_000_000;
    let repay_interest_part = 1_000_000;
    let repay_due_part = 1_000_000;
    let repay_excess = 1_000_000;

    let interest1 = interest_rate(loan1, balance1);
    let loan1_coin = common::lpn_coin(loan1);
    let interest_due1 = interest1.of(loan1_coin);

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        lpn_cwcoin(app_balance),
        coin_legacy::to_cosmwasm_on_nolus::<Nls>(common::coin(app_balance)),
    ])
    .init_lpp_with_funds(
        None,
        &[],
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
        TestCase::DEFAULT_LPP_MIN_UTILIZATION,
    )
    .init_time_alarms()
    .init_protocols_registry(Registry::NoProtocol)
    .init_oracle(None)
    .init_treasury()
    .init_profit(24)
    .init_reserve()
    .init_leaser()
    .into_generic();

    test_case
        .send_funds_from_admin(lender.clone(), &[lpn_cwcoin(init_deposit)])
        .send_funds_from_admin(hacker.clone(), &[lpn_cwcoin(hacker_balance)]);

    // initial deposit
    deposit(&mut test_case, lender, init_deposit);

    new_borrow_rate(
        &mut test_case,
        LOCAL_BASE_INTEREST_RATE,
        LOCAL_UTILIZATION_OPTIMAL_RATE,
        LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
    );

    match quote(&mut test_case, loan1) {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest1),
        _ => panic!("no liquidity"),
    }

    // borrow
    let loan_addr1 = instantiate_lease(&mut test_case, loan1, Percent100::from_percent(50));

    // double borrow
    try_open_loan(&mut test_case, loan_addr1.clone(), loan1, &[]).unwrap_err();

    test_case.app.time_shift(HALF_YEAR);

    let total_interest_due = interest_due1.checked_div(2).unwrap();

    let resp: LppBalanceResponse<Lpns> = test_case
        .app
        .query()
        .query_wasm_smart(contract_address(&test_case), &LppQueryMsg::LppBalance())
        .unwrap();
    assert_eq!(resp.total_interest_due, total_interest_due.into());

    let interest2 = interest_rate(loan1 + loan2 + total_interest_due.to_primitive(), balance1);
    let loan2_coin = common::lpn_coin(loan2);

    match quote(&mut test_case, loan2) {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest2),
        _ => panic!("no liquidity"),
    }

    // borrow 2
    let loan_addr2 = instantiate_lease(&mut test_case, loan2, Percent100::from_percent(50));

    test_case.app.time_shift(HALF_YEAR);

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1_coin);
    assert_eq!(loan1_resp.annual_interest_rate, interest1);
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        interest_due1
    );

    // repay from other addr
    _ = repay_loan::<Lpn>(loan1, &mut test_case, hacker).unwrap_err();

    // repay zero
    _ = repay_loan::<Lpn>(0, &mut test_case, loan_addr1.clone()).unwrap_err();

    // repay wrong currency
    () = test_case
        .app
        .send_tokens(
            admin,
            loan_addr2.clone(),
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(common::coin(
                repay_interest_part,
            ))],
        )
        .unwrap();

    _ = repay_loan::<Nls>(repay_interest_part, &mut test_case, loan_addr2).unwrap_err();

    // repay interest part
    () = repay_loan::<Lpn>(repay_interest_part, &mut test_case, loan_addr1.clone())
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1_coin);
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        interest_due1 - common::lpn_coin(repay_interest_part)
    );

    // repay interest + due part
    () = repay_loan::<Lpn>(
        interest_due1.to_primitive() - repay_interest_part + repay_due_part,
        &mut test_case,
        loan_addr1.clone(),
    )
    .unwrap()
    .ignore_response()
    .unwrap_response();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(
        loan1_resp.principal_due,
        common::lpn_coin(loan1 - repay_due_part)
    );
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        Some(Coin::ZERO)
    );

    // repay interest + due part, close the loan
    () = repay_loan::<Lpn>(
        loan1 - repay_due_part + repay_excess,
        &mut test_case,
        loan_addr1.clone(),
    )
    .unwrap()
    .ignore_response()
    .unwrap_response();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    assert!(maybe_loan1.is_none());

    // repay excess is returned
    let balance = bank::balance(&loan_addr1, test_case.app.query()).unwrap();
    assert_eq!(balance, loan1_coin - interest_due1);

    let resp: LppBalanceResponse<Lpns> = test_case
        .app
        .query()
        .query_wasm_smart(contract_address(&test_case), &LppQueryMsg::LppBalance())
        .unwrap();

    let total_unpaid_interest = interest2.of(loan2_coin).checked_div(2).unwrap();
    // total unpaid interest
    assert_eq!(resp.total_interest_due, total_unpaid_interest.into());
    assert_eq!(resp.total_principal_due, loan2_coin.into());
    assert_eq!(
        resp.balance,
        (common::lpn_coin(init_deposit) + interest_due1 - loan2_coin).into()
    );
}

#[test]
fn test_rewards() {
    let app_balance = 10_000_000_000;
    let deposit1 = 20_000;
    let lpp_balance_push = 80_000;
    let pushed_price = (lpp_balance_push + deposit1) / deposit1;
    let deposit2 = 10_004;
    let treasury_balance = 100_000_000;
    let tot_rewards0 = 5_000_000;
    let tot_rewards1 = 10_000_000;
    let tot_rewards2 = 22_000_000;
    let lender_reward1 = tot_rewards2 * deposit1 / (deposit1 + deposit2 / pushed_price);
    // brackets are important here to reflect rounding errors
    let lender_reward2 =
        tot_rewards2 * (deposit2 / pushed_price) / (deposit1 + deposit2 / pushed_price);

    let _admin = testing::user(ADMIN);

    let lender1 = testing::user("lender1");
    let lender2 = testing::user("lender2");
    let recipient = testing::user("recipient");
    // simplified
    // TODO: any checks for the sender of rewards?
    let treasury = testing::user("treasury");

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        lpn_cwcoin(app_balance),
        common::cwcoin_from_amount::<Nls>(app_balance),
    ])
    .init_lpp_with_funds(
        None,
        &[],
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
        TestCase::DEFAULT_LPP_MIN_UTILIZATION,
    )
    .into_generic();

    test_case
        .send_funds_from_admin(lender1.clone(), &[lpn_cwcoin(deposit1)])
        .send_funds_from_admin(lender2.clone(), &[lpn_cwcoin(deposit2)])
        .send_funds_from_admin(
            treasury.clone(),
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(common::coin(
                treasury_balance,
            ))],
        );

    // rewards before deposits
    try_distribute_rewards(&mut test_case, treasury.clone(), tot_rewards0).unwrap_err();

    // initial deposit
    deposit(&mut test_case, lender1.clone(), deposit1);
    // the initial price is 1 Nlpn = 1 LPN
    assert_eq!(
        deposit1,
        balance(&mut test_case, lender1.clone()).balance.into()
    );

    // push the price from 1, should be allowed as an interest from previous leases for example.
    test_case.send_funds_from_admin(
        contract_address(&test_case),
        &[lpn_cwcoin(lpp_balance_push)],
    );

    distribute_rewards(&mut test_case, treasury.clone(), tot_rewards1);

    // deposit after disributing rewards should not get anything
    deposit(&mut test_case, lender2.clone(), deposit2);

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, common::coin(tot_rewards1));

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Rewards {
                address: lender2.clone(),
            },
        )
        .unwrap();

    assert_eq!(Coin::ZERO, resp.rewards);

    // claim zero rewards
    _ = try_claim_rewards(&mut test_case, lender2.clone(), None).unwrap_err();

    // check reward claim with nonvalid recipient
    _ = try_claim_rewards(
        &mut test_case,
        lender1.clone(),
        Some(Addr::unchecked("invalid address")),
    )
    .unwrap_err();

    // check reward claim
    () = try_claim_rewards(&mut test_case, lender1.clone(), None)
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(Coin::ZERO, resp.rewards,);

    let balance = bank::balance(&lender1, test_case.app.query()).unwrap();
    assert_eq!(balance, common::coin::<Nls>(tot_rewards1));

    distribute_rewards(&mut test_case, treasury, tot_rewards2);

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, common::coin(lender_reward1));

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Rewards {
                address: lender2.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, common::coin(lender_reward2));

    // full withdraw, should send rewards to the lender
    burn(&mut test_case, lender1.clone(), deposit1);

    let balance = bank::balance(&lender1, test_case.app.query()).unwrap();
    assert_eq!(balance, common::coin::<Nls>(tot_rewards1 + lender_reward1));

    // lender account is removed
    let resp: Result<RewardsResponse, _> = test_case.app.query().query_wasm_smart(
        contract_address(&test_case),
        &LppQueryMsg::Rewards { address: lender1 },
    );

    assert!(resp.is_err());

    // claim rewards to other recipient
    () = try_claim_rewards(&mut test_case, lender2.clone(), Some(recipient.clone()))
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(&test_case),
            &LppQueryMsg::Rewards { address: lender2 },
        )
        .unwrap();

    assert_eq!(resp.rewards, Coin::ZERO);
    let balance = bank::balance(&recipient, test_case.app.query()).unwrap();
    assert_eq!(balance, common::coin::<Nls>(lender_reward2));
}

#[test]
fn close_all_deposits() {
    let app_balance = 10_000_000_000;
    let deposit1 = 20_000;
    let deposit2 = 10_004;
    let deposit3 = 1_000_000;

    let lender1 = testing::user("lender1");
    let lender2 = testing::user("lender2");
    let lender3 = testing::user("lender3");

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[lpn_cwcoin(app_balance)])
        .init_lpp_with_funds(
            None,
            &[],
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .into_generic();

    test_case
        .send_funds_from_admin(lender1.clone(), &[lpn_cwcoin(deposit1)])
        .send_funds_from_admin(lender2.clone(), &[lpn_cwcoin(deposit2)])
        .send_funds_from_admin(lender3.clone(), &[lpn_cwcoin(deposit3)]);

    deposit(&mut test_case, lender1.clone(), deposit1);
    deposit(&mut test_case, lender2.clone(), deposit2);
    deposit(&mut test_case, lender3.clone(), deposit3);

    expect_balance(&test_case.app, Coin::ZERO, lender1.clone());
    expect_balance(&test_case.app, Coin::ZERO, lender2.clone());
    expect_balance(&test_case.app, Coin::ZERO, lender3.clone());

    let protocol_admin = LeaserInstantiator::expected_addr();
    () = test_case
        .app
        .execute(
            protocol_admin,
            contract_address(&test_case),
            &LppExecuteMsg::CloseAllDeposits(),
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    expect_balance(&test_case.app, common::lpn_coin(deposit1), lender1);
    expect_balance(&test_case.app, common::lpn_coin(deposit2), lender2);
    expect_balance(&test_case.app, common::lpn_coin(deposit3), lender3);
}

fn expect_balance<A>(app: &App, amount: A, lender: Addr)
where
    A: Into<Coin<Lpn>>,
{
    assert_eq!(
        amount.into(),
        bank::balance::<Lpn>(&lender, app.query()).unwrap()
    )
}

fn instantiate_lease<ProtReg, Tr>(
    test_case: &mut TestCase<ProtReg, Tr, Addr, Addr, Addr, Addr, Addr, Addr>,
    downpayment: Amount,
    liability_init_percent: Percent100,
) -> Addr {
    LeaseInstantiator::instantiate::<Lpn>(
        &mut test_case.app,
        test_case.address_book.lease_code(),
        LeaseInstantiatorAddresses {
            lpp: test_case.address_book.lpp().clone(),
            time_alarms: test_case.address_book.time_alarms().clone(),
            oracle: test_case.address_book.oracle().clone(),
            profit: test_case.address_book.profit().clone(),
            reserve: test_case.address_book.reserve().clone(),
            finalizer: test_case.address_book.leaser().clone(),
        },
        LeaseInitConfig::new(
            currency::dto::<LeaseCurrency, _>(),
            common::coin(downpayment),
            None,
        ),
        LeaseInstantiatorConfig {
            liability_init_percent, // simplify case: borrow == downpayment
            ..LeaseInstantiatorConfig::default()
        },
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    )
}

fn general_interest_rate(
    loan: Amount,
    balance: Amount,
    base_rate: Percent100,
    addon_rate: Percent100,
    optimal_rate: Percent100,
) -> Percent100 {
    // TODO migrate to using SimpleFraction once it starts implementing Ord
    Percent::from_fraction(common::lpn_coin(loan), common::lpn_coin(balance))
    .map(|utilization_factor_max| {
            // TODO migrate to using SimpleFraction once it starts implementing Ord
            let utilization_factor = Percent::from_fraction(
                    optimal_rate.permilles(),
                    optimal_rate.complement().permilles(),
                ).expect("The utilization must be a valid Percent").min(utilization_factor_max);

        SimpleFraction::<Permilles>::new(addon_rate.into(), optimal_rate.into()).of(utilization_factor)
        .map(|utilization_config| Percent100::try_from(utilization_config + base_rate.into()).expect("The borrow rate must not exceed 100%"))     
        .expect("The utilization_config must be a valid Percent")     
    })
    .expect("The utilization_max must be a valid Percent: utilization_opt < 100% ensures the ratio is valid Percent100, which always fits within Percent's wider range")
}

fn new_borrow_rate<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    base_interest_rate: Percent100,
    utilization_optimal: Percent100,
    addon_optimal_interest_rate: Percent100,
) -> AppResponse {
    test_case
        .app
        .sudo(
            contract_address(test_case),
            &SudoMsg::NewBorrowRate {
                borrow_rate: InterestRate::new(
                    base_interest_rate,
                    utilization_optimal,
                    addon_optimal_interest_rate,
                )
                .expect("Couldn't construct interest rate value!"),
            },
        )
        .unwrap()
        .unwrap_response()
}

fn try_open_loan<'a, ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &'a mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    lender: Addr,
    amount: Amount,
    send_funds: &[CwCoin],
) -> anyhow::Result<ResponseWithInterChainMsgs<'a, AppResponse>> {
    test_case.app.execute(
        lender,
        contract_address(test_case),
        &LppExecuteMsg::OpenLoan {
            amount: common::lpn_coin_dto(amount),
        },
        send_funds,
    )
}

fn deposit<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Oracle, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Addr,
        Oracle,
        TimeAlarms,
    >,
    lender: Addr,
    amount: Amount,
) {
    test_case
        .app
        .execute(
            lender,
            contract_address(test_case),
            &LppExecuteMsg::Deposit(),
            &[lpn_cwcoin(amount)],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response()
}

fn try_distribute_rewards<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    lender: Addr,
    amount: Amount,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    test_case.app.execute(
        lender,
        contract_address(test_case),
        &LppExecuteMsg::DistributeRewards(),
        &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(common::coin(
            amount,
        ))],
    )
}

fn distribute_rewards<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    lender: Addr,
    amount: Amount,
) {
    try_distribute_rewards(test_case, lender, amount)
        .unwrap()
        .ignore_response()
        .unwrap_response()
}

fn try_burn<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    lender: Addr,
    amount: Amount,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    test_case.app.execute(
        lender,
        contract_address(test_case),
        &LppExecuteMsg::Burn {
            amount: common::coin(amount),
        },
        &[],
    )
}

fn burn<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    lender: Addr,
    amount: Amount,
) {
    try_burn(test_case, lender, amount)
        .unwrap()
        .ignore_response()
        .unwrap_response()
}

fn repay_loan<Currency>(
    repay_amount: Amount,
    test_case: &mut LeaseTestCase,
    loan_addr2: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>>
where
    Currency: CurrencyDef,
{
    test_case.app.execute(
        loan_addr2,
        contract_address(test_case),
        &LppExecuteMsg::RepayLoan(),
        &[coin_legacy::to_cosmwasm_on_nolus::<Currency>(
            common::coin::<Currency>(repay_amount),
        )],
    )
}

fn try_claim_rewards<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    lender: Addr,
    other_recipient: Option<Addr>,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    test_case.app.execute(
        lender,
        contract_address(test_case),
        &LppExecuteMsg::ClaimRewards { other_recipient },
        &[],
    )
}

fn quote<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    amount: Amount,
) -> QueryQuoteResponse {
    test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(test_case),
            &LppQueryMsg::Quote {
                amount: common::lpn_coin_dto(amount),
            },
        )
        .unwrap()
}

fn balance<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &mut TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
    address: Addr,
) -> BalanceResponse {
    test_case
        .app
        .query()
        .query_wasm_smart(
            contract_address(test_case),
            &LppQueryMsg::Balance { address },
        )
        .unwrap()
}

fn query_price<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
) -> PriceResponse<Lpn> {
    test_case
        .app
        .query()
        .query_wasm_smart(contract_address(test_case), &LppQueryMsg::Price())
        .unwrap()
}

fn lpn_cwcoin(amount: Amount) -> CwCoin {
    common::cwcoin(common::lpn_coin(amount))
}

fn contract_address_ref<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
) -> &Addr {
    test_case.address_book.lpp()
}

fn contract_address<ProtoReg, T, P, R, L, O, TAlarms>(
    test_case: &TestCase<ProtoReg, T, P, R, L, Addr, O, TAlarms>,
) -> Addr {
    contract_address_ref(test_case).clone()
}
