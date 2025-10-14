use currencies::{
    LeaseGroup as LeaseCurrencies, Lpn, Lpns, PaymentGroup as PriceCurrencies,
    testing::{LeaseC1, LeaseC2, LeaseC6, LeaseC7},
};
use currency::{CurrencyDef, MemberOf};
use finance::coin::{Amount, Coin};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, DepsMut, Env, Event, MessageInfo},
    cw_multi_test::{self as cw_test, AppResponse, ContractWrapper},
    testing,
};

use crate::common::{
    self, ADDON_OPTIMAL_INTEREST_RATE, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
    lease as lease_mod, leaser as leaser_mod,
    lpp::LppExecuteMsg,
    oracle as oracle_mod,
    protocols::Registry,
    test_case::{
        TestCase,
        builder::BlankBuilder as TestCaseBuilder,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
    },
};

#[test]
fn open_osmo_lease() {
    open_lease_impl::<Lpn, LeaseC7, Lpn>(true);
}

#[test]
fn open_cro_lease() {
    open_lease_impl::<Lpn, LeaseC2, Lpn>(true);
}

#[test]
#[should_panic(expected = "Unsupported currency")]
fn open_lease_unsupported_currency_by_oracle() {
    open_lease_impl::<Lpn, LeaseC6, Lpn>(false);
}

#[test]
fn open_multiple_loans() {
    type LeaseCurrency = LeaseC1;

    let user_addr = testing::user(USER);
    let other_user_addr = testing::user("other_user");

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

    test_case
        .send_funds_from_admin(user_addr.clone(), &[common::cwcoin_from_amount::<Lpn>(450)])
        .send_funds_from_admin(
            other_user_addr.clone(),
            &[common::cwcoin_from_amount::<Lpn>(225)],
        );

    leaser_mod::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        user_addr.clone(),
    );

    for _ in 0..5 {
        let mut response: ResponseWithInterChainMsgs<'_, AppResponse> = test_case
            .app
            .execute(
                user_addr.clone(),
                test_case.address_book.leaser().clone(),
                &leaser::msg::ExecuteMsg::OpenLease {
                    currency: currency::dto::<LeaseCurrency, _>(),
                    max_ltd: None,
                },
                &[common::cwcoin_from_amount::<Lpn>(75)],
            )
            .unwrap();

        response.expect_register_ica(TestCase::DEX_CONNECTION_ID, TestCase::LEASE_ICA_ID);

        let response: AppResponse = response.unwrap_response();

        test_case.app.update_block(cw_test::next_block);

        leaser_mod::assert_lease(
            &test_case.app,
            test_case.address_book.leaser().clone(),
            user_addr.clone(),
            &lease_addr(&response.events),
        );
    }

    let mut response = test_case
        .app
        .execute(
            other_user_addr.clone(),
            test_case.address_book.leaser().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[common::cwcoin_from_amount::<Lpn>(78)],
        )
        .unwrap();

    response.expect_register_ica(TestCase::DEX_CONNECTION_ID, TestCase::LEASE_ICA_ID);

    let response: AppResponse = response.unwrap_response();

    test_case.app.update_block(cw_test::next_block);

    leaser_mod::assert_lease(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        other_user_addr,
        &lease_addr(&response.events),
    );
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn open_loans_lpp_fails() {
    type LeaseCurrency = LeaseC1;

    let user_addr = testing::user(USER);
    let downpayment = common::cwcoin_from_amount::<Lpn>(86);

    fn mock_lpp_execute(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: LppExecuteMsg,
    ) -> Result<Response, lpp::contract::ContractError> {
        match msg {
            lpp::msg::ExecuteMsg::OpenLoan { amount: _ } => {
                Err(lpp::contract::ContractError::InsufficientBalance)
            }
            _ => Ok(lpp::contract::execute(deps, env, info, msg)?),
        }
    }

    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            Some(
                ContractWrapper::new(
                    mock_lpp_execute,
                    lpp::contract::instantiate,
                    lpp::contract::query,
                )
                .with_sudo(lpp::contract::sudo),
            ),
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

    test_case.send_funds_from_admin(user_addr.clone(), std::slice::from_ref(&downpayment));

    let _res: AppResponse = test_case
        .app
        .execute(
            user_addr,
            test_case.address_book.leaser().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[downpayment],
        )
        .unwrap()
        .unwrap_response();
}

#[test]
#[should_panic(expected = "The transaction amount should worth")]
fn open_loans_insufficient_transaction_amount() {
    open_loans_insufficient_amount(49);
}

#[test]
#[should_panic(expected = "The asset amount should worth")]
fn open_loans_insufficient_asset() {
    open_loans_insufficient_amount(62);
}

fn open_lease_impl<Lpn, LeaseC, DownpaymentC>(feed_prices: bool)
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns> + MemberOf<PriceCurrencies>,
    LeaseC: CurrencyDef,
    LeaseC::Group: MemberOf<LeaseCurrencies> + MemberOf<PriceCurrencies>,
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PriceCurrencies>,
{
    let user_addr = testing::user(USER);

    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        common::cwcoin_from_amount::<Lpn>(1_000_000_000),
        common::cwcoin_dex::<Lpn>(1_000_000_000),
        common::cwcoin_from_amount::<LeaseC>(1_000_000_000),
        common::cwcoin_dex::<LeaseC>(1_000_000_000),
        common::cwcoin_from_amount::<DownpaymentC>(1_000_000_000),
        common::cwcoin_dex::<DownpaymentC>(1_000_000_000),
    ])
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

    test_case.send_funds_from_admin(
        user_addr.clone(),
        &[common::cwcoin_from_amount::<DownpaymentC>(500)],
    );

    let leaser_addr: Addr = test_case.address_book.leaser().clone();

    if feed_prices {
        oracle_mod::add_feeder(&mut test_case, user_addr.clone());

        if !currency::equal::<DownpaymentC, Lpn>() {
            oracle_mod::feed_price(
                &mut test_case,
                user_addr.clone(),
                Coin::<DownpaymentC>::new(1),
                Coin::<Lpn>::new(1),
            );
        }

        if !currency::equal::<LeaseC, Lpn>() {
            oracle_mod::feed_price(
                &mut test_case,
                user_addr.clone(),
                Coin::<LeaseC>::new(1),
                Coin::<Lpn>::new(1),
            );
        }
    }

    let downpayment: Coin<DownpaymentC> = Coin::new(79);
    let quote = leaser_mod::query_quote::<DownpaymentC, LeaseC>(
        &test_case.app,
        leaser_addr.clone(),
        downpayment,
        None,
    );
    let exp_borrow: Coin<Lpn> = quote.borrow.try_into().unwrap();

    let mut response: ResponseWithInterChainMsgs<'_, AppResponse> = test_case
        .app
        .execute(
            user_addr,
            leaser_addr,
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<LeaseC, _>(),
                max_ltd: None,
            },
            &[common::cwcoin(downpayment)],
        )
        .unwrap();

    response.expect_register_ica(TestCase::DEX_CONNECTION_ID, TestCase::LEASE_ICA_ID);

    let lease = lease_addr(&response.unwrap_response().events);

    lease_mod::complete_initialization(
        &mut test_case.app,
        TestCase::DEX_CONNECTION_ID,
        lease,
        downpayment,
        exp_borrow,
    );
}

fn lease_addr(events: &[Event]) -> Addr {
    Addr::unchecked(
        events
            .last()
            .unwrap()
            .attributes
            .get(1)
            .unwrap()
            .value
            .to_owned(),
    )
}

fn open_loans_insufficient_amount(downpayment: Amount) {
    type LeaseCurrency = LeaseC1;

    let user_addr = testing::user(USER);
    let incoming_funds = common::cwcoin_from_amount::<Lpn>(200);
    let downpayment_amount = common::cwcoin_from_amount::<Lpn>(downpayment);

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

    test_case.send_funds_from_admin(user_addr.clone(), std::slice::from_ref(&incoming_funds));

    let _res: AppResponse = test_case
        .app
        .execute(
            user_addr,
            test_case.address_book.leaser().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[downpayment_amount],
        )
        .unwrap()
        .unwrap_response();
}
