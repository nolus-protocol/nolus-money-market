use sdk::cosmwasm_std::Storage;

use crate::{msg::ConfigResponse, state::config::Config, ContractError};

pub(super) fn query_config(storage: &dyn Storage) -> Result<ConfigResponse, ContractError> {
    Config::load(storage)
        .map_err(ContractError::LoadConfig)
        .map(|config| ConfigResponse { config })
}

#[cfg(test)]
mod tests {
    use currency::{
        dex::test::{PaymentC3, PaymentC6, StableC1},
        Currency,
    };
    use finance::{duration::Duration, percent::Percent};
    use sdk::{
        cosmwasm_ext::Response,
        cosmwasm_std::{from_binary, testing::mock_env},
    };
    use swap::SwapTarget;

    use crate::{
        contract::{query, sudo},
        msg::{ConfigResponse, QueryMsg, SudoMsg},
        state::{config::Config, supported_pairs::SwapLeg},
        swap_tree,
        tests::{dummy_default_instantiate_msg, dummy_instantiate_msg, setup_test},
    };

    #[test]
    fn configure() {
        use marketprice::config::Config as PriceConfig;
        let msg = dummy_instantiate_msg(
            StableC1::TICKER.to_string(),
            60,
            Percent::from_percent(50),
            swap_tree!({ base: StableC1::TICKER }, (1, PaymentC3::TICKER)),
        );
        let (mut deps, _info) = setup_test(msg);

        let msg = SudoMsg::UpdateConfig(PriceConfig::new(
            Percent::from_percent(44),
            Duration::from_secs(5),
            7,
            Percent::from_percent(88),
        ));

        let Response {
            messages,
            attributes,
            events,
            data,
            ..
        }: Response = sudo(deps.as_mut(), mock_env(), msg).unwrap();

        assert!(messages.is_empty());
        assert!(attributes.is_empty());
        assert!(events.is_empty());
        assert!(data.is_none());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            value,
            ConfigResponse {
                config: Config {
                    base_asset: StableC1::TICKER.into(),
                    price_config: PriceConfig::new(
                        Percent::from_percent(44),
                        Duration::from_secs(5),
                        7,
                        Percent::from_percent(88),
                    )
                }
            }
        );
    }

    #[test]
    fn config_supported_pairs() {
        let (mut deps, _info) = setup_test(dummy_default_instantiate_msg());

        let test_tree =
            swap_tree!({ base: StableC1::TICKER }, (1, PaymentC3::TICKER), (2, PaymentC6::TICKER));

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
                from: PaymentC3::TICKER.into(),
                to: SwapTarget {
                    pool_id: 1,
                    target: StableC1::TICKER.into(),
                },
            },
            SwapLeg {
                from: PaymentC6::TICKER.into(),
                to: SwapTarget {
                    pool_id: 2,
                    target: StableC1::TICKER.into(),
                },
            },
        ];
        expected.sort_by(|a, b| a.from.cmp(&b.from));

        assert_eq!(value, expected);
    }

    #[test]
    #[should_panic]
    fn invalid_supported_pairs() {
        let (mut deps, _info) = setup_test(dummy_default_instantiate_msg());

        let test_tree =
            swap_tree!({ base: StableC1::TICKER }, (1, PaymentC3::TICKER), (2, PaymentC3::TICKER));

        let Response {
            messages,
            attributes,
            events,
            data,
            ..
        }: Response = sudo(
            deps.as_mut(),
            mock_env(),
            SudoMsg::SwapTree { tree: test_tree },
        )
        .unwrap();

        assert!(messages.is_empty());
        assert!(attributes.is_empty());
        assert!(events.is_empty());
        assert!(data.is_none());
    }
}
