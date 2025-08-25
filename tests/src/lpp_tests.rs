use currencies::{Lpn, Lpns, Nls, testing::LeaseC1};
use currency::CurrencyDef;
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::Fraction,
    percent::{Percent, Units as PercentUnits},
    price,
    ratio::Rational,
    test,
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
        ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, CwCoin, UTILIZATION_OPTIMAL,
        cwcoin,
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

fn general_interest_rate(
    loan: u32,
    balance: u32,
    base_rate: Percent,
    addon_rate: Percent,
    optimal_rate: Percent,
) -> Percent {
    let utilization_rate = Percent::from_ratio(loan, balance).min(Percent::from_ratio(
        optimal_rate.units(),
        (Percent::HUNDRED - optimal_rate).units(),
    ));

    base_rate
        + Fraction::<PercentUnits>::of(
            &Rational::new(addon_rate.units(), optimal_rate.units()),
            utilization_rate,
        )
}

#[test]
fn config_update_parameters() {
    let app_balance = 10_000_000_000u128;

    let base_interest_rate = Percent::from_permille(210);
    let addon_optimal_interest_rate = Percent::from_permille(200);
    let utilization_optimal = Percent::from_permille(550);
    let min_utilization = Percent::from_permille(500).try_into().unwrap();

    assert_ne!(base_interest_rate, BASE_INTEREST_RATE);
    assert_ne!(addon_optimal_interest_rate, ADDON_OPTIMAL_INTEREST_RATE);
    assert_ne!(utilization_optimal, UTILIZATION_OPTIMAL);
    assert_ne!(min_utilization, TestCase::DEFAULT_LPP_MIN_UTILIZATION);

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        lpn_cwcoin(app_balance),
        cwcoin::<Nls, _>(app_balance),
    ])
    .init_lpp(
        None,
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
        TestCase::DEFAULT_LPP_MIN_UTILIZATION,
    )
    .into_generic();

    let response: AppResponse = test_case
        .app
        .sudo(
            test_case.address_book.lpp().clone(),
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
        .unwrap_response();

    assert!(response.data.is_none());
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_address", test_case.address_book.lpp()),]
    );

    let response: AppResponse = test_case
        .app
        .sudo(
            test_case.address_book.lpp().clone(),
            &SudoMsg::MinUtilization { min_utilization },
        )
        .unwrap()
        .unwrap_response();

    assert!(response.data.is_none());
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_address", test_case.address_book.lpp()),]
    );

    let quote: ConfigResponse = test_case
        .app
        .query()
        .query_wasm_smart(test_case.address_book.lpp().clone(), &LppQueryMsg::Config())
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

    let err = test_case
        .app
        .execute(
            test_case.address_book.lpp().clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::OpenLoan {
                amount: test::funds::<_, Lpn>(100),
            },
            &[lpn_cwcoin(200)],
        )
        .unwrap_err();
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

    () = test_case
        .app
        .execute(
            lease_addr,
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::OpenLoan {
                amount: test::funds::<_, Lpn>(2500),
            },
            &[lpn_cwcoin(200)],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();
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
    let withdraw_amount_nlpn = 1000u128;
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
            test_case.address_book.lpp().clone(),
            &[lpn_cwcoin(lpp_balance_push)],
        )
        .unwrap();

    let price: PriceResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(test_case.address_book.lpp().clone(), &LppQueryMsg::Price())
        .unwrap();

    let amount: Amount = 1_000;
    assert_eq!(
        price::total(Coin::new(amount), price.0),
        Coin::<Lpn>::new(1_000 * pushed_price)
    );

    // deposit to check,
    deposit(&mut test_case, lender2.clone(), test_deposit);

    // got rounding error
    let balance_nlpn: BalanceResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(test_case.address_book.lpp().clone(), &LppQueryMsg::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn.balance, price.0),
        Coin::<Lpn>::new(test_deposit - rounding_error)
    );

    // other deposits should not change asserts for lender2
    deposit(&mut test_case, lender3.clone(), post_deposit);

    let balance_nlpn: BalanceResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(test_case.address_book.lpp().clone(), &LppQueryMsg::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn.balance, price.0),
        Coin::<Lpn>::new(test_deposit - rounding_error)
    );

    // loans should not change asserts for lender2, the default loan
    let _: Addr = LeaseInstantiator::instantiate::<Lpn>(
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
        LeaseInitConfig::new(currency::dto::<LeaseCurrency, _>(), loan.into(), None),
        LeaseInstantiatorConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseInstantiatorConfig::default()
        },
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    let balance_nlpn2: BalanceResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(test_case.address_book.lpp().clone(), &LppQueryMsg::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn2.balance, price.0),
        Coin::<Lpn>::new(test_deposit - rounding_error)
    );

    // try to withdraw with overdraft
    let to_burn = Amount::from(balance_nlpn.balance) - rounding_error + overdraft;
    _ = test_case
        .app
        .execute(
            lender2.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::Burn {
                amount: to_burn.into(),
            },
            &[],
        )
        .unwrap_err();

    // partial withdraw
    () = test_case
        .app
        .execute(
            lender2.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::Burn {
                amount: withdraw_amount_nlpn.into(),
            },
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let balance_nlpn: BalanceResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    assert_eq!(balance_nlpn.balance, Coin::new(rest_nlpn));

    // full withdraw, should close lender's account
    () = test_case
        .app
        .execute(
            lender2.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::Burn {
                amount: rest_nlpn.into(),
            },
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let balance_nlpn: BalanceResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp(),
            &LppQueryMsg::Balance { address: lender2 },
        )
        .unwrap();
    assert_eq!(balance_nlpn.balance, Coin::new(Amount::ZERO));
}

#[test]
fn loan_open_wrong_id() {
    let _admin = testing::user(ADMIN);
    let lender = testing::user("lender");
    let hacker = testing::user("Mallory");

    let app_balance = 10_000_000_000u128;
    let hacker_balance = 10_000_000;
    let init_deposit = 20_000_000u128;
    let loan = 10_000u128;

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

    _ = test_case
        .app
        .execute(
            hacker,
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::OpenLoan {
                amount: Coin::<Lpn>::new(loan).into(),
            },
            &[],
        )
        .unwrap_err();
}

#[test]
fn loan_open_and_repay() {
    const LOCAL_BASE_INTEREST_RATE: Percent = Percent::from_permille(210);
    const LOCAL_ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(200);
    const LOCAL_UTILIZATION_OPTIMAL_RATE: Percent = Percent::from_permille(550);

    fn interest_rate(loan: u32, balance: u32) -> Percent {
        general_interest_rate(
            loan,
            balance,
            LOCAL_BASE_INTEREST_RATE,
            LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
            LOCAL_UTILIZATION_OPTIMAL_RATE,
        )
    }

    const YEAR: u64 = Duration::YEAR.nanos();

    let admin = testing::user(ADMIN);
    let lender = testing::user("lender");
    let hacker = testing::user("Mallory");

    let app_balance = 10_000_000_000u128;
    let hacker_balance = 10_000_000;
    let init_deposit_u32 = 20_000_000u32;
    let init_deposit = Amount::from(init_deposit_u32);
    let loan1_u32 = 10_000_000u32;
    let loan1 = Amount::from(loan1_u32);
    let balance1_u32 = init_deposit_u32 - loan1_u32;
    let loan2_u32 = 5_000_000u32;
    let loan2 = Amount::from(loan2_u32);
    let repay_interest_part = 1_000_000u128;
    let repay_due_part = 1_000_000u128;
    let repay_excess = 1_000_000u128;

    let interest1 = interest_rate(loan1_u32, balance1_u32);

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        lpn_cwcoin(app_balance),
        cwcoin::<Nls, _>(app_balance),
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

    let lease_addresses = LeaseInstantiatorAddresses {
        lpp: test_case.address_book.lpp().clone(),
        time_alarms: test_case.address_book.time_alarms().clone(),
        oracle: test_case.address_book.oracle().clone(),
        profit: test_case.address_book.profit().clone(),
        reserve: test_case.address_book.reserve().clone(),
        finalizer: test_case.address_book.leaser().clone(),
    };

    // initial deposit
    deposit(&mut test_case, lender, init_deposit);

    () = test_case
        .app
        .sudo(
            test_case.address_book.lpp().clone(),
            &SudoMsg::NewBorrowRate {
                borrow_rate: InterestRate::new(
                    LOCAL_BASE_INTEREST_RATE,
                    LOCAL_UTILIZATION_OPTIMAL_RATE,
                    LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
            },
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let quote: QueryQuoteResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Quote {
                amount: Coin::<Lpn>::new(loan1).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest1),
        _ => panic!("no liquidity"),
    }

    // borrow
    let loan_addr1 = LeaseInstantiator::instantiate::<Lpn>(
        &mut test_case.app,
        test_case.address_book.lease_code(),
        lease_addresses.clone(),
        LeaseInitConfig::new(currency::dto::<LeaseCurrency, _>(), loan1.into(), None),
        LeaseInstantiatorConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseInstantiatorConfig::default()
        },
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    // double borrow
    _ = test_case
        .app
        .execute(
            loan_addr1.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::OpenLoan {
                amount: Coin::<Lpn>::new(loan1).into(),
            },
            &[],
        )
        .unwrap_err();

    test_case.app.time_shift(Duration::from_nanos(YEAR / 2));

    let total_interest_due_u32 = interest1.of(loan1_u32) / 2;
    let total_interest_due = Amount::from(total_interest_due_u32);

    let resp: LppBalanceResponse<Lpns> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::LppBalance(),
        )
        .unwrap();

    assert_eq!(
        resp.total_interest_due,
        Coin::<Lpn>::new(total_interest_due).into()
    );

    let interest2 = interest_rate(loan1_u32 + loan2_u32 + total_interest_due_u32, balance1_u32);

    let quote: QueryQuoteResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Quote {
                amount: Coin::<Lpn>::new(loan2).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest2),
        _ => panic!("no liquidity"),
    }

    // borrow 2
    let loan_addr2 = LeaseInstantiator::instantiate::<Lpn>(
        &mut test_case.app,
        test_case.address_book.lease_code(),
        lease_addresses,
        LeaseInitConfig::new(currency::dto::<LeaseCurrency, _>(), loan2.into(), None),
        LeaseInstantiatorConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseInstantiatorConfig::default()
        },
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    test_case.app.time_shift(Duration::from_nanos(YEAR / 2));

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1.into());
    assert_eq!(loan1_resp.annual_interest_rate, interest1);
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        interest1.of(loan1).into()
    );

    // repay from other addr
    _ = repay_loan::<Lpn, _>(loan1, &mut test_case, hacker).unwrap_err();

    // repay zero
    _ = repay_loan::<Lpn, _>(0, &mut test_case, loan_addr1.clone()).unwrap_err();

    // repay wrong currency
    () = test_case
        .app
        .send_tokens(
            admin,
            loan_addr2.clone(),
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(
                repay_interest_part.into(),
            )],
        )
        .unwrap();

    _ = repay_loan::<Nls, _>(repay_interest_part, &mut test_case, loan_addr2).unwrap_err();

    // repay interest part
    () = repay_loan::<Lpn, _>(repay_interest_part, &mut test_case, loan_addr1.clone())
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1.into());
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        (interest1.of(loan1) - repay_interest_part).into()
    );

    // repay interest + due part
    () = repay_loan::<Lpn, _>(
        interest1.of(loan1) - repay_interest_part + repay_due_part,
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
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, (loan1 - repay_due_part).into());
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        Coin::new(Amount::ZERO)
    );

    // repay interest + due part, close the loan
    () = repay_loan::<Lpn, _>(
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
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    assert!(maybe_loan1.is_none());

    // repay excess is returned
    let balance = bank::balance(&loan_addr1, test_case.app.query()).unwrap();
    assert_eq!(balance, Coin::<Lpn>::from(loan1 - interest1.of(loan1)));

    let resp: LppBalanceResponse<Lpns> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::LppBalance(),
        )
        .unwrap();

    // total unpaid interest
    assert_eq!(
        resp.total_interest_due,
        Coin::<Lpn>::from(interest2.of(loan2) / 2u128).into()
    );
    assert_eq!(resp.total_principal_due, Coin::<Lpn>::from(loan2).into());
    assert_eq!(
        resp.balance,
        Coin::<Lpn>::from(init_deposit + interest1.of(loan1) - loan2).into()
    );
}

#[test]
fn compare_lpp_states() {
    const LOCAL_BASE_INTEREST_RATE: Percent = Percent::from_permille(210);
    const LOCAL_ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(200);
    const LOCAL_UTILIZATION_OPTIMAL_RATE: Percent = Percent::from_permille(550);

    fn interest_rate(loan: u32, balance: u32) -> Percent {
        general_interest_rate(
            loan,
            balance,
            LOCAL_BASE_INTEREST_RATE,
            LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
            LOCAL_UTILIZATION_OPTIMAL_RATE,
        )
    }

    const YEAR: u64 = Duration::YEAR.nanos();

    let admin = testing::user(ADMIN);
    let lender = testing::user("lender");
    let hacker = testing::user("Mallory");

    let app_balance = 10_000_000_000u128;
    let hacker_balance = 10_000_000;
    let init_deposit_u32 = 20_000_000u32;
    let init_deposit = Amount::from(init_deposit_u32);
    let loan1_u32 = 10_000_000u32;
    let loan1 = Amount::from(loan1_u32);
    let balance1_u32 = init_deposit_u32 - loan1_u32;
    let loan2_u32 = 5_000_000u32;
    let loan2 = Amount::from(loan2_u32);
    let repay_interest_part = 1_000_000u128;
    let repay_due_part = 1_000_000u128;
    let repay_excess = 1_000_000u128;

    let interest1 = interest_rate(loan1_u32, balance1_u32);

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        lpn_cwcoin(app_balance),
        coin_legacy::to_cosmwasm_on_nolus::<Nls>(app_balance.into()),
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

    () = test_case
        .app
        .sudo(
            test_case.address_book.lpp().clone(),
            &SudoMsg::NewBorrowRate {
                borrow_rate: InterestRate::new(
                    LOCAL_BASE_INTEREST_RATE,
                    LOCAL_UTILIZATION_OPTIMAL_RATE,
                    LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
            },
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let quote: QueryQuoteResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Quote {
                amount: Coin::<Lpn>::new(loan1).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest1),
        _ => panic!("no liquidity"),
    }

    // borrow
    let loan_addr1 = LeaseInstantiator::instantiate::<Lpn>(
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
        LeaseInitConfig::new(currency::dto::<LeaseCurrency, _>(), loan1.into(), None),
        LeaseInstantiatorConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseInstantiatorConfig::default()
        },
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    // double borrow
    _ = test_case
        .app
        .execute(
            loan_addr1.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::OpenLoan {
                amount: Coin::<Lpn>::new(loan1).into(),
            },
            &[],
        )
        .unwrap_err();

    test_case.app.time_shift(Duration::from_nanos(YEAR / 2));

    let total_interest_due_u32 = interest1.of(loan1_u32) / 2;
    let total_interest_due = Amount::from(total_interest_due_u32);

    let resp: LppBalanceResponse<Lpns> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::LppBalance(),
        )
        .unwrap();
    assert_eq!(
        resp.total_interest_due,
        Coin::<Lpn>::new(total_interest_due).into()
    );

    let interest2 = interest_rate(loan1_u32 + loan2_u32 + total_interest_due_u32, balance1_u32);

    let quote: QueryQuoteResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Quote {
                amount: Coin::<Lpn>::new(loan2).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest2),
        _ => panic!("no liquidity"),
    }

    // borrow 2
    let loan_addr2 = LeaseInstantiator::instantiate::<Lpn>(
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
        LeaseInitConfig::new(currency::dto::<LeaseCurrency, _>(), loan2.into(), None),
        LeaseInstantiatorConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseInstantiatorConfig::default()
        },
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    test_case.app.time_shift(Duration::from_nanos(YEAR / 2));

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1.into());
    assert_eq!(loan1_resp.annual_interest_rate, interest1);
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        interest1.of(loan1).into()
    );

    // repay from other addr
    _ = repay_loan::<Lpn, _>(loan1, &mut test_case, hacker).unwrap_err();

    // repay zero
    _ = repay_loan::<Lpn, _>(0, &mut test_case, loan_addr1.clone()).unwrap_err();

    // repay wrong currency
    () = test_case
        .app
        .send_tokens(
            admin,
            loan_addr2.clone(),
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(
                repay_interest_part.into(),
            )],
        )
        .unwrap();

    _ = repay_loan::<Nls, _>(repay_interest_part, &mut test_case, loan_addr2).unwrap_err();

    // repay interest part
    () = repay_loan::<Lpn, _>(repay_interest_part, &mut test_case, loan_addr1.clone())
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1.into());
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        (interest1.of(loan1) - repay_interest_part).into()
    );

    // repay interest + due part
    () = repay_loan::<Lpn, _>(
        interest1.of(loan1) - repay_interest_part + repay_due_part,
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
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, (loan1 - repay_due_part).into());
    assert_eq!(
        loan1_resp.interest_due(&crate::block_time(&test_case)),
        Coin::new(Amount::ZERO)
    );

    // repay interest + due part, close the loan
    () = repay_loan::<Lpn, _>(
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
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    assert!(maybe_loan1.is_none());

    // repay excess is returned
    let balance = bank::balance(&loan_addr1, test_case.app.query()).unwrap();
    assert_eq!(balance, Coin::<Lpn>::from(loan1 - interest1.of(loan1)));

    let resp: LppBalanceResponse<Lpns> = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::LppBalance(),
        )
        .unwrap();

    // total unpaid interest
    assert_eq!(
        resp.total_interest_due,
        Coin::<Lpn>::from(interest2.of(loan2) / 2u128).into()
    );
    assert_eq!(resp.total_principal_due, Coin::<Lpn>::from(loan2).into());
    assert_eq!(
        resp.balance,
        Coin::<Lpn>::from(init_deposit + interest1.of(loan1) - loan2).into()
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
        cwcoin::<Nls, _>(app_balance),
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
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(
                treasury_balance.into(),
            )],
        );

    // rewards before deposits
    _ = test_case
        .app
        .execute(
            treasury.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::DistributeRewards(),
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(
                tot_rewards0.into(),
            )],
        )
        .unwrap_err();

    // initial deposit
    deposit(&mut test_case, lender1.clone(), deposit1);
    // the initial price is 1 Nlpn = 1 LPN
    assert_eq!(
        deposit1,
        test_case
            .app
            .query()
            .query_wasm_smart::<BalanceResponse>(
                test_case.address_book.lpp().clone(),
                &LppQueryMsg::Balance {
                    address: lender1.clone(),
                },
            )
            .unwrap()
            .balance
            .into()
    );

    // push the price from 1, should be allowed as an interest from previous leases for example.
    test_case.send_funds_from_admin(
        test_case.address_book.lpp().clone(),
        &[lpn_cwcoin(lpp_balance_push)],
    );

    () = test_case
        .app
        .execute(
            treasury.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::DistributeRewards(),
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(
                tot_rewards1.into(),
            )],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    // deposit after disributing rewards should not get anything
    deposit(&mut test_case, lender2.clone(), deposit2);

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, tot_rewards1.into());

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Rewards {
                address: lender2.clone(),
            },
        )
        .unwrap();

    assert_eq!(Coin::ZERO, resp.rewards);

    // claim zero rewards
    _ = test_case
        .app
        .execute(
            lender2.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::ClaimRewards {
                other_recipient: None,
            },
            &[],
        )
        .unwrap_err();

    // check reward claim with nonvalid recipient
    _ = test_case
        .app
        .execute(
            lender1.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::ClaimRewards {
                other_recipient: Some(Addr::unchecked("invalid address")),
            },
            &[],
        )
        .unwrap_err();

    // check reward claim
    () = test_case
        .app
        .execute(
            lender1.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::ClaimRewards {
                other_recipient: None,
            },
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(Coin::ZERO, resp.rewards,);

    let balance = bank::balance(&lender1, test_case.app.query()).unwrap();
    assert_eq!(balance, Coin::<Nls>::from(tot_rewards1));

    () = test_case
        .app
        .execute(
            treasury,
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::DistributeRewards(),
            &[coin_legacy::to_cosmwasm_on_nolus::<Nls>(
                tot_rewards2.into(),
            )],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, lender_reward1.into());

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Rewards {
                address: lender2.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, lender_reward2.into());

    // full withdraw, should send rewards to the lender
    () = test_case
        .app
        .execute(
            lender1.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::Burn {
                amount: deposit1.into(),
            },
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let balance = bank::balance(&lender1, test_case.app.query()).unwrap();
    assert_eq!(balance, Coin::<Nls>::from(tot_rewards1 + lender_reward1));

    // lender account is removed
    let resp: Result<RewardsResponse, _> = test_case.app.query().query_wasm_smart(
        test_case.address_book.lpp().clone(),
        &LppQueryMsg::Rewards { address: lender1 },
    );

    assert!(resp.is_err());

    // claim rewards to other recipient
    () = test_case
        .app
        .execute(
            lender2.clone(),
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::ClaimRewards {
                other_recipient: Some(recipient.clone()),
            },
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let resp: RewardsResponse = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.lpp().clone(),
            &LppQueryMsg::Rewards { address: lender2 },
        )
        .unwrap();

    assert_eq!(resp.rewards, Coin::new(Amount::ZERO));
    let balance = bank::balance(&recipient, test_case.app.query()).unwrap();
    assert_eq!(balance, Coin::<Nls>::from(lender_reward2));
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
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::CloseAllDeposits(),
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    expect_balance(&test_case.app, deposit1, lender1);
    expect_balance(&test_case.app, deposit2, lender2);
    expect_balance(&test_case.app, deposit3, lender3);
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

fn deposit<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Oracle, TimeAlarms, Amount>(
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
) where
    Amount: Into<Coin<Lpn>>,
{
    test_case
        .app
        .execute(
            lender,
            test_case.address_book.lpp().clone(),
            &LppExecuteMsg::Deposit(),
            &[lpn_cwcoin(amount)],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response()
}

fn repay_loan<Currency, Amount>(
    repay_amount: Amount,
    test_case: &mut LeaseTestCase,
    loan_addr2: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>>
where
    Currency: CurrencyDef,
    Amount: Into<Coin<Currency>>,
{
    test_case.app.execute(
        loan_addr2,
        test_case.address_book.lpp().clone(),
        &LppExecuteMsg::RepayLoan(),
        &[coin_legacy::to_cosmwasm_on_nolus::<Currency>(
            repay_amount.into(),
        )],
    )
}

fn lpn_cwcoin<A>(amount: A) -> CwCoin
where
    A: Into<Coin<Lpn>>,
{
    cwcoin(amount)
}
