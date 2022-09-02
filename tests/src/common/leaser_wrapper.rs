use cosmwasm_std::{Addr, Uint64};
use cw_multi_test::Executor;

use finance::{duration::Duration, liability::Liability, percent::Percent};
use leaser::{
    contract::{execute, instantiate, query, reply},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, Repayment},
    ContractError,
};

use crate::common::{ContractWrapper, MockApp};

use super::ADMIN;

pub struct LeaserWrapper {
    contract_wrapper: LeaserContractWrapperReply,
}
impl LeaserWrapper {
    pub const INTEREST_RATE_MARGIN: Percent = Percent::from_permille(30);

    pub const REPAYMENT_PERIOD_SECS: u32 = 90 * 24 * 60 * 60;

    pub const REPAYMENT_PERIOD: Duration = Duration::from_secs(Self::REPAYMENT_PERIOD_SECS);

    pub fn liability() -> Liability {
        Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            Percent::from_percent(2),
            Percent::from_percent(3),
            Percent::from_percent(2),
            1,
        )
    }

    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut MockApp,
        lease_code_id: u64,
        lpp_addr: &Addr,
        time_alarms: Addr,
        market_price_oracle: Addr,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            lease_code_id: Uint64::new(lease_code_id),
            lpp_ust_addr: lpp_addr.clone(),
            lease_interest_rate_margin: Self::INTEREST_RATE_MARGIN,
            liability: Self::liability(),
            repayment: Repayment::new(Self::REPAYMENT_PERIOD_SECS, 10 * 24 * 60 * 60),
            time_alarms,
            market_price_oracle,
        };

        app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "leaser", None)
            .unwrap()
    }
}

impl Default for LeaserWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

type LeaserContractWrapperReply = Box<
    ContractWrapper<
        ExecuteMsg,
        ContractError,
        InstantiateMsg,
        ContractError,
        QueryMsg,
        ContractError,
        cosmwasm_std::Empty,
        anyhow::Error,
        ContractError,
    >,
>;
