use currencies::test::{PaymentC3, PaymentC4, PaymentC5, PaymentC7, StableC1};
use currency::Currency;
use finance::{coin::Coin, price, price::dto::PriceDTO};
use marketprice::SpotPrice;
use platform::{contract, tests};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        from_json,
        testing::{mock_env, mock_info, MockQuerier},
        Event,
    },
};

use crate::{
    api::{Alarm, AlarmsCount, DispatchAlarmsResponse, ExecuteMsg, QueryMsg},
    contract::{execute, query},
    tests::{dummy_default_instantiate_msg, setup_test},
    ContractError,
};

use super::dummy_feed_prices_msg;

#[test]
fn feed_prices_unknown_feeder() {
    let (mut deps, _) = setup_test(dummy_default_instantiate_msg());

    let msg = dummy_feed_prices_msg();
    let info = mock_info("test", &[]);

    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(ContractError::UnknownFeeder {}, err)
}

#[test]
fn feed_direct_price() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let expected_price = PriceDTO::try_from(
        price::total_of(Coin::<PaymentC4>::new(10)).is(Coin::<StableC1>::new(120)),
    )
    .unwrap();

    // Feed direct price PaymentC4/OracleBaseAsset
    let msg = ExecuteMsg::FeedPrices {
        prices: vec![expected_price.clone()],
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // query price for PaymentC5
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            currency: PaymentC4::TICKER.to_string(),
        },
    )
    .unwrap();
    let value: SpotPrice = from_json(res).unwrap();
    assert_eq!(expected_price, value);
}

#[test]
fn feed_indirect_price() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let price_a_to_b =
        PriceDTO::try_from(
            price::total_of(Coin::<PaymentC5>::new(10)).is(Coin::<PaymentC3>::new(120)),
        )
        .unwrap();
    let price_b_to_c =
        PriceDTO::try_from(
            price::total_of(Coin::<PaymentC3>::new(10)).is(Coin::<PaymentC7>::new(5)),
        )
        .unwrap();
    let price_c_to_usdc = PriceDTO::try_from(
        price::total_of(Coin::<PaymentC7>::new(10)).is(Coin::<StableC1>::new(5)),
    )
    .unwrap();

    // Feed indirect price from PaymentC5 to OracleBaseAsset
    let msg = ExecuteMsg::FeedPrices {
        prices: vec![price_a_to_b, price_b_to_c, price_c_to_usdc],
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // query price for PaymentC5
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            currency: PaymentC5::TICKER.to_string(),
        },
    )
    .unwrap();

    let expected_price = SpotPrice::try_from(
        price::total_of(Coin::<PaymentC5>::new(1)).is(Coin::<StableC1>::new(3)),
    )
    .unwrap();
    let value: SpotPrice = from_json(res).unwrap();
    assert_eq!(expected_price, value)
}

#[test]
#[should_panic(expected = "UnsupportedCurrency")]
fn query_prices_unsupported_denom() {
    let (deps, _) = setup_test(dummy_default_instantiate_msg());

    // query for unsupported denom should fail
    query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            currency: "dummy".to_string(),
        },
    )
    .unwrap();
}

#[test]
fn feed_prices_unsupported_pairs() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let prices = vec![
        PriceDTO::try_from(
            price::total_of(Coin::<PaymentC3>::new(10)).is(Coin::<PaymentC4>::new(12)),
        )
        .unwrap(),
        PriceDTO::try_from(
            price::total_of(Coin::<PaymentC3>::new(10)).is(Coin::<PaymentC7>::new(22)),
        )
        .unwrap(),
    ];

    let msg = ExecuteMsg::FeedPrices { prices };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(ContractError::UnsupportedDenomPairs {}, err);
}

#[test]
fn deliver_alarm() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());
    setup_receiver(&mut deps.querier);

    let current_price =
        price::total_of(Coin::<PaymentC7>::new(10)).is(Coin::<StableC1>::new(23451));
    let feed_price_msg = ExecuteMsg::FeedPrices {
        prices: vec![current_price.try_into().unwrap()],
    };
    let feed_resp = execute(deps.as_mut(), mock_env(), info.clone(), feed_price_msg);
    assert_eq!(Ok(CwResponse::default()), feed_resp);

    {
        let alarm_below_price =
            price::total_of(Coin::<PaymentC7>::new(10)).is(Coin::<StableC1>::new(23450));
        let add_alarm_msg = ExecuteMsg::AddPriceAlarm {
            alarm: Alarm::new(alarm_below_price, None),
        };
        let add_alarm_resp = execute(deps.as_mut(), mock_env(), info.clone(), add_alarm_msg);
        assert_eq!(Ok(CwResponse::default()), add_alarm_resp);

        let dispatch_alarms_msg = ExecuteMsg::DispatchAlarms { max_count: 10 };
        let dispatch_alarms_resp =
            execute(deps.as_mut(), mock_env(), info.clone(), dispatch_alarms_msg).unwrap();
        assert!(!any_error(&dispatch_alarms_resp));
        assert_eq!(sent_alarms(&dispatch_alarms_resp), Some(0));
        assert_eq!(0, dispatch_alarms_resp.messages.len());
    }
    {
        let alarm_below_price =
            price::total_of(Coin::<PaymentC7>::new(10)).is(Coin::<StableC1>::new(23452));
        let add_alarm_msg = ExecuteMsg::AddPriceAlarm {
            alarm: Alarm::new(alarm_below_price, None),
        };
        let add_alarm_resp = execute(deps.as_mut(), mock_env(), info.clone(), add_alarm_msg);
        assert_eq!(Ok(CwResponse::default()), add_alarm_resp);

        let dispatch_alarms_msg = ExecuteMsg::DispatchAlarms { max_count: 10 };
        let receiver = info.sender.clone();
        let dispatch_alarms_resp =
            execute(deps.as_mut(), mock_env(), info, dispatch_alarms_msg).unwrap();
        assert!(!any_error(&dispatch_alarms_resp));
        tests::assert_event(
            &dispatch_alarms_resp.events,
            &Event::new("pricealarm").add_attribute("receiver", receiver),
        );
        assert_eq!(sent_alarms(&dispatch_alarms_resp), Some(1));
        assert_eq!(1, dispatch_alarms_resp.messages.len());
    }
}

fn setup_receiver(querier: &mut MockQuerier) {
    querier.update_wasm(contract::testing::valid_contract_handler);
}

fn sent_alarms(resp: &CwResponse) -> Option<AlarmsCount> {
    tests::parse_resp::<DispatchAlarmsResponse>(&resp.data).map(|resp| resp.0)
}

fn any_error(resp: &CwResponse) -> bool {
    tests::any_error(&resp.events)
}
