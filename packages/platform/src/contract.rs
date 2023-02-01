use sdk::cosmwasm_std::{Addr, ContractInfoResponse, QuerierWrapper, WasmQuery};

use crate::error::{Error, Result};

pub fn validate_addr(querier: &QuerierWrapper, contract_address: &Addr) -> Result<()> {
    query_info(querier, contract_address).map(|_| ())
}

pub fn validate_code_id(
    querier: &QuerierWrapper,
    contract_address: &Addr,
    expected_code_id: u64,
) -> Result<()> {
    query_info(querier, contract_address).and_then(|info| {
        if info.code_id == expected_code_id {
            Ok(())
        } else {
            Err(Error::unexpected_code(
                expected_code_id,
                contract_address.clone(),
            ))
        }
    })
}

fn query_info(querier: &QuerierWrapper, contract_address: &Addr) -> Result<ContractInfoResponse> {
    let raw = WasmQuery::ContractInfo {
        contract_addr: contract_address.into(),
    }
    .into();
    querier.query(&raw).map_err(Error::from)
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::{testing::MockQuerier, Addr, QuerierWrapper};

    use crate::contract::testing::{self, CODE_ID};

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
        mock_querier.update_wasm(testing::valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);

        let address = Addr::unchecked("some address");
        assert!(validate_addr(&querier, &address).is_ok());
    }

    #[test]
    fn validate_code_id() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(testing::valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);

        let address = Addr::unchecked("some address");
        assert!(super::validate_code_id(&querier, &address, CODE_ID).is_ok());
    }
}

#[cfg(any(feature = "testing", test))]
pub mod testing {
    use sdk::cosmwasm_std::{
        to_binary, ContractInfoResponse, ContractResult, QuerierResult, SystemResult, WasmQuery,
    };

    pub const CODE_ID: u64 = 20;

    pub fn valid_contract_handler(_query: &WasmQuery) -> QuerierResult {
        SystemResult::Ok(ContractResult::Ok(
            to_binary(&{
                let mut response = ContractInfoResponse::default();

                response.code_id = CODE_ID;
                response.creator = "some data".into();

                response
            })
            .unwrap(),
        ))
    }
}
