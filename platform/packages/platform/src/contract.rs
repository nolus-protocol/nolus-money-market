use std::mem;

use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Addr, CodeInfoResponse, ContractInfoResponse, QuerierWrapper, WasmQuery};

use crate::{error::Error, result::Result};

pub type CodeId = u64;

/// Abstracts the platform specific validation of smart cotract codes and instances
pub trait Validator {
    /// Validates the code ID identifies a valid smart contract code
    fn check_code(&self, id: CodeId) -> Result<CodeId>;

    /// Validates the contract is a smart contract instance
    fn check_contract(&self, contract: &Addr) -> Result<()>;

    /// Validates the contract is an instance of the smart contract code
    fn check_contract_code(&self, contract: Addr, code: &Code) -> Result<Addr>;
}

pub fn validator(querier: QuerierWrapper<'_>) -> impl Validator + use<'_> {
    CosmwasmValidator::new(querier)
}

struct CosmwasmValidator<'q>(QuerierWrapper<'q>);
impl<'q> CosmwasmValidator<'q> {
    fn new(querier: QuerierWrapper<'q>) -> Self {
        Self(querier)
    }

    fn query_contract(&self, contract_address: &Addr) -> Result<ContractInfoResponse> {
        self.0
            .query(
                &WasmQuery::ContractInfo {
                    contract_addr: contract_address.into(),
                }
                .into(),
            )
            .map_err(Error::CosmWasmQueryContractInfo)
    }
}

impl Validator for CosmwasmValidator<'_> {
    fn check_code(&self, id: CodeId) -> Result<CodeId> {
        self.0
            .query(&WasmQuery::CodeInfo { code_id: id }.into())
            .map_err(Error::CosmWasmQueryCodeInfo)
            .inspect(|response: &CodeInfoResponse| assert_eq!(id, response.code_id))
            .map(|_| id)
    }

    fn check_contract(&self, contract: &Addr) -> Result<()> {
        self.query_contract(contract).map(mem::drop)
    }

    fn check_contract_code(&self, contract: Addr, code: &Code) -> Result<Addr> {
        self.query_contract(&contract).and_then(|response| {
            if response.code_id == code.id {
                Ok(contract)
            } else {
                Err(Error::unexpected_code(code.id, contract))
            }
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", transparent)]
/// A valid Cosmwasm code that may be stored and transferred
/// Not indended to be used in external APIs since there is no way to integrate validation on deserialization!
/// Instead, use [CodeId], or Uint64, in APIs and [Code::try_new] to validate the input.
pub struct Code {
    id: CodeId,
}

impl Code {
    pub fn try_new<V>(id: CodeId, validator: &V) -> Result<Self>
    where
        V: Validator,
    {
        validator.check_code(id).map(Self::new)
    }

    #[cfg(any(test, feature = "testing"))]
    pub const fn unchecked(id: CodeId) -> Self {
        Self::new(id)
    }

    const fn new(id: CodeId) -> Self {
        Self { id }
    }
}

impl From<Code> for CodeId {
    fn from(value: Code) -> Self {
        value.id
    }
}

#[cfg(test)]
pub mod tests {
    use sdk::{
        cosmwasm_std::{self, QuerierWrapper, testing::MockQuerier},
        testing as sdk_testing,
    };

    use crate::contract::{
        Code, Validator,
        testing::{self, CODE},
    };

    use super::CodeId;

    const USER: &str = "user";

    #[test]
    fn validate_invalid_addr() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        assert!(
            super::validator(querier)
                .check_contract(&sdk_testing::user(USER))
                .is_err()
        );
    }

    #[test]
    fn validate_valid_addr() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(testing::valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);

        assert_eq!(
            Ok(()),
            super::validator(querier).check_contract(&sdk_testing::user(USER))
        );
    }

    #[test]
    fn validate_code_id() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(testing::valid_contract_handler);
        let querier = QuerierWrapper::new(&mock_querier);

        let user_addr = sdk_testing::user(USER);
        assert_eq!(
            Ok(user_addr.clone()),
            super::validator(querier).check_contract_code(user_addr, &CODE)
        );
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
    use sdk::{
        cosmwasm_std::{
            self, ContractInfoResponse, ContractResult, QuerierResult, SystemResult, WasmQuery,
        },
        testing,
    };

    use super::Code;

    pub const CODE: Code = Code::unchecked(20);

    pub fn valid_contract_handler(_: &WasmQuery) -> QuerierResult {
        SystemResult::Ok(ContractResult::Ok(
            cosmwasm_std::to_json_binary(&ContractInfoResponse::new(
                CODE.into(),
                testing::user("user"),
                None,
                false,
                None,
            ))
            .expect("serialization succeedeed"),
        ))
    }
}
