use std::slice;

use currencies::{Lpn, Lpns, Nls};
use currency::{CurrencyDef, MemberOf};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    zero::Zero,
};
use platform::bank;
use profit::{
    msg::{ConfigResponse, ExecuteMsg, QueryMsg},
    typedefs::CadenceHours,
};
use sdk::{
    cosmwasm_std::{self, Addr, Event},
    cw_multi_test::AppResponse,
    testing,
};
use timealarms::msg::DispatchAlarmsResponse;

use crate::common::{
    self, ADMIN, CwCoin, USER, ibc,
    protocols::Registry,
    swap::DexDenom,
    test_case::{
        TestCase, builder::BlankBuilder as TestCaseBuilder, response::ResponseWithInterChainMsgs,
    },
};

fn test_case_with<Lpn>(
    cadence_hours: CadenceHours,
    custom_reserve: Option<&[CwCoin]>,
) -> TestCase<Addr, Addr, Addr, (), (), (), Addr, Addr>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    custom_reserve
        .map_or_else(
            TestCaseBuilder::<Lpn>::new,
            TestCaseBuilder::<Lpn>::with_reserve,
        )
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(cadence_hours)
        .into_generic()
}

fn test_case<Lpn>() -> TestCase<Addr, Addr, Addr, (), (), (), Addr, Addr>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    test_case_with::<Lpn>(2, None)
}

#[test]
fn update_config() {
    const INITIAL_CACDENCE_HOURS: CadenceHours = 2;
    const UPDATED_CACDENCE_HOURS: CadenceHours = INITIAL_CACDENCE_HOURS + 1;

    let mut test_case = test_case_with::<Lpn>(INITIAL_CACDENCE_HOURS, None);

    let ConfigResponse { cadence_hours } = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.profit().clone(),
            &QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(cadence_hours, INITIAL_CACDENCE_HOURS);

    () = test_case
        .app
        .execute(
            testing::user(ADMIN),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::Config {
                cadence_hours: UPDATED_CACDENCE_HOURS,
            },
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let ConfigResponse { cadence_hours } = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.profit().clone(),
            &QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(cadence_hours, UPDATED_CACDENCE_HOURS);
}

#[test]
fn update_config_unauthorized() {
    const INITIAL_CACDENCE_HOURS: CadenceHours = 2;
    const UPDATED_CACDENCE_HOURS: CadenceHours = INITIAL_CACDENCE_HOURS + 1;

    let mut test_case = test_case_with::<Lpn>(INITIAL_CACDENCE_HOURS, None);

    assert!(
        test_case
            .app
            .execute(
                testing::user(USER),
                test_case.address_book.profit().clone(),
                &ExecuteMsg::Config {
                    cadence_hours: UPDATED_CACDENCE_HOURS
                },
                &[],
            )
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("Unauthorized")
    );
}

#[test]
fn on_alarm_from_unknown() {
    let user_addr: Addr = testing::user(USER);

    let mut test_case = test_case::<Lpn>();

    test_case.send_funds_from_admin(user_addr.clone(), &[common::cwcoin_from_amount::<Lpn>(500)]);

    let query_treasury_balance = |test_case: &TestCase<_, Addr, _, _, _, _, _, _>| {
        common::query_all_balances(test_case.address_book.treasury(), test_case.app.query())
    };

    let treasury_balance = query_treasury_balance(&test_case);

    _ = test_case
        .app
        .execute(
            user_addr,
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[common::cwcoin_from_amount::<Lpn>(40)],
        )
        .unwrap_err();

    //assert that no transfer is made to treasury
    assert_eq!(treasury_balance, query_treasury_balance(&test_case));
}

#[test]
fn on_alarm_zero_balance() {
    let time_oracle_addr = testing::user("time");

    let mut test_case = test_case::<Lpn>();

    test_case.send_funds_from_admin(time_oracle_addr, &[common::cwcoin_from_amount::<Lpn>(500)]);

    () = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();
}

struct InitTreasuryBalancesResult<Lpn> {
    native: Coin<Nls>,
    lpn: Coin<Lpn>,
}

fn init_treasury_balances<
    Lpn,
    ProtocolsRegistry,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
>(
    test_case: &TestCase<ProtocolsRegistry, Addr, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>,
) -> InitTreasuryBalancesResult<Lpn>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    InitTreasuryBalancesResult {
        native: bank::balance(test_case.address_book.treasury(), test_case.app.query()).unwrap(),
        lpn: bank::balance(test_case.address_book.treasury(), test_case.app.query()).unwrap(),
    }
}

struct SendAlarmAndMaybeSwapResult {
    response: AppResponse,
    lpn_profit_swap_out: Coin<Nls>,
    has_swap: bool,
}

fn send_alarm_and_maybe_swap<Lpn, ProtocolsRegistry, Reserve, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<ProtocolsRegistry, Addr, Addr, Reserve, Leaser, Lpp, Oracle, Addr>,
    lpn_profit: Option<(Coin<Lpn>, CwCoin, Coin<Nls>)>,
) -> SendAlarmAndMaybeSwapResult
where
    Lpn: CurrencyDef,
{
    let mut response: ResponseWithInterChainMsgs<'_, AppResponse> = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    if let Some((lpn_profit_swap_in, lpn_profit_swap_in_cw, lpn_profit_swap_out)) = lpn_profit {
        let transfer_amount: CwCoin = ibc::expect_transfer(
            &mut response,
            TestCase::PROFIT_IBC_CHANNEL,
            test_case.address_book.profit().as_str(),
            test_case.address_book.profit_ica().as_str(),
        );

        assert_eq!(transfer_amount, lpn_profit_swap_in_cw);

        let response: AppResponse = response.unwrap_response();

        // ensure the attributes were relayed from the sub-message
        assert_eq!(
            response.events.as_slice(),
            &[Event::new("execute")
                .add_attribute("_contract_address", test_case.address_book.profit())]
        );

        let response = ibc::do_transfer(
            &mut test_case.app,
            test_case.address_book.profit().clone(),
            test_case.address_book.profit_ica().clone(),
            false,
            &transfer_amount,
        );

        let requests = common::swap::expect_swap(
            response,
            TestCase::DEX_CONNECTION_ID,
            TestCase::PROFIT_ICA_ID,
            |_| {},
        );

        let mut response: ResponseWithInterChainMsgs<'_, ()> = common::swap::do_swap(
            &mut test_case.app,
            test_case.address_book.profit().clone(),
            test_case.address_book.profit_ica().clone(),
            requests.into_iter(),
            |amount: Amount, from_denom: DexDenom<'_>, to_denom: DexDenom<'_>| {
                assert_eq!(amount, lpn_profit_swap_in.into());
                assert_eq!(from_denom, Lpn::dex());
                assert_eq!(to_denom, Nls::dex());

                lpn_profit_swap_out.into()
            },
        )
        .ignore_response();

        let transfer_amount: CwCoin = ibc::expect_remote_transfer(
            &mut response,
            TestCase::DEX_CONNECTION_ID,
            TestCase::PROFIT_ICA_ID,
        );

        assert_eq!(transfer_amount.amount.u128(), lpn_profit_swap_out.into());

        let response = ibc::do_transfer(
            &mut test_case.app,
            test_case.address_book.profit_ica().clone(),
            test_case.address_book.profit().clone(),
            true,
            &transfer_amount,
        )
        .unwrap_response();

        SendAlarmAndMaybeSwapResult {
            response,
            lpn_profit_swap_out,
            has_swap: true,
        }
    } else {
        SendAlarmAndMaybeSwapResult {
            response: response.unwrap_response(),
            lpn_profit_swap_out: Zero::ZERO,
            has_swap: false,
        }
    }
}

fn total_native_profit(native_profit: Coin<Nls>, lpn_profit_swap_out: Coin<Nls>) -> Coin<Nls> {
    (native_profit + lpn_profit_swap_out).saturating_sub(::profit::profit::Profit::IBC_FEE_RESERVE)
}

fn expect_transfer_events<ProtocolsRegistry, Reserve, Leaser, Lpp, Oracle>(
    test_case: &TestCase<ProtocolsRegistry, Addr, Addr, Reserve, Leaser, Lpp, Oracle, Addr>,
    alarm_result: SendAlarmAndMaybeSwapResult,
    total_native_profit: Coin<Nls>,
) {
    let SendAlarmAndMaybeSwapResult {
        mut response,
        has_swap,
        ..
    } = alarm_result;

    if has_swap {
        let sudo = response.events.remove(0);
        assert_eq!(sudo.ty.as_str(), "sudo");
        assert_eq!(
            sudo.attributes,
            [(
                "_contract_address",
                test_case.address_book.profit().as_str()
            )]
        );
    }

    assert_eq!(response.events.len(), 4, "{:?}", response.events);

    let profit_exec = &response.events[0];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [(
            "_contract_address",
            test_case.address_book.profit().as_str()
        )]
    );

    let tr_profit = &response.events[1];
    assert_eq!(tr_profit.ty.as_str(), "wasm-tr-profit");
    assert_eq!(
        tr_profit.attributes,
        [
            (
                "_contract_address",
                test_case.address_book.profit().as_str()
            ),
            ("height", &test_case.app.block_info().height.to_string()),
            ("at", &crate::block_time(test_case).nanos().to_string()),
            ("idx", "0"),
            (
                "profit-amount-amount",
                &Amount::from(total_native_profit).to_string()
            ),
            ("profit-amount-symbol", Nls::ticker())
        ]
    );

    let transfer = &response.events[2];

    assert_eq!(transfer.ty.as_str(), "transfer", "{transfer:?}");
    assert_eq!(
        transfer.attributes,
        [
            ("recipient", test_case.address_book.treasury().as_str()),
            ("sender", test_case.address_book.profit().as_str()),
            (
                "amount",
                &format!("{}{}", Amount::from(total_native_profit), Nls::bank())
            )
        ]
    );

    let time_alarms_exec = &response.events[3];
    assert_eq!(
        time_alarms_exec.ty.as_str(),
        "execute",
        "{time_alarms_exec:?}"
    );
    assert_eq!(
        time_alarms_exec.attributes,
        [("_contract_address", test_case.address_book.time_alarms())]
    );
}

fn expect_balances<Lpn, ProtocolsRegistry, Reserve, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: TestCase<ProtocolsRegistry, Addr, Addr, Reserve, Leaser, Lpp, Oracle, TimeAlarms>,
    init_treasury_native_balance: Coin<Nls>,
    total_native_profit: Coin<Nls>,
    init_treasury_lpn_balance: Coin<Lpn>,
) where
    Lpn: CurrencyDef,
{
    assert_eq!(
        bank::balance(test_case.address_book.treasury(), test_case.app.query()).unwrap(),
        init_treasury_native_balance + total_native_profit,
    );

    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.profit(), test_case.app.query()).unwrap(),
        Zero::ZERO,
    );

    assert_eq!(
        bank::balance::<Lpn>(test_case.address_book.treasury(), test_case.app.query()).unwrap(),
        init_treasury_lpn_balance,
    );
}

fn on_time_alarm_do_transfers<Lpn>(
    native_profit: Coin<Nls>,
    lpn_profit: Option<(Coin<Lpn>, Coin<Nls>)>,
) where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    let mut test_case = test_case_with::<Lpn>(
        2,
        Some(&[
            common::cwcoin_from_amount::<Lpn>(1_000_000_000),
            common::cwcoin_dex::<Lpn>(1_000_000_000),
            common::cwcoin_from_amount::<Nls>(1_000_000_000),
            common::cwcoin_dex::<Nls>(1_000_000_000),
        ]),
    );

    let InitTreasuryBalancesResult {
        native: init_treasury_native_balance,
        lpn: init_treasury_lpn_balance,
    }: InitTreasuryBalancesResult<Lpn> = init_treasury_balances(&test_case);

    if !native_profit.is_zero() {
        //send native tokens to the profit contract
        test_case.send_funds_from_admin(
            test_case.address_book.profit().clone(),
            &[common::cwcoin(native_profit)],
        );
    }

    let lpn_profit = if let Some((lpn_profit_swap_in, lpn_profit_swap_out)) = lpn_profit {
        let lpn_profit_swap_in_cw = common::cwcoin::<Lpn>(lpn_profit_swap_in);

        //send LPN tokens to the profit contract
        test_case.send_funds_from_admin(
            test_case.address_book.profit().clone(),
            slice::from_ref(&lpn_profit_swap_in_cw),
        );

        assert_eq!(
            bank::balance(test_case.address_book.profit(), test_case.app.query()).unwrap(),
            lpn_profit_swap_in,
        );

        Some((
            lpn_profit_swap_in,
            lpn_profit_swap_in_cw,
            lpn_profit_swap_out,
        ))
    } else {
        assert!(!native_profit.is_zero());

        None
    };

    let alarm_result = send_alarm_and_maybe_swap(&mut test_case, lpn_profit);

    let total_native_profit = total_native_profit(native_profit, alarm_result.lpn_profit_swap_out);

    expect_transfer_events(&test_case, alarm_result, total_native_profit);

    expect_balances(
        test_case,
        init_treasury_native_balance,
        total_native_profit,
        init_treasury_lpn_balance,
    );
}

#[test]
fn on_alarm_native_only_transfer() {
    let native_profit = common::coin::<Nls>(1000);

    on_time_alarm_do_transfers::<Lpn>(native_profit, None);
}

#[test]
fn on_alarm_foreign_only_transfer() {
    let lpn_profit = common::lpn_coin(500);
    let swapped_lpn_profit = common::coin::<Nls>(250);

    on_time_alarm_do_transfers::<Lpn>(Zero::ZERO, Some((lpn_profit, swapped_lpn_profit)));
}

#[test]
fn on_alarm_native_and_foreign_transfer() {
    let native_profit = common::coin::<Nls>(1000);
    let lpn_profit = common::lpn_coin(500);
    let swapped_lpn_profit = common::coin::<Nls>(250);

    on_time_alarm_do_transfers::<Lpn>(native_profit, Some((lpn_profit, swapped_lpn_profit)));
}

#[test]
fn integration_with_time_alarms() {
    const CADENCE_HOURS: CadenceHours = 2;

    let mut test_case = test_case_with::<Lpn>(CADENCE_HOURS, None);

    test_case
        .app
        .time_shift(Duration::from_hours(CADENCE_HOURS) + Duration::from_secs(1));

    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[common::cwcoin_from_amount::<Nls>(500)],
    );

    assert!(
        !test_case
            .app
            .query()
            .query_balance(test_case.address_book.profit().clone(), Nls::bank())
            .unwrap()
            .amount
            .is_zero()
    );

    let resp = test_case
        .app
        .execute(
            testing::user(ADMIN),
            test_case.address_book.time_alarms().clone(),
            &timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 10 },
            &[],
        )
        .unwrap()
        .unwrap_response();

    assert_eq!(
        cosmwasm_std::from_json(resp.data.clone().unwrap()),
        Ok(DispatchAlarmsResponse(1))
    );

    resp.assert_event(&Event::new("wasm-time-alarm").add_attribute("delivered", "success"));

    assert_eq!(
        test_case
            .app
            .query()
            .query_balance(test_case.address_book.profit().clone(), Nls::bank())
            .unwrap()
            .amount
            .u128(),
        ::profit::profit::Profit::IBC_FEE_RESERVE.into(),
    );
}
