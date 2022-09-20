use cosmwasm_std::{coin, Addr};
use cw_multi_test::Executor;

use finance::{liability::Liability, percent::Percent};
use lease::{
    contract::{execute, instantiate, query, reply},
    error::ContractError,
    msg::{ExecuteMsg, LoanForm, NewLeaseForm, StateQuery},
};

use crate::common::{ContractWrapper, MockApp};

use super::{ADMIN, USER};

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
    pub liability_minus_delta_to_first_liq_warn: Percent,
    pub liability_minus_delta_to_second_liq_warn: Percent,
    pub liability_minus_delta_to_third_liq_warn: Percent,
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
            liability_minus_delta_to_first_liq_warn: Percent::from_percent(2),
            liability_minus_delta_to_second_liq_warn: Percent::from_percent(3),
            liability_minus_delta_to_third_liq_warn: Percent::from_percent(2),
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
    pub fn store(self, app: &mut MockApp) -> u64 {
        app.store_code(self.contract_wrapper)
    }

    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut MockApp,
        code_id: Option<u64>,
        addresses: LeaseWrapperAddresses,
        denom: &str,
        config: LeaseWrapperConfig,
    ) -> Addr {
        let code_id = match code_id {
            Some(id) => id,
            None => app.store_code(self.contract_wrapper),
        };

        let downpayment = config.downpayment;
        let msg = Self::lease_instantiate_msg(denom, addresses, config);

        let result = app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[coin(downpayment, denom)],
            "lease",
            None,
        );

        if let Err(error) = result.as_ref() {
            eprintln!("Error: {:?}", error);

            if let Some(source) = error.source() {
                eprintln!("Source Error: {:?}", source);
            }
        }

        result.unwrap()
    }

    fn lease_instantiate_msg(
        denom: &str,
        addresses: LeaseWrapperAddresses,
        config: LeaseWrapperConfig,
    ) -> NewLeaseForm {
        NewLeaseForm {
            customer: config.customer,
            currency: denom.to_string(),
            liability: Liability::new(
                config.liability_init_percent,
                config.liability_delta_to_healthy_percent,
                config.liability_delta_to_max_percent,
                config.liability_minus_delta_to_first_liq_warn,
                config.liability_minus_delta_to_second_liq_warn,
                config.liability_minus_delta_to_third_liq_warn,
                config.liability_recalc_hours,
            ),
            loan: LoanForm {
                annual_margin_interest: config.annual_margin_interest,
                lpp: addresses.lpp.into_string(),
                interest_due_period_secs: config.interest_due_period_secs,
                grace_period_secs: config.grace_period_secs,
                profit: addresses.profit.into_string(),
            },
            time_alarms: addresses.time_alarms,
            market_price_oracle: addresses.oracle,
        }
    }
}

impl Default for LeaseWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LeaseWrapperAddresses {
    pub lpp: Addr,
    pub time_alarms: Addr,
    pub oracle: Addr,
    pub profit: Addr,
}

type LeaseContractWrapperReply = Box<
    ContractWrapper<
        ExecuteMsg,
        ContractError,
        NewLeaseForm,
        ContractError,
        StateQuery,
        ContractError,
        cosmwasm_std::Empty,
        anyhow::Error,
        ContractError,
    >,
>;
