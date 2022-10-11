use sdk::cosmwasm_std::{Addr, ContractInfoResponse, QuerierWrapper, WasmQuery};

use crate::error::{Error, Result};

pub fn validate_addr(querier: &QuerierWrapper, contract_address: &Addr) -> Result<()> {
    let raw = WasmQuery::ContractInfo {
        contract_addr: contract_address.into(),
    }
    .into();
    querier
        .query::<ContractInfoResponse>(&raw)
        .map(|_| ())
        .map_err(Error::from)
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::{testing::MockQuerier, Addr, QuerierWrapper};

    use crate::contract::testing::valid_contract_handler;

    use super::validate_addr;

    #[test]
    fn validate_invalid_addr() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let address = Addr::unchecked("some address");
        assert!(validate_addr(&querier, &address).is_err());
    }

    #[test]
    fn validate_valid_addr() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);
        let address = Addr::unchecked("some address");
        assert!(validate_addr(&querier, &address).is_ok());
    }
}

#[cfg(any(feature = "testing", test))]
pub mod testing {
    use sdk::cosmwasm_std::{
        to_binary, ContractInfoResponse, ContractResult, QuerierResult, SystemResult, WasmQuery,
    };

    pub fn valid_contract_handler(_query: &WasmQuery) -> QuerierResult {
        SystemResult::Ok(ContractResult::Ok(
            to_binary(&ContractInfoResponse::new(20, "some data")).unwrap(),
        ))
    }
}
