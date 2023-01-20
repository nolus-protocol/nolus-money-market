use std::{any::type_name, collections::HashSet};

use currency::{
    lease::{Atom, Cro, Osmo},
    lpn::Usdc,
};
use finance::{
    coin::{Amount, Coin},
    currency::Currency,
    percent::Percent,
    price::{total, total_of},
    test,
};
use leaser::msg::{QueryMsg, QuoteResponse};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{coin, Addr, Coin as CwCoin, DepsMut, Env, Event, MessageInfo},
    cw_multi_test::{next_block, ContractWrapper, Executor},
    testing::new_custom_msg_queue,
};

use crate::common::{
    cwcoin, cwcoins,
    lease_wrapper::complete_lease_initialization,
    lpp_wrapper::mock_lpp_quote_query,
    oracle_wrapper::{add_feeder, feed_price},
    test_case::TestCase,
    ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
};

type TheCurrency = Usdc;

#[test]
fn open_osmo_lease() {
    open_lease_impl::<Usdc, Osmo, Usdc>(true);
}

#[test]
#[ignore = "Fixed by TODO; contracts/lease/src/contract/state/buy_asset.rs:109 @ 5ff50b0302ba07a68b00440d670cdf8135fb1f8b"]
fn open_cro_lease() {
    open_lease_impl::<Usdc, Cro, Usdc>(true);
}

#[test]
#[should_panic(expected = "Unsupported currency")]
fn open_lease_unsupported_currency_by_oracle() {
    open_lease_impl::<Usdc, Atom, Usdc>(false);
}

#[test]
#[should_panic(expected = "is not defined in the lpns currency group")]
fn init_lpp_with_unknown_currency() {
    let user_addr = Addr::unchecked(USER);

    type NotLpn = Osmo;

    let mut test_case = TestCase::<NotLpn>::new(None);
    test_case.init(&user_addr, cwcoins::<NotLpn, _>(500));
    test_case.init_lpp(
        None,
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
    );
}

#[test]
#[should_panic = "Unsupported currency"]
fn open_lease_not_in_lpn_currency() {
    let user_addr = Addr::unchecked(USER);

    type Lpn = Usdc;
    let lease_currency = Atom::TICKER;

    let neutron_message_queue = new_custom_msg_queue();

    let mut test_case = TestCase::<Lpn>::new(Some(neutron_message_queue.clone()));
    test_case.init(&user_addr, cwcoins::<Lpn, _>(500));
    test_case.init_lpp(
        None,
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
    );
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let downpayment: CwCoin = cwcoin::<Lpn, _>(3);

    let _res = test_case
        .app
        .execute_contract(
            user_addr.clone(),
            test_case.leaser_addr.unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: lease_currency.into(),
            },
            &[downpayment.clone()],
        )
        .unwrap();

    complete_lease_initialization::<Lpn>(
        &mut test_case.app,
        &neutron_message_queue,
        &Addr::unchecked("contract6"),
        downpayment,
    );

    // let err = res.unwrap_err();
    // For some reason the downcasting does not work. That is due to different TypeId-s of LeaseError and the root
    // cause stored into the err. Suppose that is a flaw of the cw-multi-test.
    // dbg!(err.root_cause().downcast_ref::<LeaseError>());
    // assert_eq!(
    //     &LeaseError::OracleError(OracleError::Std(StdError::GenericErr { msg: "".into() })),
    //     root_err
    // );

    // assert!(err
    //     .root_cause()
    //     .to_string()
    //     .contains("Unsupported currency"));
}

#[test]
fn open_multiple_loans() {
    let user_addr = Addr::unchecked(USER);
    let user1_addr = Addr::unchecked("user1");

    type Lpn = Usdc;
    type LeaseCurrency = Atom;

    let mut test_case = TestCase::<Lpn>::new(None);
    test_case.init(&user_addr, cwcoins::<Lpn, _>(500));
    test_case.init_lpp(
        None,
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
                    currency: LeaseCurrency::TICKER.into(),
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
                currency: LeaseCurrency::TICKER.into(),
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
    type Downpayment = Lpn;
    type LeaseCurrency = Osmo;

    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::<Lpn>::new(None);
    test_case.init(&user_addr, cwcoins::<Lpn, _>(500));
    test_case.init_lpp(
        None,
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
    );
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let feeder = setup_feeder(&mut test_case);
    feed_price::<_, LeaseCurrency, Lpn>(&mut test_case, &feeder, Coin::new(2), Coin::new(1));

    let resp = query_quote::<_, Downpayment, LeaseCurrency>(&test_case, Coin::new(100));

    assert_eq!(resp.borrow.try_into(), Ok(Coin::<Lpn>::new(185)));
    assert_eq!(
        resp.total.try_into(),
        Ok(Coin::<LeaseCurrency>::new(100 * 2 + 185 * 2))
    );

    /*   TODO: test with different time periods and amounts in LPP
     */

    assert_eq!(
        Percent::from_permille(113),
        resp.annual_interest_rate + resp.annual_interest_rate_margin,
    ); // hardcoded until LPP contract is merged

    let resp = query_quote::<_, Downpayment, LeaseCurrency>(&test_case, Coin::new(15));

    assert_eq!(resp.borrow.try_into(), Ok(Coin::<Lpn>::new(27)));
    assert_eq!(
        resp.total.try_into(),
        Ok(Coin::<LeaseCurrency>::new(15 * 2 + 27 * 2))
    );
}

fn common_quote_with_conversion(downpayment: Coin<Osmo>, borrow_after_mul2: Coin<TheCurrency>) {
    type Lpn = TheCurrency;
    type LeaseCurrency = Cro;

    const LPNS: Amount = 5_000_000_000_000;
    const OSMOS: Amount = 5_000_000_000_000;
    const CROS: Amount = 5_000_000_000_000;

    const USER_ATOMS: Amount = 5_000_000_000;

    let lpp_reserve = vec![
        cwcoin::<Lpn, _>(LPNS),
        cwcoin::<Osmo, _>(OSMOS),
        cwcoin::<LeaseCurrency, _>(CROS),
    ];

    let user_reserve = cwcoins::<Atom, _>(USER_ATOMS);

    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::<Lpn>::with_reserve(None, &{
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
            coin(CROS, LeaseCurrency::BANK_SYMBOL),
        ],
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
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
    let lpn_asset_quote = Coin::<LeaseCurrency>::new(2);
    let lpn_asset_price = total_of(lpn_asset_base).is(lpn_asset_quote);

    feed_price::<_, Osmo, TheCurrency>(&mut test_case, &feeder_addr, dpn_lpn_base, dpn_lpn_quote);
    feed_price::<_, LeaseCurrency, TheCurrency>(
        &mut test_case,
        &feeder_addr,
        lpn_asset_quote,
        lpn_asset_base,
    );

    let resp = query_quote::<_, _, LeaseCurrency>(&test_case, downpayment);

    assert_eq!(
        resp.borrow.try_into(),
        Ok(borrow_after_mul2),
        "Borrow amount is different!"
    );
    assert_eq!(
        resp.total.try_into(),
        Ok(total(
            total(downpayment, dpn_lpn_price) + borrow_after_mul2,
            lpn_asset_price
        )),
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
    type Downpayment = Lpn;
    type LeaseCurrency = Osmo;

    let user_addr = Addr::unchecked(USER);
    let mut test_case = TestCase::<Lpn>::new(None);
    test_case.init(&user_addr, cwcoins::<Lpn, _>(500));
    test_case.init_lpp(
        Some(ContractWrapper::new(
            lpp::contract::execute,
            lpp::contract::instantiate,
            mock_lpp_quote_query,
        )),
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
    );
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let feeder = setup_feeder(&mut test_case);
    feed_price::<_, LeaseCurrency, Lpn>(&mut test_case, &feeder, Coin::new(3), Coin::new(1));
    let resp =
        query_quote::<_, Downpayment, LeaseCurrency>(&test_case, Coin::<Downpayment>::new(100));

    assert_eq!(resp.borrow.try_into(), Ok(Coin::<Lpn>::new(185)));
    assert_eq!(
        resp.total.try_into(),
        Ok(Coin::<LeaseCurrency>::new(100 * 3 + 185 * 3))
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

fn setup_feeder(test_case: &mut TestCase<Usdc>) -> Addr {
    let feeder = Addr::unchecked("feeder_main");
    add_feeder(test_case, &feeder);
    feeder
}

fn query_quote<LpnC, DownpaymentC, LeaseC>(
    test_case: &TestCase<LpnC>,
    downpayment: Coin<DownpaymentC>,
) -> QuoteResponse
where
    LpnC: Currency,
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: test::funds::<_, DownpaymentC>(downpayment.into()),
                lease_asset: LeaseC::TICKER.into(),
            },
        )
        .unwrap()
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn open_loans_lpp_fails() {
    type Lpn = Usdc;
    type LeaseCurrency = Atom;

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

    let mut test_case = TestCase::<Lpn>::new(None);
    test_case
        .init(&user_addr, cwcoins::<Lpn, _>(500))
        .init_lpp(
            Some(ContractWrapper::new(
                mock_lpp_execute,
                lpp::contract::instantiate,
                lpp::contract::query,
            )),
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
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
                currency: LeaseCurrency::TICKER.into(),
            },
            &[cwcoin::<Lpn, _>(30)],
        )
        .unwrap();
}

fn open_lease_impl<Lpn, LeaseC, DownpaymentC>(feed_prices: bool)
where
    Lpn: Currency,
    LeaseC: Currency,
    DownpaymentC: Currency,
{
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::<Lpn>::new(Some(new_custom_msg_queue()));
    test_case.init(&user_addr, vec![cwcoin::<DownpaymentC, _>(500)]);
    test_case.init_lpp(
        None,
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
    );
    test_case.init_timealarms();
    test_case.init_oracle(None);
    test_case.init_treasury();
    test_case.init_profit(24);
    test_case.init_leaser();

    let _lpp_addr: Addr = test_case.lpp_addr.as_ref().unwrap().clone(); // 0

    let _time_alarms_addr: Addr = test_case.timealarms.as_ref().unwrap().clone(); // 1

    let _oracle_addr: Addr = test_case.oracle.as_ref().unwrap().clone(); // 2

    let _treasury_addr: Addr = test_case.leaser_addr.as_ref().unwrap().clone(); // 3

    let _profit_addr: Addr = test_case.leaser_addr.as_ref().unwrap().clone(); // 4

    let leaser_addr: Addr = test_case.leaser_addr.as_ref().unwrap().clone(); // 5

    let lease_addr: Addr = Addr::unchecked("contract6"); // 6

    if feed_prices {
        add_feeder(&mut test_case, user_addr.clone());

        if type_name::<DownpaymentC>() != type_name::<Lpn>() {
            feed_price(
                &mut test_case,
                &user_addr,
                Coin::<DownpaymentC>::new(1),
                Coin::<Lpn>::new(1),
            );
        }

        if type_name::<LeaseC>() != type_name::<Lpn>() {
            feed_price(
                &mut test_case,
                &user_addr,
                Coin::<LeaseC>::new(1),
                Coin::<Lpn>::new(1),
            );
        }
    }

    let downpayment: CwCoin = cwcoin::<DownpaymentC, _>(40);

    test_case
        .app
        .execute_contract(
            user_addr.clone(),
            leaser_addr,
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: LeaseC::TICKER.into(),
            },
            &[downpayment.clone()],
        )
        .unwrap();

    complete_lease_initialization::<Lpn>(
        &mut test_case.app,
        test_case.custom_message_queue.as_deref().unwrap(),
        &lease_addr,
        downpayment,
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
