use sdk::{
    cosmwasm_std::{Addr, CodeInfoResponse, ContractInfoResponse, QuerierWrapper, WasmQuery},
    schemars::{self, JsonSchema},
};
use serde::{Deserialize, Serialize};

use crate::{error::Error, result::Result};

pub type CodeId = u64;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[serde(transparent)]
/// A valid Cosmwasm code that may be stored and transferred
/// Not indended to be used in external APIs since there is no way to integrate validation on deserialization!
/// Instead, use [CodeId] in APIs and [Code::try_new] to validate the input.
pub struct Code {
    id: CodeId,
}

impl Code {
    pub fn try_new(id: CodeId, querier: &QuerierWrapper<'_>) -> Result<Self> {
        let raw = WasmQuery::CodeInfo { code_id: id }.into();
        querier
            .query(&raw)
            .map_err(Error::from)
            .map(|resp: CodeInfoResponse| Self { id: resp.code_id })
    }

    #[cfg(any(test, feature = "testing"))]
    pub const fn unchecked(id: CodeId) -> Self {
        Self { id }
    }
}

impl From<Code> for CodeId {
    fn from(value: Code) -> Self {
        value.id
    }
}

pub fn validate_addr(querier: QuerierWrapper<'_>, contract_address: &Addr) -> Result<()> {
    query_info(querier, contract_address).map(|_| ())
}

pub fn validate_code_id(
    querier: QuerierWrapper<'_>,
    contract_address: &Addr,
    expected_code: Code,
) -> Result<()> {
    query_info(querier, contract_address).and_then(|info| {
        if info.code_id == expected_code.id {
            Ok(())
        } else {
            Err(Error::unexpected_code(
                expected_code.into(),
                contract_address.clone(),
            ))
        }
    })
}

fn query_info(
    querier: QuerierWrapper<'_>,
    contract_address: &Addr,
) -> Result<ContractInfoResponse> {
    let raw = WasmQuery::ContractInfo {
        contract_addr: contract_address.into(),
    }
    .into();
    querier.query(&raw).map_err(Error::from)
}

#[cfg(test)]
pub mod tests {
    use sdk::cosmwasm_std::{self, testing::MockQuerier, Addr, QuerierWrapper};

    use crate::contract::{
        testing::{self, CODE},
        Code,
    };

    use super::{validate_addr, CodeId};

    #[test]
    fn validate_invalid_addr() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let address = Addr::unchecked("some address");
        assert!(validate_addr(querier, &address).is_err());
    }

    #[test]
    fn validate_valid_addr() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(testing::valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);

        let address = Addr::unchecked("some address");
        assert!(validate_addr(querier, &address).is_ok());
    }

    #[test]
    fn validate_code_id() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(testing::valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);

        let address = Addr::unchecked("some address");
        assert!(super::validate_code_id(querier, &address, CODE).is_ok());
    }

    #[test]
    fn transparent_serde() {
        let id: CodeId = 13;
        assert_eq!(
            Code::unchecked(id),
            cosmwasm_std::from_json(cosmwasm_std::to_json_string(&id).unwrap()).unwrap()
        );
    }
}

#[cfg(any(feature = "testing", test))]
pub mod testing {
    use sdk::cosmwasm_std::{
        to_json_binary, ContractInfoResponse, ContractResult, QuerierResult, SystemResult,
        WasmQuery,
    };

    use super::Code;

    pub const CODE: Code = Code::unchecked(20);

    pub fn valid_contract_handler(_query: &WasmQuery) -> QuerierResult {
        SystemResult::Ok(ContractResult::Ok(
            to_json_binary(&{
                let mut response = ContractInfoResponse::default();

                response.code_id = CODE.into();
                response.creator = "some data".into();

                response
            })
            .unwrap(),
        ))
    }
}
