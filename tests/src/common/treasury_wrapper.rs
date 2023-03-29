use finance::currency::Currency;
use sdk::{
    cosmwasm_std::{Addr, StdError},
    cw_multi_test::Executor,
};
use treasury::{
    contract::sudo,
    msg::{ExecuteMsg, InstantiateMsg, SudoMsg},
    ContractError,
};

use crate::common::{ContractWrapper, MockApp};

use super::{cwcoin, mock_query, native_cwcoin, MockQueryMsg, ADMIN};

pub fn treasury_instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {}
}

pub struct TreasuryWrapper {
    contract_wrapper: Box<TreasuryContractWrapper>,
}

impl TreasuryWrapper {
    #[track_caller]
    pub fn instantiate<Lpn>(self, app: &mut MockApp) -> Addr
    where
        Lpn: Currency,
    {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = treasury_instantiate_msg();

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[cwcoin::<Lpn, _>(1000), native_cwcoin(1000)],
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
        )
        .with_sudo(sudo);

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
    SudoMsg,
    ContractError,
>;
