use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal256, Storage, Uint256};
use cw_storage_plus::{Item, Map};
use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: Addr,
}

pub type InstantiateReplyId = u64;

pub const CONFIG: Item<Config> = Item::new("config");
pub const INSTANTIATE_REPLY_IDS: InstantiateReplyIdSeq = InstantiateReplyIdSeq::new("instantiate_reply_ids");
pub const PENDING_INSTANCE_CREATIONS: Map<InstantiateReplyId, Addr> = Map::new("pending_instance_creations");
pub const LOANS: Map<&Addr, Addr> = Map::new("loans");


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub loan_code_id: u64,
    pub lpp_ust_addr: Addr,
    pub loan_interest_rate_margin: Decimal256,
    pub loan_max_liability: Decimal256,
    pub loan_healthy_liability: Decimal256,
    pub repayment_period_nano_sec: Uint256,
    pub grace_period_nano_sec: Uint256,
}

pub struct InstantiateReplyIdSeq<'a>(Item<'a, InstantiateReplyId>);

impl<'a> InstantiateReplyIdSeq<'a> {
    pub const fn new(namespace: &'a str) -> InstantiateReplyIdSeq {
        InstantiateReplyIdSeq(Item::new(namespace))
    }

    pub fn next(&self, store: &mut dyn Storage) -> Result<InstantiateReplyId, ContractError> {
        let mut next_seq = self.0.load(store).unwrap_or(0);
        next_seq += 1;
        self.0.save(store, &next_seq)?;
        Ok(next_seq)
    }
}
