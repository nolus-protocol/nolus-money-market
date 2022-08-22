use cosmwasm_std::{coin, Addr};
use cw_multi_test::ContractWrapper;
use finance::{liability::Liability, percent::Percent};
use lease::msg::{LoanForm, NewLeaseForm};

use cw_multi_test::{App, Executor};

use super::{ADMIN, USER};

type LeaseContractWrapperReply = Box<
    ContractWrapper<
        lease::msg::ExecuteMsg,
        lease::msg::NewLeaseForm,
        lease::msg::StateQuery,
        lease::error::ContractError,
        lease::error::ContractError,
        lease::error::ContractError,
        cosmwasm_std::Empty,
        cosmwasm_std::Empty,
        cosmwasm_std::Empty,
        anyhow::Error,
        lease::error::ContractError,
    >,
>;

pub struct LeaseWrapper {
    contract_wrapper: LeaseContractWrapperReply,
}

pub struct LeaseWrapperConfig {
    //NewLeaseForm
    pub customer: String,
    // Liability
    pub liability_init_percent: Percent,
    pub liability_delta_to_healthy_percent: Percent,
    pub liability_delta_to_max_percent: Percent,
    pub liability_recalc_hours: u16,
    // LoanForm
    pub annual_margin_interest: Percent,
    pub interest_due_period_secs: u32,
    pub grace_period_secs: u32,

    pub downpayment: u128,
}

impl Default for LeaseWrapperConfig {
    fn default() -> Self {
        Self {
            customer: USER.to_string(),
            liability_init_percent: Percent::from_percent(65),
            liability_delta_to_healthy_percent: Percent::from_percent(5),
            liability_delta_to_max_percent: Percent::from_percent(10),
            liability_recalc_hours: 20 * 24,

            annual_margin_interest: Percent::from_percent(0), // 3.1%
            interest_due_period_secs: 100, // 90 days TODO use a crate for daytime calculations
            grace_period_secs: 10,         // 10 days TODO use a crate for daytime calculations

            // TODO: extend to Coin (downpayment and lpn can be different)
            downpayment: 1000u128,
        }
    }
}

impl LeaseWrapper {
    pub fn store(self, app: &mut App) -> u64 {
        app.store_code(self.contract_wrapper)
    }

    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut App,
        code_id: Option<u64>,
        lpp_addr: &Addr,
        denom: &str,
        config: LeaseWrapperConfig,
    ) -> Addr {
        let code_id = match code_id {
            Some(id) => id,
            None => app.store_code(self.contract_wrapper),
        };

        let downpayment = config.downpayment;
        let msg = Self::lease_instantiate_msg(denom, lpp_addr.clone(), config);

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[coin(downpayment, denom)],
            "lease",
            None,
        )
        .unwrap()
    }

    fn lease_instantiate_msg(
        denom: &str,
        lpp_addr: Addr,
        config: LeaseWrapperConfig,
    ) -> NewLeaseForm {
        NewLeaseForm {
            customer: config.customer,
            currency: denom.to_string(),
            liability: Liability::new(
                config.liability_init_percent,
                config.liability_delta_to_healthy_percent,
                config.liability_delta_to_max_percent,
                config.liability_recalc_hours,
            ),
            loan: LoanForm {
                annual_margin_interest: config.annual_margin_interest,
                lpp: lpp_addr.into_string(),
                interest_due_period_secs: config.interest_due_period_secs,
                grace_period_secs: config.grace_period_secs,
            },
        }
    }
}

impl Default for LeaseWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            lease::contract::execute,
            lease::contract::instantiate,
            lease::contract::query,
        )
        .with_reply(lease::contract::reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
