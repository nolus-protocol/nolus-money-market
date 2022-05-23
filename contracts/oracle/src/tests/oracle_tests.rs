use std::collections::HashSet;
use std::str::FromStr;

use crate::contract::{execute, query};
use crate::msg::{ConfigResponse, ExecuteMsg, PriceResponse, QueryMsg};
use crate::tests::common::{
    dummy_default_instantiate_msg, dummy_instantiate_msg, setup_test, CREATOR,
};
use crate::ContractError;

use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr, Decimal, StdError};
use marketprice::feed::{DenomToPrice, Price};

use super::common::dummy_feed_prices_msg;

#[test]
fn proper_initialization() {
    let msg = dummy_instantiate_msg(
        "token".to_string(),
        60,
        50,
        vec![("unolus".to_string(), "uosmo".to_string())],
    );
    let (deps, _) = setup_test(msg);

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(CREATOR.to_string(), value.owner.to_string());
    assert_eq!("token".to_string(), value.base_asset);
    assert_eq!(60, value.price_feed_period);
    assert_eq!(50, value.feeders_percentage_needed);

    let res = query(deps.as_ref(), mock_env(), QueryMsg::SupportedDenomPairs {}).unwrap();
    let value: Vec<(String, String)> = from_binary(&res).unwrap();
    assert_eq!("unolus".to_string(), value.get(0).unwrap().0);
    assert_eq!("uosmo".to_string(), value.get(0).unwrap().1);
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
            denoms: vec!["A".to_string()],
        },
    )
    .unwrap();
    let value: PriceResponse = from_binary(&res).unwrap();
    assert_eq!(
        DenomToPrice::new(
            "A".to_string(),
            Price::new(Decimal::from_str("1.2").unwrap(), "B".to_string())
        ),
        value.prices.first().unwrap().to_owned()
    );
}

#[test]
fn query_prices_unsuppoted_denom() {
    let (deps, _) = setup_test(dummy_default_instantiate_msg());

    // query for unsupported denom should fail
    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PriceFor {
            denoms: vec!["dummy".to_string()],
        },
    )
    .unwrap_err();
    assert_eq!(StdError::generic_err("Unsupported denom"), err);
}

#[test]
fn feed_prices_unsupported_pairs() {
    let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

    let prices_map = vec![marketprice::feed::Prices {
        base: "X".to_string(),
        values: vec![
            Price {
                denom: "c".to_string(),
                amount: Decimal::from_str("1.2").unwrap(),
            },
            Price {
                denom: "D".to_string(),
                amount: Decimal::from_str("2.2").unwrap(),
            },
        ],
    }];

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
