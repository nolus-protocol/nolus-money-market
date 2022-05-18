use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    msg::{InstantiateMsg, Liability, Repayment},
    ContractError,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub lease_code_id: u64,
    pub lpp_ust_addr: Addr,
    pub lease_interest_rate_margin: u8,
    pub recalc_hours: u16,
    pub liability: Liability,
    pub repayment: Repayment,
}

impl Config {
    pub fn new(sender: Addr, msg: InstantiateMsg) -> Result<Self, ContractError> {
        Ok(Config {
            owner: sender,
            lease_code_id: msg.lease_code_id.u64(),
            lpp_ust_addr: msg.lpp_ust_addr,
            lease_interest_rate_margin: msg.lease_interest_rate_margin,
            recalc_hours: msg.recalc_hours,
            liability: msg.liability,
            repayment: msg.repayment,
        })
    }
}
