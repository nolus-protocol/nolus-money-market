use std::collections::HashSet;

use currency::{
    lease::{Atom, Cro, Osmo},
    lpn::Usdc,
};
use finance::price::{total, total_of};
use finance::{
    coin::{Amount, Coin},
    currency::Currency,
    percent::Percent,
    test,
};
use leaser::msg::{QueryMsg, QuoteResponse};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{coin, Addr, DepsMut, Env, Event, MessageInfo},
    cw_multi_test::{next_block, ContractWrapper, Executor},
};

use crate::common::{
    cwcoin, cwcoins, lpp_wrapper::mock_lpp_quote_query, test_case::TestCase, ADMIN, USER,
};

type TheCurrency = Usdc;

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn open_lease() {
    open_lease_impl::<Usdc, Usdc, Usdc>();
}

#[test]
#[should_panic(expected = "Unsupported currency")]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn open_lease_unsupported_currency_by_oracle() {
    open_lease_impl::<Usdc, Atom, Usdc>();
}

#[test]
#[should_panic(expected = "is not defined in the lpns currency group")]
fn init_lpp_with_unknown_currency() {
    let user_addr = Addr::unchecked(USER);

    type NotLpn = Osmo;

    let mut test_case = TestCase::<NotLpn>::new();
    test_case.init(&user_addr, cwcoins::<NotLpn, _>(500));
    test_case.init_lpp(None);
}

#[test]
#[ignore = "No support for stargate CosmosMsg-s at cw-multi-test, https://app.clickup.com/t/2zgr1q6"]
fn open_lease_not_in_lpn_currency() {
    let user_addr = Addr::unchecked(USER);

    type Lpn = Usdc;
    let lease_currency = Atom::TICKER;

    let mut test_case = TestCase::<Lpn>::new();
    test_case.init(&user_addr, cwcoins::<Lpn, _>(500));
    test_case.init_lpp(None);
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let res = test_case.app.execute_contract(
        user_addr.clone(),
        test_case.leaser_addr.unwrap(),
        &leaser::msg::ExecuteMsg::OpenLease {
            currency: lease_currency.into(),
        },
        &[cwcoin::<Lpn, _>(3)],
    );
    let err = res.unwrap_err();
    // For some reason the downcasting does not work. That is due to different TypeId-s of LeaseError and the root
    // cause stored into the err. Suppose that is a flaw of the cw-multi-test.
    // dbg!(err.root_cause().downcast_ref::<LeaseError>());
    // assert_eq!(
    //     &LeaseError::OracleError(OracleError::Std(StdError::GenericErr { msg: "".into() })),
    //     root_err
    // );
    assert!(err
        .root_cause()
        .to_string()
        .contains("Unsupported currency"));
}

#[test]
fn open_multiple_loans() {
    let user_addr = Addr::unchecked(USER);
    let user1_addr = Addr::unchecked("user1");

    type Lpn = Usdc;

    let mut test_case = TestCase::<Lpn>::new();
    test_case.init(&user_addr, cwcoins::<Lpn, _>(500));
    test_case.init_lpp(None);
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    test_case
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            user1_addr.clone(),
            &[cwcoin::<Lpn, _>(50)],
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
                    currency: Lpn::TICKER.into(),
                },
                &[cwcoin::<Lpn, _>(3)],
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
                currency: Lpn::TICKER.into(),
            },
            &[cwcoin::<Lpn, _>(30)],
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
    type Lpn = TheCurrency;

    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::<Lpn>::new();
    test_case.init(&user_addr, cwcoins::<Lpn, _>(500));
    test_case.init_lpp(None);
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
                downpayment: test::funds::<_, TheCurrency>(100),
                lease_asset: TheCurrency::TICKER.into(),
            },
        )
        .unwrap();

    assert_eq!(
        TryInto::<Coin<TheCurrency>>::try_into(resp.borrow).unwrap(),
        Coin::new(185)
    );
    assert_eq!(
        TryInto::<Coin<TheCurrency>>::try_into(resp.total).unwrap(),
        Coin::new(285)
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
                downpayment: test::funds::<_, TheCurrency>(15),
                lease_asset: TheCurrency::TICKER.into(),
            },
        )
        .unwrap();

    assert_eq!(
        TryInto::<Coin<TheCurrency>>::try_into(resp.borrow).unwrap(),
        Coin::new(27)
    );
    assert_eq!(
        TryInto::<Coin<TheCurrency>>::try_into(resp.total).unwrap(),
        Coin::new(42)
    );
}

fn common_quote_with_conversion(downpayment: Coin<Osmo>, borrow_after_mul2: Coin<TheCurrency>) {
    use super::oracle_tests::{add_feeder, feed_price};

    type Lpn = TheCurrency;

    const LPNS: Amount = 5_000_000_000_000;
    const OSMOS: Amount = 5_000_000_000_000;
    const CROS: Amount = 5_000_000_000_000;

    const USER_ATOMS: Amount = 5_000_000_000;

    let lpp_reserve = vec![
        cwcoin::<Lpn, _>(LPNS),
        cwcoin::<Osmo, _>(OSMOS),
        cwcoin::<Cro, _>(CROS),
    ];

    let user_reserve = cwcoins::<Atom, _>(USER_ATOMS);

    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::<Lpn>::with_reserve(&{
        let mut reserve = cwcoins::<Lpn, _>(1_000_000_000);

        reserve.extend_from_slice(lpp_reserve.as_slice());

        reserve.extend_from_slice(user_reserve.as_slice());

        reserve
    });
    test_case.init(&user_addr, user_reserve);
    test_case.init_lpp_with_funds(
        None,
        vec![
            coin(LPNS, Lpn::BANK_SYMBOL),
            coin(OSMOS, Osmo::BANK_SYMBOL),
            coin(CROS, Cro::BANK_SYMBOL),
        ],
    );
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let feeder_addr = Addr::unchecked("feeder1");

    add_feeder(&mut test_case, feeder_addr.as_str());

    let dpn_lpn_base = Coin::<Osmo>::new(1);
    let dpn_lpn_quote = Coin::<Lpn>::new(2);
    let dpn_lpn_price = total_of(dpn_lpn_base).is(dpn_lpn_quote);

    let lpn_asset_base = Coin::<Lpn>::new(1);
    let lpn_asset_quote = Coin::<Cro>::new(2);
    let lpn_asset_price = total_of(lpn_asset_base).is(lpn_asset_quote);

    feed_price::<Osmo, TheCurrency>(&mut test_case, &feeder_addr, dpn_lpn_base, dpn_lpn_quote);
    feed_price::<Cro, TheCurrency>(
        &mut test_case,
        &feeder_addr,
        lpn_asset_quote,
        lpn_asset_base,
    );

    let resp: QuoteResponse = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: downpayment.into(),
                lease_asset: Cro::TICKER.into(),
            },
        )
        .unwrap();

    assert_eq!(
        TryInto::<Coin<Cro>>::try_into(resp.borrow).unwrap(),
        total(borrow_after_mul2, lpn_asset_price),
        "Borrow amount is different!"
    );
    assert_eq!(
        TryInto::<Coin<Cro>>::try_into(resp.total).unwrap(),
        total(
            total(downpayment, dpn_lpn_price) + borrow_after_mul2,
            lpn_asset_price
        ),
        "Total amount is different!"
    );
}

#[test]
fn test_quote_with_conversion_100() {
    common_quote_with_conversion(Coin::new(100), Coin::new(371));
}

#[test]
fn test_quote_with_conversion_200() {
    common_quote_with_conversion(Coin::new(200), Coin::new(742));
}

#[test]
fn test_quote_with_conversion_5000() {
    common_quote_with_conversion(Coin::new(5000), Coin::new(18571));
}

#[test]
fn test_quote_fixed_rate() {
    type Lpn = TheCurrency;

    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::<Lpn>::new();
    test_case.init(&user_addr, cwcoins::<Lpn, _>(500));
    test_case.init_lpp(Some(ContractWrapper::new(
        lpp::contract::execute,
        lpp::contract::instantiate,
        mock_lpp_quote_query,
    )));
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
                downpayment: test::funds::<_, TheCurrency>(100),
                lease_asset: TheCurrency::TICKER.into(),
            },
        )
        .unwrap();

    assert_eq!(
        TryInto::<Coin<TheCurrency>>::try_into(resp.borrow).unwrap(),
        Coin::new(185)
    );
    assert_eq!(
        TryInto::<Coin<TheCurrency>>::try_into(resp.total).unwrap(),
        Coin::new(285)
    );

    /*   TODO: test with different time periods and amounts in LPP
        103% =
        100% lpp annual_interest_rate (when calling the test version of get_annual_interest_rate() in lpp_querier.rs)
        +
        3% margin_interest_rate of the leaser
    */

    assert_eq!(resp.annual_interest_rate, Percent::HUNDRED);

    assert_eq!(resp.annual_interest_rate_margin, Percent::from_percent(3));
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn open_loans_lpp_fails() {
    type Lpn = Usdc;

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

    let mut test_case = TestCase::<Lpn>::new();
    test_case
        .init(&user_addr, cwcoins::<Lpn, _>(500))
        .init_lpp(Some(ContractWrapper::new(
            mock_lpp_execute,
            lpp::contract::instantiate,
            lpp::contract::query,
        )))
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
                currency: Lpn::TICKER.into(),
            },
            &[cwcoin::<Lpn, _>(30)],
        )
        .unwrap();
}

fn open_lease_impl<Lpn, LeaseC, DownpaymentC>()
where
    Lpn: Currency,
    LeaseC: Currency,
    DownpaymentC: Currency,
{
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::<Lpn>::new();
    test_case.init(&user_addr, vec![cwcoin::<DownpaymentC, _>(500)]);
    test_case.init_lpp(None);
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
                currency: LeaseC::TICKER.into(),
            },
            &[cwcoin::<DownpaymentC, _>(40)],
        )
        .unwrap();

    // ensure the attributes were relayed from the sub-message

    // TODO form -> Lease, self.initial_alarm_schedule(account.balance()?, now)?;
    // assert_eq!(
    //     res.events.len(),
    //     // TODO: Add test cases which are with currency different than LPN and uncomment section
    //     // if currency == TheCurrency::SYMBOL {
    //     10 // } else {
    //        //     11
    //        // }
    // );

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
            ("amount", &format!("{}{}", "74", Lpn::BANK_SYMBOL))
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
        .any(|attribute| attribute == ("_contract_addr", lease_addr)));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute.key == "height"));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute.key == "idx"));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("id", lease_addr)));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("customer", USER)));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("air", "105")));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("currency", Lpn::TICKER)));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("loan-pool-id", lpp_addr)));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("loan-amount", "74")));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("loan-symbol", Lpn::TICKER)));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("downpayment-symbol", Lpn::TICKER)));
    assert!(lease_exec_open
        .attributes
        .iter()
        .any(|attribute| attribute == ("downpayment-amount", "40")));

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
        cwcoins::<Lpn, _>(460),
        test_case.app.wrap().query_all_balances(user_addr).unwrap()
    );
    assert_eq!(
        cwcoins::<Lpn, _>(114),
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
