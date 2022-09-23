use std::collections::HashSet;

use cosmwasm_std::{coins, Addr, DepsMut, Env, Event, MessageInfo, Response};
use cw_multi_test::{next_block, ContractWrapper, Executor};

use currency::{lease::Atom, lpn::Usdc};
use finance::{
    coin::Coin,
    currency::{Currency, SymbolStatic},
    percent::Percent,
    test::{self},
};
use lease::error::ContractError;
use leaser::msg::{QueryMsg, QuoteResponse};

use crate::common::{lpp_wrapper::mock_lpp_quote_query, test_case::TestCase, ADMIN, USER};

type TheCurrency = Usdc;

#[test]
fn open_lease() {
    open_lease_impl(Usdc::SYMBOL);
}

#[test]
fn open_lease_another_currency() {
    open_lease_impl(Atom::SYMBOL);
}

#[test]
#[should_panic(expected = "Unknown currency")]
fn init_lpp_with_unknown_currency() {
    let user_addr = Addr::unchecked(USER);

    let unknown_lpn = "token";

    let mut test_case = TestCase::new(unknown_lpn);
    test_case.init(&user_addr, coins(500, unknown_lpn));
    test_case.init_lpp(None, unknown_lpn);
}

#[test]
#[should_panic(expected = "Single currency version")]
fn open_lease_not_in_lpn_currency() {
    let user_addr = Addr::unchecked(USER);

    let lpn = Usdc::SYMBOL;
    let lease_currency = Atom::SYMBOL;

    let mut test_case = TestCase::new(lpn);
    test_case.init(&user_addr, coins(500, lpn));
    test_case.init_lpp(None, lpn);
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let res = test_case.app.execute_contract(
        user_addr.clone(),
        test_case.leaser_addr.unwrap(),
        &leaser::msg::ExecuteMsg::OpenLease {
            currency: lease_currency.to_string(),
        },
        &coins(3, lpn),
    );
    let err = res.unwrap_err();
    let root_err = err.root_cause().downcast_ref::<ContractError>().unwrap();
    assert_eq!(
        &ContractError::UnknownCurrency {
            symbol: ToOwned::to_owned(lpn)
        },
        root_err
    );
}

#[test]
fn open_multiple_loans() {
    let user_addr = Addr::unchecked(USER);
    let user1_addr = Addr::unchecked("user1");

    const LPN: SymbolStatic = Usdc::SYMBOL;

    let mut test_case = TestCase::new(LPN);
    test_case.init(&user_addr, coins(500, LPN));
    test_case.init_lpp(None, LPN);
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    test_case
        .app
        .send_tokens(Addr::unchecked(ADMIN), user1_addr.clone(), &coins(50, LPN))
        .unwrap();

    let resp: HashSet<Addr> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Leases {
                owner: user_addr.clone(),
            },
        )
        .unwrap();
    assert!(resp.is_empty());

    let mut loans = HashSet::new();
    for _ in 0..5 {
        let res = test_case
            .app
            .execute_contract(
                user_addr.clone(),
                test_case.leaser_addr.clone().unwrap(),
                &leaser::msg::ExecuteMsg::OpenLease {
                    currency: LPN.to_string(),
                },
                &coins(3, LPN),
            )
            .unwrap();
        test_case.app.update_block(next_block);
        let addr = lease_addr(&res.events);
        loans.insert(Addr::unchecked(addr));
    }

    assert_eq!(5, loans.len());

    let res = test_case
        .app
        .execute_contract(
            user1_addr.clone(),
            test_case.leaser_addr.as_ref().unwrap().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: LPN.to_string(),
            },
            &coins(30, LPN),
        )
        .unwrap();
    test_case.app.update_block(next_block);
    let user1_lease_addr = lease_addr(&res.events);

    let resp: HashSet<Addr> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Leases { owner: user1_addr },
        )
        .unwrap();
    assert!(resp.contains(&Addr::unchecked(user1_lease_addr)));
    assert_eq!(1, resp.len());

    let user0_loans: HashSet<Addr> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.unwrap(),
            &QueryMsg::Leases { owner: user_addr },
        )
        .unwrap();
    assert_eq!(loans, user0_loans);
}
#[test]
fn test_quote() {
    const LPN: SymbolStatic = TheCurrency::SYMBOL;

    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::new(LPN);
    test_case.init(&user_addr, coins(500, LPN));
    test_case.init_lpp(None, LPN);
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let resp: QuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: test::funds::<TheCurrency>(100),
            },
        )
        .unwrap();

    assert_eq!(
        Coin::<TheCurrency>::new(185),
        resp.borrow.try_into().unwrap()
    );
    assert_eq!(
        Coin::<TheCurrency>::new(285),
        resp.total.try_into().unwrap()
    );

    /*   TODO: test with different time periods and amounts in LPP
     */

    assert_eq!(
        Percent::from_permille(113),
        resp.annual_interest_rate + resp.annual_interest_rate_margin,
    ); // hardcoded until LPP contract is merged

    let resp: QuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.unwrap(),
            &QueryMsg::Quote {
                downpayment: test::funds::<TheCurrency>(15),
            },
        )
        .unwrap();

    assert_eq!(
        Coin::<TheCurrency>::new(27),
        resp.borrow.try_into().unwrap()
    );
    assert_eq!(Coin::<TheCurrency>::new(42), resp.total.try_into().unwrap());
}

#[test]
fn test_quote_fixed_rate() {
    const LPN: SymbolStatic = TheCurrency::SYMBOL;

    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::new(LPN);
    test_case.init(&user_addr, coins(500, LPN));
    test_case.init_lpp(
        Some(ContractWrapper::new(
            lpp::contract::execute,
            lpp::contract::instantiate,
            mock_lpp_quote_query,
        )),
        LPN,
    );
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let resp: QuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: test::funds::<TheCurrency>(100),
            },
        )
        .unwrap();

    assert_eq!(
        Coin::<TheCurrency>::new(185),
        resp.borrow.try_into().unwrap()
    );
    assert_eq!(
        Coin::<TheCurrency>::new(285),
        resp.total.try_into().unwrap()
    );

    /*   TODO: test with different time periods and amounts in LPP
        103% =
        100% lpp annual_interest_rate (when calling the test version of get_annual_interest_rate() in lpp_querier.rs)
        +
        3% margin_interest_rate of the leaser
    */

    assert_eq!(Percent::HUNDRED, resp.annual_interest_rate,);

    assert_eq!(Percent::from_percent(3), resp.annual_interest_rate_margin,);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn open_loans_lpp_fails() {
    const LPN: SymbolStatic = Usdc::SYMBOL;
    let user_addr = Addr::unchecked(USER);

    fn mock_lpp_execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: lpp::msg::ExecuteMsg,
    ) -> Result<Response, lpp::error::ContractError> {
        match msg {
            lpp::msg::ExecuteMsg::OpenLoan { amount: _ } => {
                Err(lpp::error::ContractError::InsufficientBalance)
            }
            _ => Ok(lpp::contract::execute(deps, env, info, msg)?),
        }
    }

    let mut test_case = TestCase::new(LPN);
    test_case
        .init(&user_addr, coins(500, LPN))
        .init_lpp(
            Some(ContractWrapper::new(
                mock_lpp_execute,
                lpp::contract::instantiate,
                lpp::contract::query,
            )),
            LPN,
        )
        .init_timealarms()
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .init_leaser();

    let _res = test_case
        .app
        .execute_contract(
            user_addr.clone(),
            test_case.leaser_addr.as_ref().unwrap().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: LPN.to_string(),
            },
            &coins(30, LPN),
        )
        .unwrap();
}

fn open_lease_impl(currency: SymbolStatic) {
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(currency);
    test_case.init(&user_addr, coins(500, currency));
    test_case.init_lpp(None, currency);
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let lpp_addr: &str = test_case.lpp_addr.as_ref().unwrap().as_str(); // 0

    let time_alarms_addr: &str = test_case.timealarms.as_ref().unwrap().as_str(); // 1

    let _oracle_addr: &str = test_case.oracle.as_ref().unwrap().as_str(); // 2

    let _treasury_addr: &str = test_case.leaser_addr.as_ref().unwrap().as_str(); // 3

    let _profit_addr: &str = test_case.leaser_addr.as_ref().unwrap().as_str(); // 4

    let leaser_addr: &str = test_case.leaser_addr.as_ref().unwrap().as_str(); // 5

    let lease_addr: Addr = Addr::unchecked("contract6"); // 6
    let lease_addr: &str = lease_addr.as_str();

    let mut res = test_case
        .app
        .execute_contract(
            user_addr.clone(),
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency.to_string(),
            },
            &coins(40, currency),
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(
        res.events.len(),
        // TODO: Add test cases which are with currency different than LPN and uncomment section
        // if currency == TheCurrency::SYMBOL {
        10 // } else {
           //     11
           // }
    );

    // reflect only returns standard wasm-execute event
    let leaser_exec = res.events.remove(0);
    assert_eq!(leaser_exec.ty.as_str(), "execute");
    assert_eq!(leaser_exec.attributes, [("_contract_addr", leaser_addr)]);

    let lease_inst = res.events.remove(0);
    assert_eq!(lease_inst.ty.as_str(), "instantiate");
    assert_eq!(
        lease_inst.attributes,
        [
            ("_contract_addr", lease_addr),
            ("code_id", &test_case.lease_code_id.unwrap().to_string())
        ]
    );

    let lpp_exec = res.events.remove(0);
    assert_eq!(lpp_exec.ty.as_str(), "execute");
    assert_eq!(lpp_exec.attributes, [("_contract_addr", lpp_addr)]);

    let lpp_wasm = res.events.remove(0);
    assert_eq!(lpp_wasm.ty.as_str(), "wasm");
    assert_eq!(
        lpp_wasm.attributes,
        [("_contract_addr", lpp_addr), ("method", "try_open_loan"),]
    );

    let transfer_event = res.events.remove(0);
    assert_eq!(transfer_event.ty.as_str(), "transfer");
    assert_eq!(
        transfer_event.attributes,
        [
            ("recipient", lease_addr),
            ("sender", lpp_addr),
            ("amount", &format!("{}{}", "74", currency))
        ]
    );

    let lease_reply = res.events.remove(0);
    assert_eq!(lease_reply.ty.as_str(), "reply");
    assert_eq!(
        lease_reply.attributes,
        [("_contract_addr", lease_addr), ("mode", "handle_success"),]
    );

    let lease_exec_open = res.events.remove(0);
    assert_eq!(lease_exec_open.ty.as_str(), "wasm-ls-open");
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("_contract_addr", lease_addr),));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute.key == "height",));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute.key == "idx",));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("id", lease_addr),));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("customer", USER),));
    dbg!(&lease_exec_open.attributes);
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("air", "105"),));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("currency", currency),));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("loan-pool-id", lpp_addr),));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("loan-amount", "74"),));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("loan-symbol", currency),));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("downpayment-symbol", currency),));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("downpayment-amount", "40"),));

    // TODO: Add test cases which are with currency different than LPN and uncomment section
    // if currency != Lpn::SYMBOL {
    //     let oracle_exec = res.events.remove(0);
    //     assert_eq!(oracle_exec.ty.as_str(), "execute");
    //     assert_eq!(
    //         oracle_exec.attributes,
    //         [("_contract_addr", oracle_addr)]
    //     );
    //
    //     let oracle_wasm = res.events.remove(0);
    //     assert_eq!(oracle_wasm.ty.as_str(), "wasm");
    //     assert_eq!(
    //         oracle_wasm.attributes,
    //         [
    //             ("_contract_addr", oracle_addr),
    //             ("method", "try_add_price_hook"),
    //         ]
    //     );
    // }

    let leaser_reply = res.events.remove(0);
    assert_eq!(leaser_reply.ty.as_str(), "execute");
    assert_eq!(
        leaser_reply.attributes,
        [("_contract_addr", time_alarms_addr),]
    );

    let leaser_reply = res.events.remove(0);
    assert_eq!(leaser_reply.ty.as_str(), "reply");
    assert_eq!(
        leaser_reply.attributes,
        [("_contract_addr", leaser_addr), ("mode", "handle_success"),]
    );

    let lease_opened = res.events.remove(0);
    assert_eq!(lease_opened.ty.as_str(), "wasm");
    assert_eq!(
        lease_opened.attributes,
        [
            ("_contract_addr", leaser_addr),
            ("lease_address", lease_addr)
        ]
    );

    assert_eq!(
        coins(460, currency),
        test_case.app.wrap().query_all_balances(user_addr).unwrap()
    );
    assert_eq!(
        coins(114, currency),
        test_case.app.wrap().query_all_balances(lease_addr).unwrap()
    );
}

fn lease_addr(events: &[Event]) -> String {
    events
        .last()
        .unwrap()
        .attributes
        .get(1)
        .unwrap()
        .value
        .clone()
}
