    use std::collections::HashSet;
use std::str::FromStr;

    use crate::contract::{instantiate, query, execute};
    use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg, ConfigResponse};

    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Decimal256, Addr};
    use marketprice::feed::Observation;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        let msg = InstantiateMsg { base_asset: "token".to_string(), price_feed_period: 60, feeders_percentage_needed: 50 };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("token".to_string() , value.base_asset);
        assert_eq!("creator".to_string() , value.owner.to_string());
    }

    #[test]
    fn register_feeder() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { base_asset: "token".to_string(), price_feed_period: 60, feeders_percentage_needed: 50 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("creator", &coins(2, "token"));

        let msg = ExecuteMsg::RegisterFeeder {feeder_address: "invalid".to_string()};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should add new address to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(1, resp.len());

        // should return error that address is already added
        let info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::RegisterFeeder {feeder_address: "invalid".to_string()};
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_ok())
    }
    
    #[test]
    fn feed_prices_unknown_feeder() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { base_asset: "token".to_string(), price_feed_period: 60, feeders_percentage_needed: 50 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::FeedPrice {
            base: "OSM".to_string(),
            prices: vec![
                ("mAAPL".to_string(), Decimal256::from_str("1.2").unwrap()),
                ("mGOGL".to_string(), Decimal256::from_str("2.2").unwrap()),
            ],
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
}

    #[test]
    fn feed_prices() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
       

        let msg = InstantiateMsg { base_asset: "token".to_string(), price_feed_period: 60, feeders_percentage_needed: 50 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::RegisterFeeder {feeder_address: "creator".to_string()};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::FeedPrice {
            base: "OSM".to_string(),
            prices: vec![
                ("mAAPL".to_string(), Decimal256::from_str("1.2").unwrap()),
                ("mGOGL".to_string(), Decimal256::from_str("2.2").unwrap()),
            ],
        };
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();      


        // should add new address to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), 
            QueryMsg::Price { base: "OSM".to_string(), quote: "mGOGL".to_string() }).unwrap();
        let value: Observation = from_binary(&res).unwrap();
        assert_eq!(Decimal256::from_str("2.2").unwrap(), value.price());

    }
