use std::collections::HashSet;

use cosmwasm_std::{coins, Addr, Coin};
use cw_multi_test::{next_block, Executor};
use leaser::msg::{QueryMsg, QuoteResponse};

use crate::common::{test_case::TestCase, ADMIN, USER};

#[test]
fn open_lease() {
    let denom = "UST";
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_lpp(None);
    test_case.init_leaser();

    let res = test_case
        .app
        .execute_contract(
            user_addr.clone(),
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: denom.to_string(),
            },
            &coins(40, denom),
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(8, res.events.len(), "{:?}", res.events);
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

    let lease_reply = &res.events[2];
    assert_eq!(lease_reply.ty.as_str(), "execute");
    assert_eq!(lease_reply.attributes, [("_contract_addr", "contract0")]);

    let lease_reply = &res.events[3];
    assert_eq!(lease_reply.ty.as_str(), "wasm");
    assert_eq!(
        lease_reply.attributes,
        [("_contract_addr", "contract0"), ("method", "try_open_loan")]
    );

    let lease_reply = &res.events[4];
    assert_eq!(lease_reply.ty.as_str(), "transfer");
    assert_eq!(
        lease_reply.attributes,
        [
            ("recipient", "contract2"),
            ("sender", "contract0"),
            ("amount", "74UST")
        ]
    );

    let lease_reply = &res.events[5];
    assert_eq!(lease_reply.ty.as_str(), "reply");
    assert_eq!(
        lease_reply.attributes,
        [("_contract_addr", "contract2"), ("mode", "handle_success")]
    );

    let lease_reply = &res.events[6];
    assert_eq!(lease_reply.ty.as_str(), "reply");
    assert_eq!(
        lease_reply.attributes,
        [("_contract_addr", "contract1"), ("mode", "handle_success")]
    );

    let lease_reply = &res.events[7];
    assert_eq!(lease_reply.ty.as_str(), "wasm");
    assert_eq!(
        lease_reply.attributes,
        [
            ("_contract_addr", test_case.leaser_addr.unwrap().as_str()),
            ("lease_address", "contract2")
        ]
    );

    let lease_address = &res.events[7].attributes.get(1).unwrap().value;

    assert_eq!(
        coins(460, denom),
        test_case.app.wrap().query_all_balances(user_addr).unwrap()
    );
    assert_eq!(
        coins(114, denom),
        test_case
            .app
            .wrap()
            .query_all_balances(lease_address)
            .unwrap()
    );
}
#[test]
fn open_lease_custom_currency() {
    let denom = "unolus";
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_lpp(None);
    test_case.init_leaser();

    assert_eq!(
        coins(500, denom),
        test_case
            .app
            .wrap()
            .query_all_balances(user_addr.clone())
            .unwrap()
    );

    let res = test_case
        .app
        .execute_contract(
            user_addr.clone(),
            test_case.leaser_addr.unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: denom.to_string(),
            },
            &coins(3, denom),
        )
        .unwrap();

    let lease_address = &res.events[7].attributes.get(1).unwrap().value;

    assert_eq!(
        coins(497, denom),
        test_case.app.wrap().query_all_balances(user_addr).unwrap()
    );
    assert_eq!(
        coins(5, denom),
        test_case
            .app
            .wrap()
            .query_all_balances(lease_address)
            .unwrap()
    );
}

#[test]
fn open_multiple_loans() {
    let denom = "unolus";
    let user_addr = Addr::unchecked(USER);
    let user1_addr = Addr::unchecked("user1");

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_lpp(None);
    test_case.init_leaser();

    test_case
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            user1_addr.clone(),
            &coins(50, denom),
        )
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
                    currency: denom.to_string(),
                },
                &coins(3, denom),
            )
            .unwrap();
        test_case.app.update_block(next_block);
        let addr = res.events[7].attributes.get(1).unwrap().value.clone();
        loans.insert(Addr::unchecked(addr));
    }

    assert_eq!(5, loans.len());

    let res = test_case
        .app
        .execute_contract(
            user1_addr.clone(),
            test_case.leaser_addr.as_ref().unwrap().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: denom.to_string(),
            },
            &coins(30, denom),
        )
        .unwrap();
    test_case.app.update_block(next_block);
    let user1_lease_addr = res.events[7].attributes.get(1).unwrap().value.clone();

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
    let denom = "UST";
    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_lpp(None);
    test_case.init_leaser();

    let resp: QuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.unwrap(),
            &QueryMsg::Quote {
                downpayment: Coin::new(100, denom),
            },
        )
        .unwrap();

    assert_eq!(185, resp.borrow.amount.u128());
}
