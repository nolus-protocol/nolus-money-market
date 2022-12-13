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
        owner: crate::access_control::OWNER.get_address::<_, ContractError>(deps)?,
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
    crate::access_control::OWNER.assert_address::<_, ContractError>(deps.as_ref(), &info.sender)?;

    //TODO merge the next checks with the code in Config::validate()
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
    // TODO make sure the price_feed_period >= last block time
    Config::update(
        deps.storage,
        Duration::from_secs(price_feed_period),
        expected_feeders,
    )?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use currency::{
        lease::{Cro, Osmo},
        lpn::Usdc,
        native::Nls,
    };
    use finance::{currency::Currency, duration::Duration, percent::Percent};
    use sdk::{
        cosmwasm_ext::Response,
        cosmwasm_std::{
            coins, from_binary,
            testing::{mock_env, mock_info},
            DepsMut, MessageInfo,
        },
    };
    use swap::SwapTarget;
    use trees::tr;

    use crate::{
        contract::{execute, query},
        msg::{ConfigResponse, ExecuteMsg, QueryMsg},
        state::supported_pairs::{SwapLeg, TreeStore},
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
            TreeStore(tr((0, Usdc::TICKER.to_string())) / tr((1, Cro::TICKER.to_string()))),
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
            TreeStore(tr((0, Usdc::TICKER.to_string())) / tr((1, Cro::TICKER.to_string()))),
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

        let test_tree = tr((0, Usdc::TICKER.into()))
            / tr((1, Cro::TICKER.into()))
            / tr((2, Osmo::TICKER.into()));

        let msg = ExecuteMsg::SwapTree {
            tree: TreeStore(test_tree),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_ok());

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::SupportedCurrencyPairs {},
        )
        .unwrap();
        let mut value: Vec<SwapLeg> = from_binary(&res).unwrap();
        value.sort_by(|a, b| a.from.cmp(&b.from));

        let mut expected = vec![
            SwapLeg {
                from: Cro::TICKER.into(),
                to: SwapTarget {
                    pool_id: 1,
                    target: Usdc::TICKER.to_owned(),
                },
            },
            SwapLeg {
                from: Osmo::TICKER.into(),
                to: SwapTarget {
                    pool_id: 2,
                    target: Usdc::TICKER.to_owned(),
                },
            },
        ];
        expected.sort_by(|a, b| a.from.cmp(&b.from));

        assert_eq!(expected, value);
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn config_supported_pairs_unauthorized() {
        let (mut deps, _) = setup_test(dummy_default_instantiate_msg());
        let info = mock_info("user", &coins(1000, Nls::TICKER));

        let msg = ExecuteMsg::SwapTree {
            tree: TreeStore(tr((0, Usdc::TICKER.into())) / tr((1, Cro::TICKER.into()))),
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_supported_pairs() {
        let (mut deps, info) = setup_test(dummy_default_instantiate_msg());

        let test_tree = tr((0, Usdc::TICKER.into()))
            / tr((1, Cro::TICKER.into()))
            / tr((2, Cro::TICKER.into()));

        let msg = ExecuteMsg::SwapTree {
            tree: TreeStore(test_tree),
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
