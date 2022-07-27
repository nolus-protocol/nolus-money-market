use std::collections::HashSet;

use crate::common::{lpp_wrapper::mock_lpp_quote_query, test_case::TestCase, ADMIN, USER};
use cosmwasm_std::{coins, Addr, DepsMut, Env, Event, MessageInfo, Response};
use cw_multi_test::{next_block, ContractWrapper, Executor};
use finance::{
    coin::{self, Coin},
    currency::{Currency, Nls, SymbolStatic, Usdc},
    percent::Percent,
};
use lease::error::ContractError;
use leaser::msg::{QueryMsg, QuoteResponse};

type TheCurrency = Usdc;

#[test]
fn open_lease() {
    open_lease_impl(Usdc::SYMBOL);
}

#[test]
fn open_lease_another_currency() {
    open_lease_impl(Nls::SYMBOL);
}

#[test]
#[should_panic(expected = "Unknown currency")]
fn init_lpp_with_unknown_currency() {
    let user_addr = Addr::unchecked(USER);

    let unknown_lpn = "token";

    let mut test_case = TestCase::new(unknown_lpn);
    test_case.init(&user_addr, coins(500, unknown_lpn));
    test_case.init_lpp(None);
}

#[test]
#[should_panic(expected = "Single currency version")]
fn open_lease_not_in_lpn_currency() {
    let user_addr = Addr::unchecked(USER);

    let lpn = Usdc::SYMBOL;
    let lease_currency = Nls::SYMBOL;

    let mut test_case = TestCase::new(lpn);
    test_case.init(&user_addr, coins(500, lpn));
    test_case.init_lpp(None);
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
            symbol: lpn.to_owned()
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
    test_case.init_lpp(None);
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
    test_case.init_lpp(None);
    test_case.init_leaser();

    let resp: QuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: coin::funds::<TheCurrency>(100),
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

    assert_eq!(Percent::from_permille(91), resp.annual_interest_rate); // hardcoded until LPP contract is merged

    let resp: QuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.unwrap(),
            &QueryMsg::Quote {
                downpayment: coin::funds::<TheCurrency>(15),
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
    test_case.init_lpp(Some(ContractWrapper::new(
        lpp::contract::execute,
        lpp::contract::instantiate,
        mock_lpp_quote_query,
    )));
    test_case.init_leaser();

    let resp: QuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: coin::funds::<TheCurrency>(100),
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

    assert_eq!(
        Percent::HUNDRED
            .checked_add(Percent::from_percent(3))
            .unwrap(),
        resp.annual_interest_rate
    );
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
        .init_lpp(Some(ContractWrapper::new(
            mock_lpp_execute,
            lpp::contract::instantiate,
            lpp::contract::query,
        )))
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
    test_case.init_lpp(None);
    test_case.init_leaser();

    let res = test_case
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
    assert_eq!(9, res.events.len(), "{:?}", res.events);
    // reflect only returns standard wasm-execute event
    let leaser_exec = &res.events[0];
    assert_eq!(leaser_exec.ty.as_str(), "execute");
    assert_eq!(
        leaser_exec.attributes,
        [("_contract_addr", test_case.leaser_addr.clone().unwrap())]
    );

    let lease_exec = &res.events[1];
    assert_eq!(lease_exec.ty.as_str(), "instantiate");
    assert_eq!(
        lease_exec.attributes,
        [
            ("_contract_addr", "contract2"),
            ("code_id", &test_case.lease_code_id.unwrap().to_string())
        ]
    );

    let lease_exec = &res.events[2];
    assert_eq!(lease_exec.ty.as_str(), "wasm-ls-open");
    assert_eq!(
        lease_exec.attributes,
        [("_contract_addr", "contract2"), ("customer", "user")]
    );

    let lease_reply = &res.events[3];
    assert_eq!(lease_reply.ty.as_str(), "execute");
    assert_eq!(lease_reply.attributes, [("_contract_addr", "contract0")]);

    let lease_reply = &res.events[4];
    assert_eq!(lease_reply.ty.as_str(), "wasm");
    assert_eq!(
        lease_reply.attributes,
        [("_contract_addr", "contract0"), ("method", "try_open_loan")]
    );

    let lease_reply = &res.events[5];
    assert_eq!(lease_reply.ty.as_str(), "transfer");
    assert_eq!(
        lease_reply.attributes,
        [
            ("recipient", "contract2"),
            ("sender", "contract0"),
            ("amount", format!("{}{}", "74", currency).as_ref())
        ]
    );

    let lease_reply = &res.events[6];
    assert_eq!(lease_reply.ty.as_str(), "reply");
    assert_eq!(
        lease_reply.attributes,
        [("_contract_addr", "contract2"), ("mode", "handle_success")]
    );

    let lease_reply = &res.events[7];
    assert_eq!(lease_reply.ty.as_str(), "reply");
    assert_eq!(
        lease_reply.attributes,
        [("_contract_addr", "contract1"), ("mode", "handle_success")]
    );

    let lease_reply = &res.events[8];
    assert_eq!(lease_reply.ty.as_str(), "wasm");
    assert_eq!(
        lease_reply.attributes,
        [
            ("_contract_addr", test_case.leaser_addr.unwrap().as_str()),
            ("lease_address", "contract2")
        ]
    );

    let lease_address = &res.events[8].attributes.get(1).unwrap().value;

    assert_eq!(
        coins(460, currency),
        test_case.app.wrap().query_all_balances(user_addr).unwrap()
    );
    assert_eq!(
        coins(114, currency),
        test_case
            .app
            .wrap()
            .query_all_balances(lease_address)
            .unwrap()
    );
}

fn lease_addr(events: &[Event]) -> String {
    events[8].attributes.get(1).unwrap().value.clone()
}
