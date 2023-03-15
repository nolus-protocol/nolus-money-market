use access_control::SingleUserAccess;
use marketprice::config::Config as PriceConfig;
use sdk::{cosmwasm_ext::Response, cosmwasm_std::Storage};

use crate::{msg::ConfigResponse, state::config::Config, ContractError};

pub fn query_config(storage: &dyn Storage) -> Result<ConfigResponse, ContractError> {
    let owner = SingleUserAccess::load_contract_owner(storage)?.into();
    let config = Config::load(storage)?;

    Ok(ConfigResponse { owner, config })
}

pub fn try_configure(
    storage: &mut dyn Storage,
    price_config: PriceConfig,
) -> Result<Response, ContractError> {
    Config::update(storage, price_config)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use currency::{
        lease::{Cro, Osmo},
        lpn::Usdc,
    };
    use finance::{currency::Currency, duration::Duration, percent::Percent};

    use sdk::cosmwasm_std::{from_binary, testing::mock_env};
    use swap::SwapTarget;

    use crate::contract::sudo;
    use crate::{
        contract::query,
        msg::{ConfigResponse, QueryMsg, SudoMsg},
        state::{config::Config, supported_pairs::SwapLeg},
        swap_tree,
        tests::{dummy_default_instantiate_msg, dummy_instantiate_msg, setup_test},
    };

    #[test]
    fn configure() {
        use marketprice::config::Config as PriceConfig;
        let msg = dummy_instantiate_msg(
            Usdc::TICKER.to_string(),
            60,
            Percent::from_percent(50),
            swap_tree!((1, Cro::TICKER)),
        );
        let (mut deps, info) = setup_test(msg);

        let msg = SudoMsg::UpdateConfig(PriceConfig::new(
            Percent::from_percent(44),
            Duration::from_secs(5),
            7,
            Percent::from_percent(88),
        ));

        drop(sudo(deps.as_mut(), mock_env(), msg).unwrap());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            ConfigResponse {
                owner: info.sender,
                config: Config {
                    base_asset: Usdc::TICKER.into(),
                    price_config: PriceConfig::new(
                        Percent::from_percent(44),
                        Duration::from_secs(5),
                        7,
                        Percent::from_percent(88),
                    )
                }
            },
            value
        );
    }

    #[test]
    fn config_supported_pairs() {
        let (mut deps, _info) = setup_test(dummy_default_instantiate_msg());

        let test_tree = swap_tree!((1, Cro::TICKER), (2, Osmo::TICKER));

        let res = sudo(
            deps.as_mut(),
            mock_env(),
            SudoMsg::SwapTree { tree: test_tree },
        );
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
                    target: Usdc::TICKER.into(),
                },
            },
            SwapLeg {
                from: Osmo::TICKER.into(),
                to: SwapTarget {
                    pool_id: 2,
                    target: Usdc::TICKER.into(),
                },
            },
        ];
        expected.sort_by(|a, b| a.from.cmp(&b.from));

        assert_eq!(expected, value);
    }

    #[test]
    #[should_panic]
    fn invalid_supported_pairs() {
        let (mut deps, _info) = setup_test(dummy_default_instantiate_msg());

        let test_tree = swap_tree!((1, Cro::TICKER), (2, Cro::TICKER));

        drop(
            sudo(
                deps.as_mut(),
                mock_env(),
                SudoMsg::SwapTree { tree: test_tree },
            )
            .unwrap(),
        );
    }
}
