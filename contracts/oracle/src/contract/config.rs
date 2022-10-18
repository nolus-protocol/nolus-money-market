use finance::{duration::Duration, percent::Percent};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Deps, DepsMut, MessageInfo},
};

use crate::{msg::ConfigResponse, state::Config, ContractError};

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(ConfigResponse {
        base_asset: config.base_asset,
        owner: config.owner,
        price_feed_period: config.price_feed_period,
        expected_feeders: config.expected_feeders,
    })
}

pub fn try_configure(
    deps: DepsMut,
    info: MessageInfo,
    price_feed_period: u32,
    expected_feeders: Percent,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if expected_feeders == Percent::ZERO || expected_feeders > Percent::HUNDRED {
        return Err(ContractError::Configuration(
            "Percent of expected available feeders should be > 0 and <= 1000".to_string(),
        ));
    }
    if price_feed_period == 0 {
        return Err(ContractError::Configuration(
            "Price feed period can not be 0".to_string(),
        ));
    }
    Config::update(
        deps.storage,
        Duration::from_secs(price_feed_period),
        expected_feeders,
    )?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use currency::{lpn::Usdc, native::Nls, test::TestCurrencyA};
    use finance::{currency::Currency, duration::Duration, percent::Percent};
    use sdk::{
        cosmwasm_ext::Response,
        cosmwasm_std::{
            coins, from_binary,
            testing::{mock_env, mock_info},
            DepsMut, MessageInfo,
        },
    };

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
            Usdc::TICKER.to_string(),
            60,
            Percent::from_percent(50),
            vec![vec![Nls::TICKER.to_string(), Usdc::TICKER.to_string()]],
            "timealarms".to_string(),
        );
        let (mut deps, _) = setup_test(msg);

        let unauth_info = mock_info("anyone", &coins(2, Nls::TICKER));
        let msg = ExecuteMsg::Config {
            price_feed_period_secs: 15,
            expected_feeders: Percent::from_percent(12),
        };
        let _res = execute(deps.as_mut(), mock_env(), unauth_info, msg).unwrap();
    }

    #[test]
    fn configure() {
        let msg = dummy_instantiate_msg(
            Usdc::TICKER.to_string(),
            60,
            Percent::from_percent(50),
            vec![vec![Nls::TICKER.to_string(), Usdc::TICKER.to_string()]],
            "timealarms".to_string(),
        );
        let (mut deps, info) = setup_test(msg);

        let msg = ExecuteMsg::Config {
            price_feed_period_secs: 33,
            expected_feeders: Percent::from_percent(44),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should now be 12
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(Percent::from_percent(44), value.expected_feeders);
        assert_eq!(Duration::from_secs(33), value.price_feed_period);
    }

    #[test]
    #[should_panic(expected = "Price feed period can not be 0")]
    fn configure_invalid_period() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        let msg = ExecuteMsg::Config {
            price_feed_period_secs: 0,
            expected_feeders: Percent::from_percent(44),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn configure_feeders_percent() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());
        let expected_err = ContractError::Configuration(
            "Percent of expected available feeders should be > 0 and <= 1000".to_string(),
        );

        let err = exec_configure(deps.as_mut(), &info, 120, 0).unwrap_err();
        assert_eq!(expected_err, err);
        let err = exec_configure(deps.as_mut(), &info, 120, 1001).unwrap_err();
        assert_eq!(expected_err, err);
        let err = exec_configure(deps.as_mut(), &info, 120, 10401).unwrap_err();
        assert_eq!(expected_err, err);
        let err = exec_configure(deps.as_mut(), &info, 0, 10401).unwrap_err();
        assert_eq!(expected_err, err);
        let err = exec_configure(deps.as_mut(), &info, 0, 101).unwrap_err();
        assert_eq!(expected_err, err);
        exec_configure(deps.as_mut(), &info, 120, 14).unwrap();
    }

    fn exec_configure(
        deps: DepsMut,
        info: &MessageInfo,
        period: u32,
        f_percent: u16,
    ) -> Result<Response, ContractError> {
        let msg = ExecuteMsg::Config {
            price_feed_period_secs: period,
            expected_feeders: Percent::from_percent(f_percent),
        };
        execute(deps, mock_env(), info.to_owned(), msg)
    }

    #[test]
    fn config_supported_pairs() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        let test_vec = vec![
            vec![TestCurrencyA::TICKER.to_string(), Usdc::TICKER.to_string()],
            vec![Nls::TICKER.to_string(), Usdc::TICKER.to_string()],
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
    #[should_panic(expected = "Unauthorized")]
    fn config_supported_pairs_unauthorized() {
        let (mut deps, _) = setup_test(dummy_default_instantiate_msg());
        let info = mock_info("user", &coins(1000, Nls::TICKER));

        let msg = ExecuteMsg::CurrencyPaths {
            paths: vec![vec![Nls::TICKER.to_string(), Usdc::TICKER.to_string()]],
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn invalid_supported_pairs() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        let msg = ExecuteMsg::CurrencyPaths {
            paths: vec![
                vec![TestCurrencyA::TICKER.to_string(), Usdc::TICKER.to_string()],
                vec![Nls::TICKER.to_string(), Nls::TICKER.to_string()],
            ],
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            ContractError::InvalidResolutionPath(vec![
                Nls::TICKER.to_string(),
                Nls::TICKER.to_string()
            ]),
            err
        );
    }
}
