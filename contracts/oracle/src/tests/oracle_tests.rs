use std::collections::HashSet;

use crate::contract::{execute, query};
use crate::msg::{ConfigResponse, ExecuteMsg, PricesResponse, QueryMsg};
use crate::tests::common::{
    dummy_default_instantiate_msg, dummy_instantiate_msg, setup_test, CREATOR,
};
use crate::ContractError;

use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{coins, from_binary, Addr};
use marketprice::storage::Price;

use super::common::dummy_feed_prices_msg;

#[test]
fn proper_initialization() {
    let msg = dummy_instantiate_msg(
        "token".to_string(),
        60,
        50,
        vec![("unolus".to_string(), "uosmo".to_string())],
        "timealarms".to_string(),
    );
    let (deps, _) = setup_test(msg);

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(CREATOR.to_string(), value.owner.to_string());
    assert_eq!("token".to_string(), value.base_asset);
    assert_eq!(60, value.price_feed_period_secs);
    assert_eq!(50, value.feeders_percentage_needed);

    let res = query(deps.as_ref(), mock_env(), QueryMsg::SupportedDenomPairs {}).unwrap();
    let value: Vec<(String, String)> = from_binary(&res).unwrap();
    assert_eq!("unolus".to_string(), value.get(0).unwrap().0);
    assert_eq!("uosmo".to_string(), value.get(0).unwrap().1);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn configure_unauthorized() {
    let msg = dummy_instantiate_msg(
        "token".to_string(),
        60,
        50,
        vec![("unolus".to_string(), "uosmo".to_string())],
        "timealarms".to_string(),
    );
    let (mut deps, _) = setup_test(msg);

    let unauth_info = mock_info("anyone", &coins(2, "token"));
    let msg = ExecuteMsg::Config {
        price_feed_period_secs: 15,
        feeders_percentage_needed: 12,
    };
    let _res = execute(deps.as_mut(), mock_env(), unauth_info, msg).unwrap();
}

#[test]
fn configure() {
    let msg = dummy_instantiate_msg(
        "token".to_string(),
        60,
        50,
        vec![("unolus".to_string(), "uosmo".to_string())],
        "timealarms".to_string(),
    );
    let (mut deps, info) = setup_test(msg);

    let msg = ExecuteMsg::Config {
        price_feed_period_secs: 33,
        feeders_percentage_needed: 44,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // should now be 12
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(44, value.feeders_percentage_needed);
    assert_eq!(33, value.price_feed_period_secs);
}

#[test]
fn register_feeder() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    // register new feeder address
    let msg = ExecuteMsg::RegisterFeeder {
        feeder_address: "addr0000".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // check if the new address is added to FEEDERS Item
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
    let resp: HashSet<Addr> = from_binary(&res).unwrap();
    assert_eq!(2, resp.len());
    assert!(resp.contains(&Addr::unchecked("addr0000")));

    // should not add the same address twice
    let msg = ExecuteMsg::RegisterFeeder {
        feeder_address: "addr0000".to_string(),
    };
    let _ = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    // validate that the address in not added twice
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
    let resp: HashSet<Addr> = from_binary(&res).unwrap();
    assert_eq!(2, resp.len());

    // register new feeder address
    let msg = ExecuteMsg::RegisterFeeder {
        feeder_address: "addr0001".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    // check if the new address is added to FEEDERS Item
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
    let resp: HashSet<Addr> = from_binary(&res).unwrap();
    assert_eq!(3, resp.len());
    assert!(resp.contains(&Addr::unchecked("addr0000")));
    assert!(resp.contains(&Addr::unchecked("addr0001")));
}

#[test]
fn feed_prices_unknown_feeder() {
    let (mut deps, _) = setup_test(dummy_default_instantiate_msg());

    let msg = dummy_feed_prices_msg();
    let info = mock_info("test", &[]);

    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(ContractError::UnknownFeeder {}, err)
}

#[test]
fn feed_prices() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let msg = dummy_feed_prices_msg();
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // check if price is realy pushed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PriceFor {
            denoms: HashSet::from(["A".to_string()]),
        },
    )
    .unwrap();
    let value: PricesResponse = from_binary(&res).unwrap();
    assert_eq!(
        Price::new("A", 10, "B", 12),
        value.prices.first().unwrap().to_owned()
    );
}

#[test]
#[should_panic(expected = "Unsupported denom")]
fn query_prices_unsuppoted_denom() {
    let (deps, _) = setup_test(dummy_default_instantiate_msg());

    // query for unsupported denom should fail
    query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PriceFor {
            denoms: HashSet::from(["dummy".to_string()]),
        },
    )
    .unwrap();
}

#[test]
fn feed_prices_unsupported_pairs() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let prices_map = vec![Price::new("X", 10, "C", 12), Price::new("X", 10, "D", 22)];

    let msg = ExecuteMsg::FeedPrices { prices: prices_map };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(ContractError::UnsupportedDenomPairs {}, err);
}

#[test]
fn config_supported_pairs() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let test_vec = vec![
        ("denom1".to_string(), "denom2".to_string()),
        ("denom3".to_string(), "denom4".to_string()),
    ];

    let msg = ExecuteMsg::SupportedDenomPairs {
        pairs: test_vec.clone(),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_ok());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::SupportedDenomPairs {}).unwrap();
    let value: Vec<(String, String)> = from_binary(&res).unwrap();
    assert_eq!(test_vec, value);
}

#[test]
fn invalid_supported_pairs() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let msg = ExecuteMsg::SupportedDenomPairs {
        pairs: vec![
            ("denom1".to_string(), "denom2".to_string()),
            ("denom3".to_string(), "denom3".to_string()),
        ],
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        ContractError::InvalidDenomPair(("denom3".to_string(), "denom3".to_string())),
        err
    );
}
