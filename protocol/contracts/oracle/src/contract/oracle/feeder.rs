use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use marketprice::feeders::PriceFeeders;
use sdk::cosmwasm_std::{Addr, DepsMut, StdResult, Storage};

use crate::{msg::Config, result::ContractResult, ContractError};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct Feeders {
    config: Config,
}

impl Feeders {
    const FEEDERS: PriceFeeders<'static> = PriceFeeders::new("feeders");

    pub(crate) fn get(storage: &dyn Storage) -> StdResult<HashSet<Addr>> {
        Self::FEEDERS.get(storage)
    }

    pub(crate) fn is_feeder(storage: &dyn Storage, address: &Addr) -> StdResult<bool> {
        Self::FEEDERS.is_registered(storage, address)
    }

    pub(crate) fn try_register(deps: DepsMut<'_>, address: String) -> ContractResult<()> {
        // check if address is valid
        let f_address = deps
            .api
            .addr_validate(&address)
            .map_err(ContractError::RegisterFeederAddressValidation)?;
        Self::FEEDERS.register(deps, f_address).map_err(Into::into)
    }

    pub(crate) fn try_remove(deps: DepsMut<'_>, address: String) -> ContractResult<()> {
        let f_address = deps
            .api
            .addr_validate(&address)
            .map_err(ContractError::UnregisterFeederAddressValidation)?;

        if !Self::is_feeder(deps.storage, &f_address).map_err(ContractError::LoadFeeders)? {
            return Err(ContractError::UnknownFeeder {});
        }

        Self::FEEDERS.remove(deps, &f_address).map_err(Into::into)
    }

    pub(crate) fn total_registered(storage: &dyn Storage) -> StdResult<usize> {
        Self::get(storage).map(|c| c.len())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use sdk::{
        cosmwasm_ext::Response as CwResponse,
        cosmwasm_std::{from_binary, testing::mock_env, Addr, DepsMut},
    };

    use crate::{
        contract::{query, sudo},
        msg::{QueryMsg, SudoMsg},
        result::ContractResult,
        tests::{dummy_default_instantiate_msg, setup_test},
    };

    #[test]
    fn register_feeder() {
        let (mut deps, _info) = setup_test(dummy_default_instantiate_msg());

        // register new feeder address
        register(deps.as_mut(), "addr0000").unwrap();

        // check if the new address is added to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(2, resp.len());
        assert!(resp.contains(&Addr::unchecked("addr0000")));

        // should not add the same address twice
        assert!(register(deps.as_mut(), "addr0000").is_err());

        // register new feeder address
        register(deps.as_mut(), "addr0001").unwrap();
        // check if the new address is added to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(3, resp.len());
        assert!(resp.contains(&Addr::unchecked("addr0000")));
        assert!(resp.contains(&Addr::unchecked("addr0001")));
    }

    #[test]
    fn remove_feeder() {
        let (mut deps, _info) = setup_test(dummy_default_instantiate_msg());

        register(deps.as_mut(), "addr0000").unwrap();
        register(deps.as_mut(), "addr0001").unwrap();
        register(deps.as_mut(), "addr0002").unwrap();
        register(deps.as_mut(), "addr0003").unwrap();

        // check if the new address is added to FEEDERS Item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(5, resp.len());
        assert!(resp.contains(&Addr::unchecked("addr0000")));
        assert!(resp.contains(&Addr::unchecked("addr0001")));

        remove(deps.as_mut(), "addr0000");
        remove(deps.as_mut(), "addr0001");
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Feeders {}).unwrap();
        let resp: HashSet<Addr> = from_binary(&res).unwrap();
        assert_eq!(3, resp.len());
        assert!(!resp.contains(&Addr::unchecked("addr0000")));
        assert!(!resp.contains(&Addr::unchecked("addr0001")));
    }

    fn register(deps: DepsMut<'_>, address: &str) -> ContractResult<CwResponse> {
        sudo(
            deps,
            mock_env(),
            SudoMsg::RegisterFeeder {
                feeder_address: address.to_string(),
            },
        )
    }

    fn remove(deps: DepsMut<'_>, address: &str) {
        let CwResponse {
            messages,
            attributes,
            events,
            data,
            ..
        }: CwResponse = sudo(
            deps,
            mock_env(),
            SudoMsg::RemoveFeeder {
                feeder_address: address.to_string(),
            },
        )
        .unwrap();

        assert_eq!(messages.len(), 0);
        assert_eq!(attributes.len(), 0);
        assert_eq!(events.len(), 0);
        assert_eq!(data, None);
    }
}
