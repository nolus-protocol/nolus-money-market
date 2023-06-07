use finance::percent::Percent;
use rewards_dispatcher::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg},
    state::reward_scale::{Bar, RewardScale, TotalValueLocked},
};
use sdk::{cosmwasm_std::Addr, cw_multi_test::Executor};

use crate::common::{ContractWrapper, MockApp};

use super::ADMIN;

pub struct DispatcherWrapper {
    contract_wrapper: Box<DispatcherContractWrapper>,
}

impl DispatcherWrapper {
    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut MockApp,
        lpp: Addr,
        oracle: Addr,
        timealarms: Addr,
        treasury: Addr,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            cadence_hours: 10,
            lpp,
            oracle,
            timealarms,
            treasury,
            tvl_to_apr: RewardScale::try_from(vec![
                Bar {
                    tvl: Default::default(),
                    apr: Percent::from_permille(10),
                },
                Bar {
                    tvl: TotalValueLocked::new(1000),
                    apr: Percent::from_permille(10),
                },
            ])
            .unwrap(),
        };

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[],
            "dispatcher",
            None,
        )
        .unwrap()
    }
}

impl Default for DispatcherWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            rewards_dispatcher::contract::execute,
            rewards_dispatcher::contract::instantiate,
            rewards_dispatcher::contract::query,
        )
        .with_sudo(rewards_dispatcher::contract::sudo);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

type DispatcherContractWrapper = ContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    QueryMsg,
    ContractError,
    SudoMsg,
    ContractError,
>;
