use std::collections::HashMap;

use osmosis_std::types::osmosis::gamm::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse,
};

use finance::{coin::Amount, duration::Duration, price};
use ::lease::api::{ExecuteMsg, StateResponse};
use sdk::{
    cosmos_sdk_proto::{ibc::applications::transfer::v1::MsgTransfer, traits::TypeUrl as _},
    cosmwasm_std::{Addr, Binary, Event},
    cw_multi_test::AppResponse,
};

use crate::{
    common::{
        cwcoin,
        leaser::Instantiator as LeaserInstantiator,
        test_case::{
            response::{RemoteChain, ResponseWithInterChainMsgs},
            TestCase,
        },
        ADMIN,
    },
    lease,
};

use super::{LeaseCoin, LpnCoin, PaymentCoin, PaymentCurrency, DOWNPAYMENT};

fn liquidation_time_alarm(time_pass: Duration, liquidation_amount: Option<LeaseCoin>) {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = lease::create_payment_coin(DOWNPAYMENT);
    let lease_address = lease::open_lease(&mut test_case, downpayment, None);

    let StateResponse::Opened {
        amount: lease_amount,
        ..
    } = lease::state_query(&test_case, lease_address.as_ref()) else {
        unreachable!()
    };
    let lease_amount: LeaseCoin = lease_amount.try_into().unwrap();

    test_case.app.time_shift(time_pass);

    lease::feed_price(&mut test_case);

    let mut response: ResponseWithInterChainMsgs<'_, AppResponse> = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            lease_address.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    if liquidation_amount.is_some() {
        response.expect_submit_tx(
            TestCase::LEASER_CONNECTION_ID,
            "0",
            &[MsgSwapExactAmountIn::TYPE_URL],
        );
    }

    let liquidation_start_response: AppResponse = response.unwrap_response();

    let Some(liquidation_amount): Option<LeaseCoin> = liquidation_amount else {
        assert!(!liquidation_start_response.has_event(&Event::new("wasm-ls-liquidation-start")));

        return;
    };

    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            Addr::unchecked(ADMIN),
            &[cwcoin(liquidation_amount)],
        )
        .unwrap();

    let liquidated_in_lpn: LpnCoin = price::total(liquidation_amount, lease::price_lpn_of());

    test_case.send_funds_from_admin(Addr::unchecked("ica0"), &[cwcoin(liquidated_in_lpn)]);

    let mut response: ResponseWithInterChainMsgs<'_, ()> = test_case
        .app
        .sudo(
            lease_address.clone(),
            &sdk::neutron_sdk::sudo::msg::SudoMsg::Response {
                request: sdk::neutron_sdk::sudo::msg::RequestPacket {
                    sequence: None,
                    source_port: None,
                    source_channel: None,
                    destination_port: None,
                    destination_channel: None,
                    data: None,
                    timeout_height: None,
                    timeout_timestamp: None,
                },
                data: Binary(platform::trx::encode_msg_responses(
                    [platform::trx::encode_msg_response(
                        MsgSwapExactAmountInResponse {
                            token_out_amount: Amount::from(liquidated_in_lpn).to_string(),
                        },
                        MsgSwapExactAmountIn::TYPE_URL,
                    )]
                    .into_iter(),
                )),
            },
        )
        .unwrap()
        .ignore_response();

    response.expect_submit_tx(
        TestCase::LEASER_CONNECTION_ID,
        "0",
        &[MsgTransfer::TYPE_URL],
    );

    () = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            lease_address.clone(),
            &[cwcoin(liquidated_in_lpn)],
        )
        .unwrap();

    () = test_case
        .app
        .sudo(
            lease_address.clone(),
            &sdk::neutron_sdk::sudo::msg::SudoMsg::Response {
                request: sdk::neutron_sdk::sudo::msg::RequestPacket {
                    sequence: None,
                    source_port: None,
                    source_channel: None,
                    destination_port: None,
                    destination_channel: None,
                    data: None,
                    timeout_height: None,
                    timeout_timestamp: None,
                },
                data: Binary::default(),
            },
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    assert_eq!(
        test_case
            .app
            .query()
            .query_all_balances(lease_address.clone())
            .unwrap(),
        &[],
    );

    let liquidation_attributes: HashMap<String, String> = liquidation_start_response
        .events
        .into_iter()
        .find(|event| event.ty == "wasm-ls-liquidation-start")
        .expect("No liquidation emitted!")
        .attributes
        .into_iter()
        .map(|attribute| (attribute.key, attribute.value))
        .collect();

    let query_result = lease::state_query(&test_case, lease_address.as_str());

    let liquidated_amount: LeaseCoin = liquidation_attributes["amount-amount"]
        .parse::<Amount>()
        .unwrap()
        .into();

    assert_eq!(liquidated_amount, liquidation_amount);

    if let StateResponse::Opened {
        amount,
        previous_margin_due,
        previous_interest_due,
        ..
    } = query_result
    {
        assert_eq!(
            LeaseCoin::try_from(amount).unwrap(),
            lease_amount - liquidated_amount
        );

        assert!(previous_margin_due.is_zero());

        assert!(previous_interest_due.is_zero());
    }
}

#[test]
fn liquidation_time_alarm_0() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD - Duration::from_nanos(1),
        None,
    );
}

#[test]
fn liquidation_time_alarm_1() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::GRACE_PERIOD
            - Duration::from_nanos(1),
        None,
    );
}

#[test]
fn liquidation_time_alarm_2() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::GRACE_PERIOD,
        Some(LeaseCoin::new(45792563600)),
    );
}
