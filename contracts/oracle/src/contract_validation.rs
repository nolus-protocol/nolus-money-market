use cosmwasm_std::{
    Addr, ContractInfoResponse, Empty, QuerierWrapper, QueryRequest, StdResult, WasmQuery,
};

//TODO move to the platform
pub fn validate_contract_addr(querier: &QuerierWrapper, contract_address: &Addr) -> StdResult<()> {
    get_contract_info(querier, contract_address).map(|_| ())
}

fn get_contract_info(
    querier: &QuerierWrapper,
    contract_address: &Addr,
) -> StdResult<ContractInfoResponse> {
    let raw = QueryRequest::<Empty>::Wasm(WasmQuery::ContractInfo {
        contract_addr: contract_address.into(),
    });
    querier.query(&raw)
}

#[cfg(test)]
pub mod tests {
    use cosmwasm_std::{
        testing::MockQuerier, to_binary, Addr, ContractInfoResponse, ContractResult, QuerierResult,
        QuerierWrapper, SystemResult, WasmQuery,
    };

    use super::validate_contract_addr;

    pub fn valid_contract_query(_query: &WasmQuery) -> QuerierResult {
        SystemResult::Ok(ContractResult::Ok(
            to_binary(&ContractInfoResponse::new(20, "some data")).unwrap(),
        ))
    }

    #[test]
    fn validate_contract_addr_user_address() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let address = Addr::unchecked("some address");
        assert!(validate_contract_addr(&querier, &address).is_err());
    }

    #[test]
    fn validate_contract_addr_contract_address() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(valid_contract_query);
        let querier = QuerierWrapper::new(&mock_querier);
        let address = Addr::unchecked("some address");
        assert!(validate_contract_addr(&querier, &address).is_ok());
    }
}
