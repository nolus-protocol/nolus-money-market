use cosmwasm_std::Addr;
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
    pub lease_interest_rate_margin: u8,
    pub lease_max_liability: u8,
    pub lease_healthy_liability: u8,
    pub lease_initial_liability: u8,
    pub repayment_period_sec: u32,
    pub grace_period_sec: u32,
}

impl Config {
    pub fn new(sender: Addr, msg: InstantiateMsg) -> Result<Self, ContractError> {
        Ok(Config {
            owner: sender,
            lease_code_id: msg.lease_code_id,
            lpp_ust_addr: msg.lpp_ust_addr,
            lease_interest_rate_margin: msg.lease_interest_rate_margin,
            lease_max_liability: msg.liability.max,
            lease_healthy_liability: msg.liability.healthy,
            lease_initial_liability: msg.liability.initial,
            repayment_period_sec: msg.repayment.period_sec,
            grace_period_sec: msg.repayment.grace_period_sec,
        })
    }
    pub fn update_from(&mut self, msg: UpdateConfigMsg) -> Result<(), ContractError> {
        self.lease_interest_rate_margin = msg.lease_interest_rate_margin;
        self.lease_max_liability = msg.lease_max_liability;
        self.lease_healthy_liability = msg.lease_healthy_liability;
        self.lease_initial_liability = msg.lease_healthy_liability;
        self.repayment_period_sec = msg.repayment_period_sec;
        self.grace_period_sec = msg.grace_period_sec;
        Ok(())
    }
}
