use std::slice;

use currencies::test::StableC1;
use currency::Currency;
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    zero::Zero,
};
use platform::bank;
use profit::msg::{ConfigResponse, ExecuteMsg, QueryMsg};
use sdk::{
    cosmwasm_std::{from_json, Addr, Event},
    cw_multi_test::AppResponse,
};
use timealarms::msg::DispatchAlarmsResponse;

use crate::common::{
    self, cwcoin, cwcoin_dex, ibc,
    swap::DexDenom,
    test_case::{
        builder::BlankBuilder as TestCaseBuilder, response::ResponseWithInterChainMsgs, TestCase,
    },
    CwCoin, Native, ADMIN, USER,
};

#[test]
fn update_config() {
    type Lpn = StableC1;

    const INITIAL_CACDENCE_HOURS: u16 = 2;
    const UPDATED_CACDENCE_HOURS: u16 = INITIAL_CACDENCE_HOURS + 1;

    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(INITIAL_CACDENCE_HOURS)
        .into_generic();

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
            Addr::unchecked(ADMIN),
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
    type Lpn = StableC1;

    const INITIAL_CACDENCE_HOURS: u16 = 2;
    const UPDATED_CACDENCE_HOURS: u16 = INITIAL_CACDENCE_HOURS + 1;

    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(INITIAL_CACDENCE_HOURS)
        .into_generic();

    assert!(test_case
        .app
        .execute(
            Addr::unchecked(USER),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::Config {
                cadence_hours: UPDATED_CACDENCE_HOURS
            },
            &[],
        )
        .unwrap_err()
        .root_cause()
        .to_string()
        .contains("Unauthorized"));
}

#[test]
fn on_alarm_from_unknown() {
    type Lpn = StableC1;
    let user_addr: Addr = Addr::unchecked(USER);

    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(2)
        .into_generic();

    test_case.send_funds_from_admin(user_addr.clone(), &[cwcoin::<Lpn, _>(500)]);

    let treasury_balance = test_case
        .app
        .query()
        .query_all_balances(test_case.address_book.treasury().clone())
        .unwrap();

    _ = test_case
        .app
        .execute(
            user_addr,
            test_case.address_book.profit().clone(),
            &profit::msg::ExecuteMsg::TimeAlarm {},
            &[cwcoin::<Lpn, _>(40)],
        )
        .unwrap_err();

    //assert that no transfer is made to treasury
    assert_eq!(
        treasury_balance,
        test_case
            .app
            .query()
            .query_all_balances(test_case.address_book.treasury().clone())
            .unwrap()
    );
}

#[test]
fn on_alarm_zero_balance() {
    type Lpn = StableC1;
    let time_oracle_addr = Addr::unchecked("time");

    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(2)
        .into_generic();

    test_case.send_funds_from_admin(time_oracle_addr, &[cwcoin::<Lpn, _>(500)]);

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

struct SendAlarmAndMaybeSwapResult {
    response: AppResponse,
    lpn_profit_swap_out: Coin<Native>,
    has_swap: bool,
}

fn send_alarm_and_maybe_swap<Lpn, ProtocolsRegistry, Dispatcher, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<ProtocolsRegistry, Dispatcher, Addr, Addr, Leaser, Lpp, Oracle, Addr>,
    lpn_profit: Option<(Coin<Lpn>, CwCoin, Coin<Native>)>,
) -> SendAlarmAndMaybeSwapResult
where
    Lpn: Currency,
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
                .add_attribute("_contract_addr", test_case.address_book.profit())]
        );

        let mut response: ResponseWithInterChainMsgs<'_, ()> = ibc::do_transfer(
            &mut test_case.app,
            test_case.address_book.profit().clone(),
            test_case.address_book.profit_ica().clone(),
            false,
            &transfer_amount,
        )
        .ignore_response();

        let requests = common::swap::expect_swap(
            &mut response,
            TestCase::DEX_CONNECTION_ID,
            TestCase::PROFIT_ICA_ID,
        );

        () = response.unwrap_response();

        let mut response: ResponseWithInterChainMsgs<'_, ()> = common::swap::do_swap(
            &mut test_case.app,
            test_case.address_book.profit().clone(),
            test_case.address_book.profit_ica().clone(),
            requests.into_iter(),
            |amount: Amount, from_denom: DexDenom<'_>, to_denom: DexDenom<'_>| {
                assert_eq!(amount, lpn_profit_swap_in.into());
                assert_eq!(from_denom, Lpn::DEX_SYMBOL);
                assert_eq!(to_denom, Native::DEX_SYMBOL);

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

fn on_time_alarm_do_transfers<Lpn>(
    native_profit: Coin<Native>,
    lpn_profit: Option<(Coin<Lpn>, Coin<Native>)>,
) where
    Lpn: Currency,
{
    let mut test_case = TestCaseBuilder::<Lpn>::with_reserve(&[
        cwcoin::<Lpn, _>(1_000_000_000),
        cwcoin_dex::<Lpn, _>(1_000_000_000),
        cwcoin::<Native, _>(1_000_000_000),
        cwcoin_dex::<Native, _>(1_000_000_000),
    ])
    .init_treasury_without_dispatcher()
    .init_time_alarms()
    .init_oracle(None)
    .init_profit(2)
    .into_generic();

    let init_treasury_native_balance: Coin<Native> = bank::balance(
        &test_case.address_book.treasury().clone(),
        test_case.app.query(),
    )
    .unwrap();

    let init_treasury_lpn_balance: Coin<Lpn> = bank::balance(
        &test_case.address_book.treasury().clone(),
        test_case.app.query(),
    )
    .unwrap();

    if !native_profit.is_zero() {
        //send native tokens to the profit contract
        test_case.send_funds_from_admin(
            test_case.address_book.profit().clone(),
            &[cwcoin(native_profit)],
        );
    }

    let lpn_profit = if let Some((lpn_profit_swap_in, lpn_profit_swap_out)) = lpn_profit {
        let lpn_profit_swap_in_cw = cwcoin::<Lpn, _>(lpn_profit_swap_in);

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

    let SendAlarmAndMaybeSwapResult {
        mut response,
        lpn_profit_swap_out,
        has_swap,
    } = send_alarm_and_maybe_swap(&mut test_case, lpn_profit);

    if has_swap {
        let sudo = response.events.remove(0);
        assert_eq!(sudo.ty.as_str(), "sudo");
        assert_eq!(
            sudo.attributes,
            [("_contract_addr", test_case.address_book.profit().as_str())]
        );
    }

    let total_native_profit =
        native_profit + lpn_profit_swap_out - ::profit::profit::Profit::IBC_FEE_RESERVE;

    assert_eq!(response.events.len(), 4, "{:?}", response.events);

    let profit_exec = &response.events[0];
    assert_eq!(profit_exec.ty.as_str(), "execute");
    assert_eq!(
        profit_exec.attributes,
        [("_contract_addr", test_case.address_book.profit().as_str())]
    );

    let tr_profit = &response.events[1];
    assert_eq!(tr_profit.ty.as_str(), "wasm-tr-profit");
    assert_eq!(
        tr_profit.attributes,
        [
            ("_contract_addr", test_case.address_book.profit().as_str()),
            ("height", &test_case.app.block_info().height.to_string()),
            ("at", &test_case.app.block_info().time.nanos().to_string()),
            ("idx", "0"),
            (
                "profit-amount-amount",
                &Amount::from(total_native_profit).to_string()
            ),
            ("profit-amount-symbol", Native::TICKER)
        ]
    );

    let [transfer, time_alarms_exec] = if has_swap {
        [&response.events[3], &response.events[2]]
    } else {
        [&response.events[2], &response.events[3]]
    };

    assert_eq!(transfer.ty.as_str(), "transfer", "{transfer:?}");
    assert_eq!(
        transfer.attributes,
        [
            ("recipient", test_case.address_book.treasury().as_str()),
            ("sender", test_case.address_book.profit().as_str()),
            (
                "amount",
                &format!(
                    "{}{}",
                    Amount::from(total_native_profit),
                    Native::BANK_SYMBOL
                )
            )
        ]
    );

    assert_eq!(
        time_alarms_exec.ty.as_str(),
        "execute",
        "{time_alarms_exec:?}"
    );
    assert_eq!(
        time_alarms_exec.attributes,
        [("_contract_addr", test_case.address_book.time_alarms())]
    );

    assert_eq!(
        bank::balance::<Native>(test_case.address_book.treasury(), test_case.app.query()).unwrap(),
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

#[test]
fn on_alarm_native_only_transfer() {
    type Lpn = StableC1;

    let native_profit = 1000.into();

    on_time_alarm_do_transfers::<Lpn>(native_profit, None);
}

#[test]
fn on_alarm_foreign_only_transfer() {
    type Lpn = StableC1;

    let lpn_profit = 500.into();
    let swapped_lpn_profit = 250.into();

    on_time_alarm_do_transfers::<Lpn>(Zero::ZERO, Some((lpn_profit, swapped_lpn_profit)));
}

#[test]
fn on_alarm_native_and_foreign_transfer() {
    type Lpn = StableC1;

    let native_profit = 1000.into();
    let lpn_profit = 500.into();
    let swapped_lpn_profit = 250.into();

    on_time_alarm_do_transfers::<Lpn>(native_profit, Some((lpn_profit, swapped_lpn_profit)));
}

#[test]
fn integration_with_time_alarms() {
    type Lpn = StableC1;
    const CADENCE_HOURS: u16 = 2;

    let mut test_case = TestCaseBuilder::<Lpn>::new()
        .init_treasury_without_dispatcher()
        .init_time_alarms()
        .init_oracle(None)
        .init_profit(CADENCE_HOURS)
        .into_generic();

    test_case
        .app
        .time_shift(Duration::from_hours(CADENCE_HOURS) + Duration::from_secs(1));

    test_case.send_funds_from_admin(
        test_case.address_book.profit().clone(),
        &[cwcoin::<Native, _>(500)],
    );

    assert!(!test_case
        .app
        .query()
        .query_balance(test_case.address_book.profit().clone(), Native::BANK_SYMBOL)
        .unwrap()
        .amount
        .is_zero());

    let resp = test_case
        .app
        .execute(
            Addr::unchecked(ADMIN),
            test_case.address_book.time_alarms().clone(),
            &timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 10 },
            &[],
        )
        .unwrap()
        .unwrap_response();

    assert_eq!(
        from_json(resp.data.clone().unwrap()),
        Ok(DispatchAlarmsResponse(1))
    );

    resp.assert_event(&Event::new("wasm-time-alarm").add_attribute("delivered", "success"));

    assert_eq!(
        test_case
            .app
            .query()
            .query_balance(test_case.address_book.profit().clone(), Native::BANK_SYMBOL)
            .unwrap()
            .amount
            .u128(),
        ::profit::profit::Profit::IBC_FEE_RESERVE.into(),
    );
}
