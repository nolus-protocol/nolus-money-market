use cosmwasm_std::{Addr, Uint64};
use cw_multi_test::ContractWrapper;

use cw_multi_test::{App, Executor};
use finance::duration::Duration;
use finance::liability::Liability;
use finance::percent::Percent;
use leaser::msg::Repayment;
use leaser::ContractError;

use super::ADMIN;

type LeaserContractWrapperReply = Box<
    ContractWrapper<
        leaser::msg::ExecuteMsg,
        leaser::msg::InstantiateMsg,
        leaser::msg::QueryMsg,
        ContractError,
        ContractError,
        ContractError,
        cosmwasm_std::Empty,
        cosmwasm_std::Empty,
        cosmwasm_std::Empty,
        anyhow::Error,
        ContractError,
    >,
>;

pub struct LeaserWrapper {
    contract_wrapper: LeaserContractWrapperReply,
}
impl LeaserWrapper {
    pub const INTEREST_RATE_MARGIN: Percent = Percent::from_permille(30);

    pub const REPAYMENT_PERIOD_SECS: u32 = 90 * 24 * 60 * 60;

    pub const REPAYMENT_PERIOD: Duration = Duration::from_secs(Self::REPAYMENT_PERIOD_SECS);

    #[track_caller]
    pub fn instantiate(self, app: &mut App, lease_code_id: u64, lpp_addr: &Addr) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = leaser::msg::InstantiateMsg {
            lease_code_id: Uint64::new(lease_code_id),
            lpp_ust_addr: lpp_addr.clone(),
            lease_interest_rate_margin: Self::INTEREST_RATE_MARGIN,
            liability: Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(70),
                Percent::from_percent(80),
                1,
            ),
            repayment: Repayment::new(Self::REPAYMENT_PERIOD_SECS, 10 * 24 * 60 * 60),
        };

        app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "leaser", None)
            .unwrap()
    }
}

impl Default for LeaserWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            leaser::contract::execute,
            leaser::contract::instantiate,
            leaser::contract::query,
        )
        .with_reply(leaser::contract::reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
