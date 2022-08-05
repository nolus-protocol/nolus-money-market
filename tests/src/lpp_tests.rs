use cosmwasm_std::{coins, Addr, coin};
use cw_multi_test::Executor;
use finance::{currency::{Usdc, Currency}, price, percent::Percent};
use finance::coin::{self, Coin,};

use crate::common::{test_case::TestCase, USER, ADMIN, lpp_wrapper::LppWrapper, mock_app, lease_wrapper::{LeaseWrapper, LeaseWrapperConfig}};
use lpp::msg::{ExecuteMsg as ExecuteLpp, BalanceResponse, PriceResponse, LppBalanceResponse};
use lpp::msg::QueryMsg as QueryLpp;

type TheCurrency = Usdc;

#[test]
#[should_panic(expected = "Unauthorized contract Id")]
fn open_loan_unauthorized_contract_id() {
    let denom = TheCurrency::SYMBOL;
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_lpp(None);

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
    let denom = TheCurrency::SYMBOL;
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_lpp(None);

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
    let pushed_price = (lpp_balance_push+init_deposit)/init_deposit;
    let test_deposit = 10_004;
    let rounding_error = test_deposit % pushed_price; // should be 4 for this setup
    let post_deposit = 1_000_000;
    let loan = 1_000_000;
    let overdraft = 5_000;
    let withdraw_amount_nlpn = 1000u128;
    let rest_nlpn = test_deposit/pushed_price - withdraw_amount_nlpn;

    let denom = TheCurrency::SYMBOL;
    let admin = Addr::unchecked(ADMIN);

    let lender1 = Addr::unchecked("lender1");
    let lender2 = Addr::unchecked("lender2");
    let lender3 = Addr::unchecked("lender3");

    let mut app = mock_app(&[coin(app_balance, denom)]);
    let lease_id = LeaseWrapper::default().store(&mut app);
    let (lpp, _) = LppWrapper::default().instantiate(&mut app, lease_id.into(), denom, 0);

    app.send_tokens(admin.clone(), lender1.clone(), &[coin(init_deposit, denom)]).unwrap();
    app.send_tokens(admin.clone(), lender2.clone(), &[coin(test_deposit, denom)]).unwrap();
    app.send_tokens(admin.clone(), lender3.clone(), &[coin(post_deposit, denom)]).unwrap();

    // initial deposit
    app.execute_contract(lender1, lpp.clone(), &ExecuteLpp::Deposit(), &coins(init_deposit, denom)).unwrap();

    // push the price from 1, should be allowed as an interest from previous leases for example.
    app.send_tokens(admin, lpp.clone(), &[coin(lpp_balance_push, denom)]).unwrap();

    let price: PriceResponse<TheCurrency> = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Price()).unwrap();
    assert_eq!(price::total(Coin::new(1_000), price.0), Coin::<TheCurrency>::new(1_000*pushed_price));

    // deposit to check, 
    app.execute_contract(lender2.clone(), lpp.clone(), &ExecuteLpp::Deposit(), &coins(test_deposit, denom)).unwrap();
       
    // got rounding error
    let balance_nlpn: BalanceResponse = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Balance { address: lender2.clone() }).unwrap();
    let price: PriceResponse<TheCurrency> = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Price()).unwrap();
    assert_eq!(price::total(balance_nlpn.balance.into(), price.0), Coin::<TheCurrency>::new(test_deposit - rounding_error));

    // other deposits should not change asserts for lender2
    app.execute_contract(lender3, lpp.clone(), &ExecuteLpp::Deposit(), &coins(post_deposit, denom)).unwrap();

    let balance_nlpn: BalanceResponse = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Balance { address: lender2.clone() }).unwrap();
    let price: PriceResponse<TheCurrency> = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Price()).unwrap();
    assert_eq!(price::total(balance_nlpn.balance.into(), price.0), Coin::<TheCurrency>::new(test_deposit - rounding_error));

    // loans should not change asserts for lender2, the default loan
    let balance_lpn: LppBalanceResponse<TheCurrency> = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance() ).unwrap();
    dbg!(balance_lpn);
    LeaseWrapper::default().instantiate(&mut app, Some(lease_id), &lpp, denom, 
        LeaseWrapperConfig { 
            liability_init_percent: Percent::from_percent(50), // simplify case: borrow == downpayment
            downpayment: loan,
            .. 
            LeaseWrapperConfig::default()}
    );
    let balance_lpn: LppBalanceResponse<TheCurrency> = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::LppBalance() ).unwrap();
    dbg!(balance_lpn);

    let balance_nlpn: BalanceResponse = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Balance { address: lender2.clone() }).unwrap();
    let price: PriceResponse<TheCurrency> = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Price()).unwrap();
    assert_eq!(price::total(balance_nlpn.balance.into(), price.0), Coin::<TheCurrency>::new(test_deposit - rounding_error));

    // try to withdraw with overdraft
    let to_burn: u128 = balance_nlpn.balance.u128() - rounding_error + overdraft;
    let resp = app.execute_contract(lender2.clone(), lpp.clone(), &ExecuteLpp::Burn { amount: to_burn.into() }, &[]);
    assert!(resp.is_err());

    // partial withdraw
    app.execute_contract(lender2.clone(), lpp.clone(), &ExecuteLpp::Burn { amount: withdraw_amount_nlpn.into() }, &[]).unwrap();

    let balance_nlpn: BalanceResponse = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Balance { address: lender2.clone() }).unwrap();
    assert_eq!(rest_nlpn, balance_nlpn.balance.u128());

    // full withdraw, should close lender's account
    app.execute_contract(lender2.clone(), lpp.clone(), &ExecuteLpp::Burn { amount: (rest_nlpn).into() }, &[]).unwrap();
    let balance_nlpn: BalanceResponse = app.wrap().query_wasm_smart(lpp.clone(), &QueryLpp::Balance { address: lender2 }).unwrap();
    assert_eq!(0, balance_nlpn.balance.u128());

}

