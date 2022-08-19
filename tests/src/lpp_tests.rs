use cosmwasm_std::{coin, coins, Addr};
use cw_multi_test::Executor;
use finance::coin::{self, Coin};
use finance::{
    currency::{Currency, Nls, Usdc},
    duration::Duration,
    fraction::Fraction,
    percent::Percent,
    price,
};

use crate::common::{
    lease_wrapper::{LeaseWrapper, LeaseWrapperConfig},
    lpp_wrapper::LppWrapper,
    mock_app,
    test_case::TestCase,
    AppExt, ADMIN, USER,
};
use lpp::msg::{
    BalanceResponse, ExecuteMsg as ExecuteLpp, LppBalanceResponse, PriceResponse,
    QueryLoanResponse, QueryMsg as QueryLpp, QueryQuoteResponse, RewardsResponse,
};

type TheCurrency = Usdc;

#[test]
#[should_panic(expected = "Unauthorized contract Id")]
fn open_loan_unauthorized_contract_id() {
    let user_balance = 500;
    let lpp_balance = 5000;

    let denom = TheCurrency::SYMBOL;
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(user_balance, denom));

    let lease_id = LeaseWrapper::default().store(&mut test_case.app);

    let (lpp, _) = LppWrapper::default().instantiate(
        &mut test_case.app,
        lease_id.into(),
        denom,
        lpp_balance,
    );

    test_case.lpp_addr = Some(lpp.clone());

    //redeploy lease contract to change the code_id
    test_case.init_lease();

    let lease_addr = test_case.get_lease_instance();

    test_case
        .app
        .execute_contract(
            lease_addr,
            test_case.lpp_addr.unwrap(),
            &lpp::msg::ExecuteMsg::OpenLoan {
                amount: coin::funds::<TheCurrency>(100),
            },
            &coins(200, denom),
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "No liquidity")]
fn open_loan_no_liquidity() {
    let balance = 1000;

    let denom = TheCurrency::SYMBOL;
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(balance, denom));

    let lease_id = LeaseWrapper::default().store(&mut test_case.app);

    let (lpp, _) = LppWrapper::default().instantiate(
        &mut test_case.app,
        lease_id.into(),
        denom,
        balance,
    );

    test_case.lpp_addr = Some(lpp.clone());

    let lease_addr = test_case.get_lease_instance();

    test_case
        .app
        .execute_contract(
            lease_addr,
            test_case.lpp_addr.unwrap(),
            &lpp::msg::ExecuteMsg::OpenLoan {
                amount: coin::funds::<TheCurrency>(100),
            },
            &coins(200, denom),
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

    let denom = TheCurrency::SYMBOL;
    let admin = Addr::unchecked(ADMIN);

    let lender1 = Addr::unchecked("lender1");
    let lender2 = Addr::unchecked("lender2");
    let lender3 = Addr::unchecked("lender3");

    let mut app = mock_app(&[coin(app_balance, denom)]);
    let lease_id = LeaseWrapper::default().store(&mut app);
    let (lpp, _) = LppWrapper::default().instantiate(&mut app, lease_id.into(), denom, 0);

    app.send_tokens(admin.clone(), lender1.clone(), &[coin(init_deposit, denom)])
        .unwrap();
    app.send_tokens(admin.clone(), lender2.clone(), &[coin(test_deposit, denom)])
        .unwrap();
    app.send_tokens(admin.clone(), lender3.clone(), &[coin(post_deposit, denom)])
        .unwrap();

    // initial deposit
    app.execute_contract(
        lender1.clone(),
        lpp.clone(),
        &ExecuteLpp::Deposit(),
        &coins(init_deposit, denom),
    )
    .unwrap();

    // push the price from 1, should be allowed as an interest from previous leases for example.
    app.send_tokens(admin, lpp.clone(), &[coin(lpp_balance_push, denom)])
        .unwrap();

    let price: PriceResponse<TheCurrency> = app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Price())
        .unwrap();
    assert_eq!(
        price::total(Coin::new(1_000), price.0),
        Coin::<TheCurrency>::new(1_000 * pushed_price)
    );

    // deposit to check,
    app.execute_contract(
        lender2.clone(),
        lpp.clone(),
        &ExecuteLpp::Deposit(),
        &coins(test_deposit, denom),
    )
    .unwrap();

    // got rounding error
    let balance_nlpn: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<TheCurrency> = app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn.balance.into(), price.0),
        Coin::<TheCurrency>::new(test_deposit - rounding_error)
    );

    // other deposits should not change asserts for lender2
    app.execute_contract(
        lender3.clone(),
        lpp.clone(),
        &ExecuteLpp::Deposit(),
        &coins(post_deposit, denom),
    )
    .unwrap();

    let balance_nlpn: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<TheCurrency> = app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn.balance.into(), price.0),
        Coin::<TheCurrency>::new(test_deposit - rounding_error)
    );

    // loans should not change asserts for lender2, the default loan
    let balance_lpp: LppBalanceResponse<TheCurrency> = app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance())
        .unwrap();
    dbg!(balance_lpp);
    LeaseWrapper::default().instantiate(
        &mut app,
        Some(lease_id),
        &lpp,
        denom,
        LeaseWrapperConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            downpayment: loan,
            ..LeaseWrapperConfig::default()
        },
    );
    let balance_lpp: LppBalanceResponse<TheCurrency> = app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance())
        .unwrap();
    dbg!(&balance_lpp);

    let balance_nlpn2: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    let price: PriceResponse<TheCurrency> = app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Price())
        .unwrap();
    assert_eq!(
        price::total(balance_nlpn2.balance.into(), price.0),
        Coin::<TheCurrency>::new(test_deposit - rounding_error)
    );

    let balance_nlpn1: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender1,
            },
        )
        .unwrap();

    let balance_nlpn3: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender3,
            },
        )
        .unwrap();

    // check for balance consistency
    assert_eq!(
        balance_lpp.balance_nlpn,
        (balance_nlpn1.balance + balance_nlpn2.balance + balance_nlpn3.balance).into()
    );

    // try to withdraw with overdraft
    let to_burn: u128 = balance_nlpn.balance.u128() - rounding_error + overdraft;
    let resp = app.execute_contract(
        lender2.clone(),
        lpp.clone(),
        &ExecuteLpp::Burn {
            amount: to_burn.into(),
        },
        &[],
    );
    assert!(resp.is_err());

    // partial withdraw
    app.execute_contract(
        lender2.clone(),
        lpp.clone(),
        &ExecuteLpp::Burn {
            amount: withdraw_amount_nlpn.into(),
        },
        &[],
    )
    .unwrap();

    let balance_nlpn: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Balance {
                address: lender2.clone(),
            },
        )
        .unwrap();
    assert_eq!(rest_nlpn, balance_nlpn.balance.u128());

    // full withdraw, should close lender's account
    app.execute_contract(
        lender2.clone(),
        lpp.clone(),
        &ExecuteLpp::Burn {
            amount: (rest_nlpn).into(),
        },
        &[],
    )
    .unwrap();
    let balance_nlpn: BalanceResponse = app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::Balance { address: lender2 })
        .unwrap();
    assert_eq!(0, balance_nlpn.balance.u128());
}

#[test]
fn loan_open_wrong_id() {
    let denom = TheCurrency::SYMBOL;
    let admin = Addr::unchecked(ADMIN);
    let lender = Addr::unchecked("lender");
    let hacker = Addr::unchecked("Mallory");

    let app_balance = 10_000_000_000u128;
    let hacker_balance = 10_000_000;
    let init_deposit = 20_000_000u128;
    let loan = 10_000u128;

    let mut app = mock_app(&[coin(app_balance, denom)]);
    let lease_id = LeaseWrapper::default().store(&mut app);
    let (lpp, _) = LppWrapper::default().instantiate(&mut app, lease_id.into(), denom, 0);
    app.send_tokens(admin.clone(), lender, &[coin(init_deposit, denom)])
        .unwrap();
    app.send_tokens(admin, hacker.clone(), &[coin(hacker_balance, denom)])
        .unwrap();

    let res = app.execute_contract(
        hacker,
        lpp,
        &ExecuteLpp::OpenLoan {
            amount: Coin::<TheCurrency>::new(loan).into(),
        },
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn loan_open_and_repay() {
    const YEAR: u64 = Duration::YEAR.nanos();

    let denom = TheCurrency::SYMBOL;
    let admin = Addr::unchecked(ADMIN);
    let lender = Addr::unchecked("lender");
    let hacker = Addr::unchecked("Mallory");

    let app_balance = 10_000_000_000u128;
    let hacker_balance = 10_000_000;
    let init_deposit = 20_000_000u128;
    let loan1 = 10_000_000u128;
    let loan2 = 5_000_000u128;
    let repay_interest_part = 1_000_000u128;
    let repay_due_part = 1_000_000u128;
    let repay_excess = 1_000_000u128;

    let base_interest_rate = Percent::from_percent(21);
    let addon_optimal_interest_rate = Percent::from_percent(20);
    let utilization_optimal = Percent::from_percent(55);

    let utilization1 = Percent::from_permille((1000 * loan1 / init_deposit) as u32);
    let interest1 = base_interest_rate + addon_optimal_interest_rate.of(utilization1)
        - addon_optimal_interest_rate.of(utilization_optimal);
    dbg!(Percent::from_percent(1)); // scale
    dbg!(utilization1);
    dbg!(interest1);

    // net setup
    let mut app = mock_app(&[coin(app_balance, denom), coin(app_balance, Nls::SYMBOL)]);
    let lease_id = LeaseWrapper::default().store(&mut app);
    let (lpp, _) = LppWrapper::default().instantiate(&mut app, lease_id.into(), denom, 0);
    app.send_tokens(admin.clone(), lender.clone(), &[coin(init_deposit, denom)])
        .unwrap();
    app.send_tokens(
        admin.clone(),
        hacker.clone(),
        &[coin(hacker_balance, denom)],
    )
    .unwrap();

    // initial deposit
    app.execute_contract(
        lender.clone(),
        lpp.clone(),
        &ExecuteLpp::Deposit(),
        &coins(init_deposit, denom),
    )
    .unwrap();

    app.execute_contract(
        lender,
        lpp.clone(),
        &ExecuteLpp::UpdateParameters {
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        },
        &[],
    )
    .unwrap();

    let quote: QueryQuoteResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Quote {
                amount: Coin::<TheCurrency>::new(loan1).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(interest1, quote),
        _ => panic!("no liquidity"),
    }

    // borrow
    let loan_addr1 = LeaseWrapper::default().instantiate(
        &mut app,
        Some(lease_id),
        &lpp,
        denom,
        LeaseWrapperConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            downpayment: loan1,
            ..LeaseWrapperConfig::default()
        },
    );

    // double borrow
    app.execute_contract(
        loan_addr1.clone(),
        lpp.clone(),
        &ExecuteLpp::OpenLoan {
            amount: Coin::<TheCurrency>::new(loan1).into(),
        },
        &[],
    )
    .unwrap_err();

    app.time_shift(Duration::from_nanos(YEAR / 2));

    let total_interest_due = interest1.of(loan1) / 2;

    let resp: LppBalanceResponse<TheCurrency> = app
        .wrap()
        .query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance())
        .unwrap();
    dbg!(&resp);
    assert_eq!(total_interest_due, resp.total_interest_due.into());

    let total_liability = loan1 + loan2 + total_interest_due;
    let utilization2 = Percent::from_permille(
        (1000 * (total_liability) / (init_deposit + total_interest_due)) as u32,
    );
    let interest2 = base_interest_rate + addon_optimal_interest_rate.of(utilization2)
        - addon_optimal_interest_rate.of(utilization_optimal);

    let quote: QueryQuoteResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Quote {
                amount: Coin::<TheCurrency>::new(loan2).into(),
            },
        )
        .unwrap();
    match quote {
        QueryQuoteResponse::QuoteInterestRate(quote) => assert_eq!(interest2, quote),
        _ => panic!("no liquidity"),
    }

    // borrow 2
    let loan_addr2 = LeaseWrapper::default().instantiate(
        &mut app,
        Some(lease_id),
        &lpp,
        denom,
        LeaseWrapperConfig {
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            downpayment: loan2,
            ..LeaseWrapperConfig::default()
        },
    );

    app.time_shift(Duration::from_nanos(YEAR / 2));

    let maybe_loan1: QueryLoanResponse<TheCurrency> = app
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
    assert_eq!(loan1_resp.interest_due, interest1.of(loan1).into());

    // repay from other addr
    app.execute_contract(
        hacker,
        lpp.clone(),
        &ExecuteLpp::RepayLoan(),
        &[coin(loan1, denom)],
    )
    .unwrap_err();

    // repay zero
    app.execute_contract(
        loan_addr1.clone(),
        lpp.clone(),
        &ExecuteLpp::RepayLoan(),
        &[coin(0, denom)],
    )
    .unwrap_err();

    // repay wrong currency
    app.send_tokens(
        admin,
        loan_addr2.clone(),
        &[coin(repay_interest_part, Nls::SYMBOL)],
    )
    .unwrap();
    app.execute_contract(
        loan_addr2,
        lpp.clone(),
        &ExecuteLpp::RepayLoan(),
        &[coin(repay_interest_part, Nls::SYMBOL)],
    )
    .unwrap_err();

    // repay interest part
    app.execute_contract(
        loan_addr1.clone(),
        lpp.clone(),
        &ExecuteLpp::RepayLoan(),
        &[coin(repay_interest_part, denom)],
    )
    .unwrap();

    let maybe_loan1: QueryLoanResponse<TheCurrency> = app
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
        loan1_resp.interest_due,
        (interest1.of(loan1) - repay_interest_part).into()
    );

    // repay interest + due part
    app.execute_contract(
        loan_addr1.clone(),
        lpp.clone(),
        &ExecuteLpp::RepayLoan(),
        &[coin(
            interest1.of(loan1) - repay_interest_part + repay_due_part,
            denom,
        )],
    )
    .unwrap();

    let maybe_loan1: QueryLoanResponse<TheCurrency> = app
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
    assert_eq!(loan1_resp.interest_due, Coin::new(0));

    // repay interest + due part, close the loan
    app.execute_contract(
        loan_addr1.clone(),
        lpp.clone(),
        &ExecuteLpp::RepayLoan(),
        &[coin(loan1 - repay_due_part + repay_excess, denom)],
    )
    .unwrap();

    let maybe_loan1: QueryLoanResponse<TheCurrency> = app
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
    let balance = app.wrap().query_balance(loan_addr1, denom).unwrap();
    assert_eq!(balance.amount.u128(), loan1 - interest1.of(loan1));

    let resp: LppBalanceResponse<TheCurrency> = app
        .wrap()
        .query_wasm_smart(lpp, &QueryLpp::LppBalance())
        .unwrap();

    // accumulated interest, both paid and unpaid
    assert_eq!(
        resp.total_interest_due,
        (interest1.of(loan1) + interest2.of(loan2) / 2u128).into()
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

    let denom = TheCurrency::SYMBOL;
    let admin = Addr::unchecked(ADMIN);

    let lender1 = Addr::unchecked("lender1");
    let lender2 = Addr::unchecked("lender2");
    let recipient = Addr::unchecked("recipient");
    // simplified
    // TODO: any checks for the sender of rewards?
    let treasury = Addr::unchecked("treasury");

    let mut app = mock_app(&[coin(app_balance, denom), coin(app_balance, Nls::SYMBOL)]);
    let lease_id = LeaseWrapper::default().store(&mut app);
    let (lpp, _) = LppWrapper::default().instantiate(&mut app, lease_id.into(), denom, 0);

    app.send_tokens(admin.clone(), lender1.clone(), &[coin(deposit1, denom)])
        .unwrap();
    app.send_tokens(admin.clone(), lender2.clone(), &[coin(deposit2, denom)])
        .unwrap();
    app.send_tokens(
        admin.clone(),
        treasury.clone(),
        &[coin(treasury_balance, Nls::SYMBOL)],
    )
    .unwrap();

    // rewards before deposits
    app.execute_contract(
        treasury.clone(),
        lpp.clone(),
        &ExecuteLpp::DistributeRewards(),
        &[coin(tot_rewards0, Nls::SYMBOL)],
    )
    .unwrap_err();

    // initial deposit
    app.execute_contract(
        lender1.clone(),
        lpp.clone(),
        &ExecuteLpp::Deposit(),
        &coins(deposit1, denom),
    )
    .unwrap();

    // push the price from 1, should be allowed as an interest from previous leases for example.
    app.send_tokens(admin, lpp.clone(), &[coin(lpp_balance_push, denom)])
        .unwrap();

    app.execute_contract(
        treasury.clone(),
        lpp.clone(),
        &ExecuteLpp::DistributeRewards(),
        &[coin(tot_rewards1, Nls::SYMBOL)],
    )
    .unwrap();

    // deposit after disributing rewards should not get anything
    app.execute_contract(
        lender2.clone(),
        lpp.clone(),
        &ExecuteLpp::Deposit(),
        &coins(deposit2, denom),
    )
    .unwrap();

    let resp: RewardsResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, tot_rewards1.into());

    let resp: RewardsResponse = app
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
    app.execute_contract(
        lender2.clone(),
        lpp.clone(),
        &ExecuteLpp::ClaimRewards {
            other_recipient: None,
        },
        &[],
    )
    .unwrap_err();

    // check reward claim
    app.execute_contract(
        lender1.clone(),
        lpp.clone(),
        &ExecuteLpp::ClaimRewards {
            other_recipient: None,
        },
        &[],
    )
    .unwrap();

    let resp: RewardsResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, Coin::new(0));

    let balance = app
        .wrap()
        .query_balance(lender1.clone(), Nls::SYMBOL)
        .unwrap();
    assert_eq!(balance.amount.u128(), tot_rewards1);

    app.execute_contract(
        treasury,
        lpp.clone(),
        &ExecuteLpp::DistributeRewards(),
        &[coin(tot_rewards2, Nls::SYMBOL)],
    )
    .unwrap();

    let resp: RewardsResponse = app
        .wrap()
        .query_wasm_smart(
            lpp.clone(),
            &QueryLpp::Rewards {
                address: lender1.clone(),
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, lender_reward1.into());

    let resp: RewardsResponse = app
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
    app.execute_contract(
        lender1.clone(),
        lpp.clone(),
        &ExecuteLpp::Burn {
            amount: deposit1.into(),
        },
        &[],
    )
    .unwrap();

    let balance = app
        .wrap()
        .query_balance(lender1.clone(), Nls::SYMBOL)
        .unwrap();
    assert_eq!(balance.amount.u128(), tot_rewards1 + lender_reward1);

    // lender account is removed
    let resp: Result<RewardsResponse, _> = app.wrap().query_wasm_smart(
        lpp.clone(),
        &QueryLpp::Rewards {
            address: lender1,
        },
    );

    assert!(resp.is_err());

    // claim rewards to other recipient
    app.execute_contract(
        lender2.clone(),
        lpp.clone(),
        &ExecuteLpp::ClaimRewards {
            other_recipient: Some(recipient.clone()),
        },
        &[],
    )
    .unwrap();

    let resp: RewardsResponse = app
        .wrap()
        .query_wasm_smart(
            lpp,
            &QueryLpp::Rewards {
                address: lender2,
            },
        )
        .unwrap();

    assert_eq!(resp.rewards, Coin::new(0));
    let balance = app.wrap().query_balance(recipient, Nls::SYMBOL).unwrap();
    assert_eq!(balance.amount.u128(), lender_reward2);
}
