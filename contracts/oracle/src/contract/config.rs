use cosmwasm_std::{Deps, DepsMut, MessageInfo, Response, Storage};
use finance::currency::SymbolOwned;

use crate::{msg::ConfigResponse, state::config::Config, ContractError};

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(ConfigResponse {
        base_asset: config.base_asset,
        owner: config.owner,
        price_feed_period_secs: config.price_feed_period_secs,
        feeders_percentage_needed: config.feeders_percentage_needed,
    })
}

pub fn try_configure(
    deps: DepsMut,
    info: MessageInfo,
    price_feed_period_secs: u32,
    feeders_percentage_needed: u8,
) -> Result<Response, ContractError> {
    Config::update(
        deps.storage,
        price_feed_period_secs,
        feeders_percentage_needed,
        info.sender,
    )?;

    Ok(Response::new())
}

pub fn try_configure_supported_pairs(
    storage: &mut dyn Storage,
    info: MessageInfo,
    pairs: Vec<(SymbolOwned, SymbolOwned)>,
) -> Result<Response, ContractError> {
    Config::update_supported_pairs(storage, pairs, info.sender)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_env, mock_info},
    };

    use crate::{
        contract::{execute, query},
        msg::{ConfigResponse, ExecuteMsg, QueryMsg},
        tests::{dummy_default_instantiate_msg, dummy_instantiate_msg, setup_test},
        ContractError,
    };

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
            ContractError::InvalidDenomPair("denom3".to_string(), "denom3".to_string()),
            err
        );
    }
}
