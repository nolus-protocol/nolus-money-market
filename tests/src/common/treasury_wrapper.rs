use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, StdError},
    cw_multi_test::Executor,
};
use treasury::{
    msg::{ExecuteMsg, InstantiateMsg},
    ContractError,
};

use crate::common::{ContractWrapper, MockApp};

use super::{mock_query, MockQueryMsg, ADMIN, NATIVE_DENOM};

pub fn treasury_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {}
}

pub struct TreasuryWrapper {
    contract_wrapper: Box<TreasuryContractWrapper>,
}

impl TreasuryWrapper {
    #[track_caller]
    pub fn instantiate(self, app: &mut MockApp, denom: &str) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = treasury_instantiate_msg();

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[CwCoin::new(1000, denom), CwCoin::new(1000, NATIVE_DENOM)],
            "treasury",
            None,
        )
        .unwrap()
    }
}

impl Default for TreasuryWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            treasury::contract::execute,
            treasury::contract::instantiate,
            mock_query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

type TreasuryContractWrapper = ContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    MockQueryMsg,
    StdError,
>;
