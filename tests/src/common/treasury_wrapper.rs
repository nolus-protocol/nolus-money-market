use currency::Currency;
use sdk::cosmwasm_std::{Addr, StdError};
use treasury::{
    contract::sudo,
    msg::{ExecuteMsg, InstantiateMsg, SudoMsg},
    ContractError,
};

use super::{
    cwcoin, mock_query, native_cwcoin, test_case::App, CwContractWrapper, MockQueryMsg, ADMIN,
};

pub(crate) struct TreasuryWrapper {
    contract_wrapper: Box<TreasuryContractWrapper>,
    rewards_dispatcher: Addr,
}

impl TreasuryWrapper {
    pub fn new(rewards_dispatcher: Addr) -> Self {
        Self {
            contract_wrapper: Box::new(
                CwContractWrapper::new(
                    treasury::contract::execute,
                    treasury::contract::instantiate,
                    mock_query,
                )
                .with_sudo(sudo),
            ),
            rewards_dispatcher,
        }
    }

    pub fn new_with_no_dispatcher() -> Self {
        Self::new(Addr::unchecked("DEADCODE"))
    }

    #[track_caller]
    pub fn instantiate<Lpn>(self, app: &mut App) -> Addr
    where
        Lpn: Currency,
    {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            rewards_dispatcher: self.rewards_dispatcher,
        };

        app.instantiate(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[cwcoin::<Lpn, _>(1000), native_cwcoin(1000)],
            "treasury",
            None,
        )
        .unwrap()
        .unwrap_response()
    }
}

type TreasuryContractWrapper = CwContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    MockQueryMsg,
    StdError,
    SudoMsg,
    ContractError,
>;
