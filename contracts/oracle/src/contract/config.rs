use cosmwasm_std::{Deps, DepsMut, MessageInfo, Response};
use finance::{duration::Duration, percent::Percent};

use crate::{msg::ConfigResponse, state::Config, ContractError};

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(ConfigResponse {
        base_asset: config.base_asset,
        owner: config.owner,
        price_feed_period: config.price_feed_period,
        feeders_percentage_needed: config.feeders_percentage_needed,
    })
}

pub fn try_configure(
    deps: DepsMut,
    info: MessageInfo,
    price_feed_period: Duration,
    feeders_percentage_needed: Percent,
) -> Result<Response, ContractError> {
    Config::update(
        deps.storage,
        price_feed_period,
        feeders_percentage_needed,
        info.sender,
    )?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_env, mock_info},
    };
    use currency::{lpn::Usdc, native::Nls, test::TestCurrencyA};
    use finance::{currency::Currency, duration::Duration, percent::Percent};

    use crate::{
        contract::{execute, query},
        msg::{ConfigResponse, ExecuteMsg, QueryMsg},
        state::supported_pairs::ResolutionPath,
        tests::{dummy_default_instantiate_msg, dummy_instantiate_msg, setup_test},
        ContractError,
    };

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn configure_unauthorized() {
        let msg = dummy_instantiate_msg(
            Usdc::SYMBOL.to_string(),
            60,
            Percent::from_percent(50),
            vec![vec![Nls::SYMBOL.to_string(), Usdc::SYMBOL.to_string()]],
            "timealarms".to_string(),
        );
        let (mut deps, _) = setup_test(msg);

        let unauth_info = mock_info("anyone", &coins(2, Nls::SYMBOL));
        let msg = ExecuteMsg::Config {
            price_feed_period_secs: 15,
            feeders_percentage_needed: Percent::from_percent(12),
        };
        let _res = execute(deps.as_mut(), mock_env(), unauth_info, msg).unwrap();
    }

    #[test]
    fn configure() {
        let msg = dummy_instantiate_msg(
            Usdc::SYMBOL.to_string(),
            60,
            Percent::from_percent(50),
            vec![vec![Nls::SYMBOL.to_string(), Usdc::SYMBOL.to_string()]],
            "timealarms".to_string(),
        );
        let (mut deps, info) = setup_test(msg);

        let msg = ExecuteMsg::Config {
            price_feed_period_secs: 33,
            feeders_percentage_needed: Percent::from_percent(44),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should now be 12
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(Percent::from_percent(44), value.feeders_percentage_needed);
        assert_eq!(Duration::from_secs(33), value.price_feed_period);
    }

    #[test]
    fn config_supported_pairs() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        let test_vec = vec![
            vec![TestCurrencyA::SYMBOL.to_string(), Usdc::SYMBOL.to_string()],
            vec![Nls::SYMBOL.to_string(), Usdc::SYMBOL.to_string()],
        ];

        let msg = ExecuteMsg::CurrencyPaths {
            paths: test_vec.clone(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_ok());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::SupportedDenomPairs {}).unwrap();
        let value: Vec<ResolutionPath> = from_binary(&res).unwrap();
        assert_eq!(test_vec, value);
    }

    #[test]
    fn invalid_supported_pairs() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        let msg = ExecuteMsg::CurrencyPaths {
            paths: vec![
                vec![TestCurrencyA::SYMBOL.to_string(), Usdc::SYMBOL.to_string()],
                vec![Nls::SYMBOL.to_string(), Nls::SYMBOL.to_string()],
            ],
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            ContractError::InvalidResolutionPath(vec![
                Nls::SYMBOL.to_string(),
                Nls::SYMBOL.to_string()
            ]),
            err
        );
    }
}
