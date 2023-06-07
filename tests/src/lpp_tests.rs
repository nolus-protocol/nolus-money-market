use currency::{lease::Atom, lpn::Usdc, native::Nls, Currency};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::Fraction,
    percent::{Percent, Units as PercentUnits},
    price,
    ratio::Rational,
    test,
};
use lpp::{
    borrow::InterestRate,
    msg::{
        BalanceResponse, ExecuteMsg as ExecuteLpp, LppBalanceResponse, PriceResponse,
        QueryLoanResponse, QueryMsg as QueryLpp, QueryQuoteResponse, RewardsResponse, SudoMsg,
    },
    state::Config,
};
use platform::{bank, coin_legacy};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, Event, Timestamp},
    cw_multi_test::{AppResponse, Executor},
};

use crate::common::{
    cwcoin, cwcoins,
    lease_wrapper::{LeaseInitConfig, LeaseWrapper, LeaseWrapperAddresses, LeaseWrapperConfig},
    test_case::TestCase,
    AppExt, MockApp, ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER,
    UTILIZATION_OPTIMAL,
};

type Lpn = Usdc;
type LeaseCurrency = Atom;

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

    let base_interest_rate = Percent::from_percent(21);
    let addon_optimal_interest_rate = Percent::from_percent(20);
    let utilization_optimal = Percent::from_percent(55);

    let mut test_case: TestCase<Lpn> =
        TestCase::with_reserve(&[lpn_cwcoin(app_balance), cwcoin::<Nls, _>(app_balance)]);
    test_case.init_lease().init_lpp(
        None,
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
    );

    let lpp = test_case.lpp().clone();

    let response: AppResponse = test_case
        .app
        .wasm_sudo(
            lpp.clone(),
            &SudoMsg::NewBorrowRate {
                borrow_rate: InterestRate::new(
                    base_interest_rate,
                    utilization_optimal,
                    addon_optimal_interest_rate,
                )
                .expect("Couldn't construct interest rate value!"),
            },
        )
        .unwrap();

    assert!(response.data.is_none());
    assert_eq!(
        &response.events,
        &[Event::new("sudo").add_attribute("_contract_addr", "contract0"),]
    );

    let quote: Config = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp, &QueryLpp::Config())
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
}

#[test]
#[should_panic(expected = "Expecting code id 1 for the contract contract5")]
fn open_loan_unauthorized_contract_id() {
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::<Lpn>::new();
    test_case
        .init(user_addr, &mut [lpn_cwcoin(500)])
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_timealarms()
        .init_oracle(None)
        .init_treasury()
        .init_profit(24);

    let lpp_addr = test_case.lpp().clone();

    //redeploy lease contract to change the code_id
    test_case.init_lease();

    let lease_addr = test_case.open_lease::<Lpn>(LeaseCurrency::TICKER);

    test_case
        .app
        .execute_contract(
            lease_addr,
            lpp_addr,
            &lpp::msg::ExecuteMsg::OpenLoan {
                amount: test::funds::<_, Lpn>(100),
            },
            &[lpn_cwcoin(200)],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "No liquidity")]
fn open_loan_no_liquidity() {
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::<Lpn>::new();
    test_case
        .init(user_addr, &mut [lpn_cwcoin(500)])
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_timealarms()
        .init_oracle(None)
        .init_treasury()
        .init_profit(24);

    let lease_addr = test_case.open_lease::<Lpn>(LeaseCurrency::TICKER);

    let lpp_addr = test_case.lpp().clone();

    test_case
        .app
        .execute_contract(
            lease_addr,
            lpp_addr,
            &lpp::msg::ExecuteMsg::OpenLoan {
                amount: test::funds::<_, Lpn>(100),
            },
            &[lpn_cwcoin(200)],
        )
        .unwrap();
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

    let admin = Addr::unchecked(ADMIN);

    let lender1 = Addr::unchecked("lender1");
    let lender2 = Addr::unchecked("lender2");
    let lender3 = Addr::unchecked("lender3");

    let mut test_case: TestCase<Lpn> = TestCase::with_reserve(&[lpn_cwcoin(app_balance)]);
    test_case
        .init_lease()
        .init_lpp_with_funds(
            None,
            &[],
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_timealarms()
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .send_funds_from_admin(lender1.clone(), &[lpn_cwcoin(init_deposit)])
        .send_funds_from_admin(lender2.clone(), &[lpn_cwcoin(init_deposit)])
        .send_funds_from_admin(lender3.clone(), &[lpn_cwcoin(init_deposit)]);

    let lease_id = test_case.lease_code_id();
    let lpp = test_case.lpp().clone();
    let time_alarms = test_case.time_alarms().clone();
    let market_price_oracle = test_case.oracle().clone();
    let profit = test_case.profit().clone();

    // initial deposit
    test_case
        .app
        .execute_contract(
            lender1.clone(),
            lpp.clone(),
            &ExecuteLpp::Deposit(),
            &[lpn_cwcoin(init_deposit)],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    // push the price from 1, should be allowed as an interest from previous leases for example.
    test_case
        .app
        .send_tokens(admin, lpp.clone(), &[lpn_cwcoin(lpp_balance_push)])
        .unwrap();

    let price: PriceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Price())
        .unwrap();
    assert_eq!(
        price::total(Coin::new(1_000), price.0),
        Coin::<Lpn>::new(1_000 * pushed_price)
    );

    // deposit to check,
    test_case
        .app
        .execute_contract(
            lender2.clone(),
            lpp.clone(),
            &ExecuteLpp::Deposit(),
            &[lpn_cwcoin(test_deposit)],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    // got rounding error
    let balance_nlpn: BalanceResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn.balance.into(), price.0),
        Coin::<Lpn>::new(test_deposit - rounding_error)
    );

    // other deposits should not change asserts for lender2
    test_case
        .app
        .execute_contract(
            lender3.clone(),
            lpp.clone(),
            &ExecuteLpp::Deposit(),
            &[lpn_cwcoin(post_deposit)],
        )
        .unwrap();

    let balance_nlpn: BalanceResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn.balance.into(), price.0),
        Coin::<Lpn>::new(test_deposit - rounding_error)
    );

    // loans should not change asserts for lender2, the default loan
    let balance_lpp: LppBalanceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance())
        .unwrap();
    dbg!(balance_lpp);

    LeaseWrapper::default().instantiate::<Lpn>(
        &mut test_case.app,
        Some(lease_id),
        LeaseWrapperAddresses {
            lpp: lpp.clone(),
            time_alarms,
            oracle: market_price_oracle,
            profit,
        },
        LeaseInitConfig::new(LeaseCurrency::TICKER, loan.into(), None),
        LeaseWrapperConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseWrapperConfig::default()
        },
    );

    test_case
        .message_receiver
        .assert_register_ica(TestCase::<Lpn>::LEASER_CONNECTION_ID);

    test_case.message_receiver.assert_empty();

    let balance_lpp: LppBalanceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance())
        .unwrap();
    dbg!(&balance_lpp);

    let balance_nlpn2: BalanceResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn2.balance.into(), price.0),
        Coin::<Lpn>::new(test_deposit - rounding_error)
    );

    let balance_nlpn1: BalanceResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Balance { address: lender1 })
        .unwrap();

    let balance_nlpn3: BalanceResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Balance { address: lender3 })
        .unwrap();

    // check for balance consistency
    assert_eq!(
        Coin::new((balance_nlpn1.balance + balance_nlpn2.balance + balance_nlpn3.balance).u128()),
        balance_lpp.balance_nlpn
    );

    // try to withdraw with overdraft
    let to_burn: u128 = balance_nlpn.balance.u128() - rounding_error + overdraft;
    let resp = test_case.app.execute_contract(
        lender2.clone(),
        lpp.clone(),
        &ExecuteLpp::Burn {
            amount: to_burn.into(),
        },
        &[],
    );
    assert!(resp.is_err());

    test_case.message_receiver.assert_empty();

    // partial withdraw
    test_case
        .app
        .execute_contract(
            lender2.clone(),
            lpp.clone(),
            &ExecuteLpp::Burn {
                amount: withdraw_amount_nlpn.into(),
            },
            &[],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let balance_nlpn: BalanceResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    assert_eq!(balance_nlpn.balance.u128(), rest_nlpn);

    // full withdraw, should close lender's account
    test_case
        .app
        .execute_contract(
            lender2.clone(),
            lpp.clone(),
            &ExecuteLpp::Burn {
                amount: (rest_nlpn).into(),
            },
            &[],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let balance_nlpn: BalanceResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp, &QueryLpp::Balance { address: lender2 })
        .unwrap();
    assert_eq!(balance_nlpn.balance.u128(), 0);
}

#[test]
fn loan_open_wrong_id() {
    let _admin = Addr::unchecked(ADMIN);
    let lender = Addr::unchecked("lender");
    let hacker = Addr::unchecked("Mallory");

    let app_balance = 10_000_000_000u128;
    let hacker_balance = 10_000_000;
    let init_deposit = 20_000_000u128;
    let loan = 10_000u128;

    let mut test_case: TestCase<Lpn> = TestCase::with_reserve(&lpn_cwcoins(app_balance));
    test_case
        .init_lease()
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .send_funds_from_admin(lender, &lpn_cwcoins(init_deposit))
        .send_funds_from_admin(hacker.clone(), &lpn_cwcoins(hacker_balance));

    let lpp = test_case.lpp().clone();

    let res = test_case.app.execute_contract(
        hacker,
        lpp,
        &ExecuteLpp::OpenLoan {
            amount: Coin::<Lpn>::new(loan).into(),
        },
        &[],
    );
    assert!(res.is_err());
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

    let admin = Addr::unchecked(ADMIN);
    let lender = Addr::unchecked("lender");
    let hacker = Addr::unchecked("Mallory");

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

    let mut test_case: TestCase<Lpn> =
        TestCase::with_reserve(&[lpn_cwcoin(app_balance), cwcoin::<Nls, _>(app_balance)]);
    test_case
        .init_lease()
        .init_lpp_with_funds(
            None,
            &[],
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_timealarms()
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .send_funds_from_admin(lender.clone(), &[lpn_cwcoin(init_deposit)])
        .send_funds_from_admin(hacker.clone(), &[lpn_cwcoin(hacker_balance)]);

    let lpp = test_case.lpp().clone();
    let time_alarms = test_case.time_alarms().clone();
    let oracle = test_case.oracle().clone();
    let profit = test_case.profit().clone();
    let lease_id = test_case.lease_code_id();

    test_case.message_receiver.assert_empty();

    let lease_addresses = LeaseWrapperAddresses {
        lpp: lpp.clone(),
        time_alarms,
        oracle,
        profit,
    };

    // initial deposit
    test_case
        .app
        .execute_contract(
            lender,
            lpp.clone(),
            &ExecuteLpp::Deposit(),
            &[lpn_cwcoin(init_deposit)],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    test_case
        .app
        .wasm_sudo(
            lpp.clone(),
            &SudoMsg::NewBorrowRate {
                borrow_rate: InterestRate::new(
                    LOCAL_BASE_INTEREST_RATE,
                    LOCAL_UTILIZATION_OPTIMAL_RATE,
                    LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
            },
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let quote: QueryQuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Quote {
                amount: Coin::<Lpn>::new(loan1).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest1),
        _ => panic!("no liquidity"),
    }

    // borrow
    let loan_addr1 = LeaseWrapper::default().instantiate::<Lpn>(
        &mut test_case.app,
        Some(lease_id),
        lease_addresses.clone(),
        LeaseInitConfig::new(LeaseCurrency::TICKER, loan1.into(), None),
        LeaseWrapperConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseWrapperConfig::default()
        },
    );

    test_case
        .message_receiver
        .assert_register_ica(TestCase::<Lpn>::LEASER_CONNECTION_ID);

    test_case.message_receiver.assert_empty();

    // double borrow
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::OpenLoan {
                amount: Coin::<Lpn>::new(loan1).into(),
            },
            &[],
        )
        .unwrap_err();

    test_case.message_receiver.assert_empty();

    test_case.app.time_shift(Duration::from_nanos(YEAR / 2));

    let total_interest_due_u32 = interest1.of(loan1_u32) / 2;
    let total_interest_due = Amount::from(total_interest_due_u32);

    let resp: LppBalanceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance())
        .unwrap();
    dbg!(&resp);
    assert_eq!(resp.total_interest_due, Coin::new(total_interest_due));

    let interest2 = interest_rate(loan1_u32 + loan2_u32 + total_interest_due_u32, balance1_u32);

    let quote: QueryQuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Quote {
                amount: Coin::<Lpn>::new(loan2).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest2),
        _ => panic!("no liquidity"),
    }

    // borrow 2
    let loan_addr2 = LeaseWrapper::default().instantiate::<Lpn>(
        &mut test_case.app,
        Some(lease_id),
        lease_addresses,
        LeaseInitConfig::new(LeaseCurrency::TICKER, loan2.into(), None),
        LeaseWrapperConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseWrapperConfig::default()
        },
    );

    test_case
        .message_receiver
        .assert_register_ica(TestCase::<Lpn>::LEASER_CONNECTION_ID);

    test_case.message_receiver.assert_empty();

    test_case.app.time_shift(Duration::from_nanos(YEAR / 2));

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1.into());
    assert_eq!(loan1_resp.annual_interest_rate, interest1);
    assert_eq!(
        loan1_resp.interest_due(block_time(&test_case.app)),
        interest1.of(loan1).into()
    );

    // repay from other addr
    test_case
        .app
        .execute_contract(
            hacker,
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(loan1)],
        )
        .unwrap_err();

    test_case.message_receiver.assert_empty();

    // repay zero
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(0)],
        )
        .unwrap_err();

    test_case.message_receiver.assert_empty();

    // repay wrong currency
    test_case
        .app
        .send_tokens(
            admin,
            loan_addr2.clone(),
            &[coin_legacy::to_cosmwasm::<Nls>(repay_interest_part.into())],
        )
        .unwrap();
    test_case
        .app
        .execute_contract(
            loan_addr2,
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[coin_legacy::to_cosmwasm::<Nls>(repay_interest_part.into())],
        )
        .unwrap_err();

    test_case.message_receiver.assert_empty();

    // repay interest part
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(repay_interest_part)],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1.into());
    assert_eq!(
        loan1_resp.interest_due(block_time(&test_case.app)),
        (interest1.of(loan1) - repay_interest_part).into()
    );

    // repay interest + due part
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(
                interest1.of(loan1) - repay_interest_part + repay_due_part,
            )],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, (loan1 - repay_due_part).into());
    assert_eq!(
        loan1_resp.interest_due(block_time(&test_case.app)),
        Coin::new(0)
    );

    // repay interest + due part, close the loan
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(loan1 - repay_due_part + repay_excess)],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    assert!(maybe_loan1.is_none());

    // repay excess is returned
    let balance = bank::balance(&loan_addr1, &test_case.app.wrap()).unwrap();
    assert_eq!(balance, Coin::<Lpn>::from(loan1 - interest1.of(loan1)));

    let resp: LppBalanceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp, &QueryLpp::LppBalance())
        .unwrap();

    // total unpaid interest
    assert_eq!(
        resp.total_interest_due,
        (interest2.of(loan2) / 2u128).into()
    );
    assert_eq!(resp.total_principal_due, loan2.into());
    assert_eq!(
        resp.balance,
        (init_deposit + interest1.of(loan1) - loan2).into()
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

    let admin = Addr::unchecked(ADMIN);
    let lender = Addr::unchecked("lender");
    let hacker = Addr::unchecked("Mallory");

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

    let mut test_case: TestCase<Lpn> = TestCase::with_reserve(&[
        lpn_cwcoin(app_balance),
        coin_legacy::to_cosmwasm::<Nls>(app_balance.into()),
    ]);
    test_case
        .init_lease()
        .init_lpp_with_funds(
            None,
            &[],
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .init_timealarms()
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .send_funds_from_admin(lender.clone(), &[lpn_cwcoin(init_deposit)])
        .send_funds_from_admin(hacker.clone(), &[lpn_cwcoin(hacker_balance)]);

    test_case.message_receiver.assert_empty();

    let lease_id = test_case.lease_code_id();
    let lpp = test_case.lpp().clone();
    let time_alarms = test_case.time_alarms().clone();
    let market_oracle = test_case.oracle().clone();
    let profit = test_case.profit().clone();

    // initial deposit
    test_case
        .app
        .execute_contract(
            lender,
            lpp.clone(),
            &ExecuteLpp::Deposit(),
            &[lpn_cwcoin(init_deposit)],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    test_case
        .app
        .wasm_sudo(
            lpp.clone(),
            &SudoMsg::NewBorrowRate {
                borrow_rate: InterestRate::new(
                    LOCAL_BASE_INTEREST_RATE,
                    LOCAL_UTILIZATION_OPTIMAL_RATE,
                    LOCAL_ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
            },
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let quote: QueryQuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Quote {
                amount: Coin::<Lpn>::new(loan1).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest1),
        _ => panic!("no liquidity"),
    }

    // borrow
    let loan_addr1 = LeaseWrapper::default().instantiate::<Lpn>(
        &mut test_case.app,
        Some(lease_id),
        LeaseWrapperAddresses {
            lpp: lpp.clone(),
            time_alarms: time_alarms.clone(),
            oracle: market_oracle.clone(),
            profit: profit.clone(),
        },
        LeaseInitConfig::new(LeaseCurrency::TICKER, loan1.into(), None),
        LeaseWrapperConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseWrapperConfig::default()
        },
    );

    test_case
        .message_receiver
        .assert_register_ica(TestCase::<Lpn>::LEASER_CONNECTION_ID);

    test_case.message_receiver.assert_empty();

    // double borrow
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::OpenLoan {
                amount: Coin::<Lpn>::new(loan1).into(),
            },
            &[],
        )
        .unwrap_err();

    test_case.message_receiver.assert_empty();

    test_case.app.time_shift(Duration::from_nanos(YEAR / 2));

    let total_interest_due_u32 = interest1.of(loan1_u32) / 2;
    let total_interest_due = Amount::from(total_interest_due_u32);

    let resp: LppBalanceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance())
        .unwrap();
    dbg!(&resp);
    assert_eq!(resp.total_interest_due, Coin::new(total_interest_due));

    let interest2 = interest_rate(loan1_u32 + loan2_u32 + total_interest_due_u32, balance1_u32);

    let quote: QueryQuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Quote {
                amount: Coin::<Lpn>::new(loan2).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(quote, interest2),
        _ => panic!("no liquidity"),
    }

    // borrow 2
    let loan_addr2 = LeaseWrapper::default().instantiate::<Lpn>(
        &mut test_case.app,
        Some(lease_id),
        LeaseWrapperAddresses {
            lpp: lpp.clone(),
            time_alarms,
            oracle: market_oracle,
            profit,
        },
        LeaseInitConfig::new(LeaseCurrency::TICKER, loan2.into(), None),
        LeaseWrapperConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            ..LeaseWrapperConfig::default()
        },
    );

    test_case
        .message_receiver
        .assert_register_ica(TestCase::<Lpn>::LEASER_CONNECTION_ID);

    test_case.message_receiver.assert_empty();

    test_case.app.time_shift(Duration::from_nanos(YEAR / 2));

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1.into());
    assert_eq!(loan1_resp.annual_interest_rate, interest1);
    assert_eq!(
        loan1_resp.interest_due(block_time(&test_case.app)),
        interest1.of(loan1).into()
    );

    // repay from other addr
    test_case
        .app
        .execute_contract(
            hacker,
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(loan1)],
        )
        .unwrap_err();

    test_case.message_receiver.assert_empty();

    // repay zero
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(0)],
        )
        .unwrap_err();

    test_case.message_receiver.assert_empty();

    // repay wrong currency
    test_case
        .app
        .send_tokens(
            admin,
            loan_addr2.clone(),
            &[coin_legacy::to_cosmwasm::<Nls>(repay_interest_part.into())],
        )
        .unwrap();
    test_case
        .app
        .execute_contract(
            loan_addr2,
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[coin_legacy::to_cosmwasm::<Nls>(repay_interest_part.into())],
        )
        .unwrap_err();

    test_case.message_receiver.assert_empty();

    // repay interest part
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(repay_interest_part)],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, loan1.into());
    assert_eq!(
        loan1_resp.interest_due(block_time(&test_case.app)),
        (interest1.of(loan1) - repay_interest_part).into()
    );

    // repay interest + due part
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(
                interest1.of(loan1) - repay_interest_part + repay_due_part,
            )],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    let loan1_resp = maybe_loan1.unwrap();
    assert_eq!(loan1_resp.principal_due, (loan1 - repay_due_part).into());
    assert_eq!(
        loan1_resp.interest_due(block_time(&test_case.app)),
        Coin::new(0)
    );

    // repay interest + due part, close the loan
    test_case
        .app
        .execute_contract(
            loan_addr1.clone(),
            lpp.clone(),
            &ExecuteLpp::RepayLoan(),
            &[lpn_cwcoin(loan1 - repay_due_part + repay_excess)],
        )
        .unwrap();

    test_case.message_receiver.assert_empty();

    let maybe_loan1: QueryLoanResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Loan {
                lease_addr: loan_addr1.clone(),
            },
        )
        .unwrap();
    assert!(maybe_loan1.is_none());

    // repay excess is returned
    let balance = bank::balance(&loan_addr1, &test_case.app.wrap()).unwrap();
    assert_eq!(balance, Coin::<Lpn>::from(loan1 - interest1.of(loan1)));

    let resp: LppBalanceResponse<Lpn> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp, &QueryLpp::LppBalance())
        .unwrap();

    // total unpaid interest
    assert_eq!(
        resp.total_interest_due,
        (interest2.of(loan2) / 2u128).into()
    );
    assert_eq!(resp.total_principal_due, loan2.into());
    assert_eq!(
        resp.balance,
        (init_deposit + interest1.of(loan1) - loan2).into()
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

    let _admin = Addr::unchecked(ADMIN);

    let lender1 = Addr::unchecked("lender1");
    let lender2 = Addr::unchecked("lender2");
    let recipient = Addr::unchecked("recipient");
    // simplified
    // TODO: any checks for the sender of rewards?
    let treasury = Addr::unchecked("treasury");

    let mut test_case: TestCase<Lpn> =
        TestCase::with_reserve(&[lpn_cwcoin(app_balance), cwcoin::<Nls, _>(app_balance)]);
    test_case
        .init_lease()
        .init_lpp_with_funds(
            None,
            &[],
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
        )
        .send_funds_from_admin(lender1.clone(), &[lpn_cwcoin(deposit1)])
        .send_funds_from_admin(lender2.clone(), &[lpn_cwcoin(deposit2)])
        .send_funds_from_admin(
            treasury.clone(),
            &[coin_legacy::to_cosmwasm::<Nls>(treasury_balance.into())],
        );

    let lpp = test_case.lpp().clone();

    // rewards before deposits
    test_case
        .app
        .execute_contract(
            treasury.clone(),
            lpp.clone(),
            &ExecuteLpp::DistributeRewards(),
            &[coin_legacy::to_cosmwasm::<Nls>(tot_rewards0.into())],
        )
        .unwrap_err();

    // initial deposit
    test_case
        .app
        .execute_contract(
            lender1.clone(),
            lpp.clone(),
            &ExecuteLpp::Deposit(),
            &[lpn_cwcoin(deposit1)],
        )
        .unwrap();

    // push the price from 1, should be allowed as an interest from previous leases for example.
    test_case.send_funds_from_admin(lpp.clone(), &[lpn_cwcoin(lpp_balance_push)]);

    test_case
        .app
        .execute_contract(
            treasury.clone(),
            lpp.clone(),
            &ExecuteLpp::DistributeRewards(),
            &[coin_legacy::to_cosmwasm::<Nls>(tot_rewards1.into())],
        )
        .unwrap();

    // deposit after disributing rewards should not get anything
    test_case
        .app
        .execute_contract(
            lender2.clone(),
            lpp.clone(),
            &ExecuteLpp::Deposit(),
            &[lpn_cwcoin(deposit2)],
        )
        .unwrap();

    let resp: RewardsResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, tot_rewards1.into());

    let resp: RewardsResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Rewards {
                address: lender2.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, Coin::new(0));

    // claim zero rewards
    test_case
        .app
        .execute_contract(
            lender2.clone(),
            lpp.clone(),
            &ExecuteLpp::ClaimRewards {
                other_recipient: None,
            },
            &[],
        )
        .unwrap_err();

    // check reward claim with nonvalid recipient
    test_case
        .app
        .execute_contract(
            lender1.clone(),
            lpp.clone(),
            &ExecuteLpp::ClaimRewards {
                other_recipient: Some(Addr::unchecked("-")),
            },
            &[],
        )
        .unwrap_err();

    // check reward claim
    test_case
        .app
        .execute_contract(
            lender1.clone(),
            lpp.clone(),
            &ExecuteLpp::ClaimRewards {
                other_recipient: None,
            },
            &[],
        )
        .unwrap();

    let resp: RewardsResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, Coin::new(0));

    let balance = bank::balance(&lender1, &test_case.app.wrap()).unwrap();
    assert_eq!(balance, Coin::<Nls>::from(tot_rewards1));

    test_case
        .app
        .execute_contract(
            treasury,
            lpp.clone(),
            &ExecuteLpp::DistributeRewards(),
            &[coin_legacy::to_cosmwasm::<Nls>(tot_rewards2.into())],
        )
        .unwrap();

    let resp: RewardsResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, lender_reward1.into());

    let resp: RewardsResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Rewards {
                address: lender2.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, lender_reward2.into());

    // full withdraw, should send rewards to the lender
    test_case
        .app
        .execute_contract(
            lender1.clone(),
            lpp.clone(),
            &ExecuteLpp::Burn {
                amount: deposit1.into(),
            },
            &[],
        )
        .unwrap();

    let balance = bank::balance(&lender1, &test_case.app.wrap()).unwrap();
    assert_eq!(balance, Coin::<Nls>::from(tot_rewards1 + lender_reward1));

    // lender account is removed
    let resp: Result<RewardsResponse, _> = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Rewards { address: lender1 });

    assert!(resp.is_err());

    // claim rewards to other recipient
    test_case
        .app
        .execute_contract(
            lender2.clone(),
            lpp.clone(),
            &ExecuteLpp::ClaimRewards {
                other_recipient: Some(recipient.clone()),
            },
            &[],
        )
        .unwrap();

    let resp: RewardsResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(lpp, &QueryLpp::Rewards { address: lender2 })
        .unwrap();

    assert_eq!(resp.rewards, Coin::new(0));
    let balance = bank::balance(&recipient, &test_case.app.wrap()).unwrap();
    assert_eq!(balance, Coin::<Nls>::from(lender_reward2));
}

fn lpn_cwcoin<A>(amount: A) -> CwCoin
where
    A: Into<Coin<Lpn>>,
{
    cwcoin(amount)
}

fn lpn_cwcoins<A>(amount: A) -> Vec<CwCoin>
where
    A: Into<Coin<Lpn>>,
{
    cwcoins(amount)
}

fn block_time(app: &MockApp) -> Timestamp {
    app.block_info().time
}
