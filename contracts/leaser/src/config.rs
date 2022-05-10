use cosmwasm_std::{Addr, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    msg::{InstantiateMsg, UpdateConfigMsg},
    ContractError,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub lease_code_id: u64,
    pub lpp_ust_addr: Addr,
    pub lease_interest_rate_margin: Decimal,
    pub lease_max_liability: Decimal,
    pub lease_healthy_liability: Decimal,
    pub lease_initial_liability: Decimal,
    pub repayment_period_sec: u32,
    pub grace_period_sec: u32,
}

impl Config {
    pub fn new(sender: Addr, msg: InstantiateMsg) -> Result<Self, ContractError> {
        Ok(Config {
            owner: sender,
            lease_code_id: msg.lease_code_id,
            lpp_ust_addr: msg.lpp_ust_addr,
            lease_interest_rate_margin: Decimal::percent(msg.lease_interest_rate_margin.into()),
            lease_max_liability: Decimal::percent(msg.lease_max_liability.into()),
            lease_healthy_liability: Config::validate_lease_healthy_liability(
                msg.lease_healthy_liability.into(),
                msg.lease_max_liability.into(),
            )?,
            lease_initial_liability: Config::validate_lease_initial_liability(
                msg.lease_initial_liability.into(),
                msg.lease_healthy_liability.into(),
            )?,
            repayment_period_sec: msg.repayment_period_sec,
            grace_period_sec: msg.grace_period_sec,
        })
    }

    pub fn update_from(&mut self, msg: UpdateConfigMsg) -> Result<(), ContractError> {
        self.lease_interest_rate_margin = Decimal::percent(msg.lease_interest_rate_margin.into());
        self.lease_max_liability = Decimal::percent(msg.lease_max_liability.into());
        self.lease_healthy_liability = Config::validate_lease_healthy_liability(
            msg.lease_healthy_liability.into(),
            msg.lease_max_liability.into(),
        )?;
        self.lease_initial_liability = Config::validate_lease_initial_liability(
            msg.lease_initial_liability.into(),
            msg.lease_healthy_liability.into(),
        )?;
        self.repayment_period_sec = msg.repayment_period_sec;
        self.grace_period_sec = msg.grace_period_sec;
        Ok(())
    }

    fn validate_lease_healthy_liability(
        lease_healthy_liability: u64,
        lease_max_liability: u64,
    ) -> Result<Decimal, ContractError> {
        if lease_healthy_liability < lease_max_liability {
            Ok(Decimal::percent(lease_healthy_liability))
        } else {
            Err(ContractError::ValidationError {
                msg: "LeaseHealthyLiability% must be less than LeaseMaxLiability%".to_string(),
            })
        }
    }

    fn validate_lease_initial_liability(
        lease_initial_liability: u64,
        lease_healthy_liability: u64,
    ) -> Result<Decimal, ContractError> {
        if lease_initial_liability <= lease_healthy_liability {
            Ok(Decimal::percent(lease_initial_liability))
        } else {
            Err(ContractError::ValidationError {
                msg: "LeaseInitialLiability% must be less or equal to LeaseHealthyLiability%"
                    .to_string(),
            })
        }
    }
}
