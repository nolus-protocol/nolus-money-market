use crate::{config::Config, ContractError};
use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::{Item, Map};

pub type InstantiateReplyId = u64;

pub const CONFIG: Item<Config> = Item::new("config");
pub const INSTANTIATE_REPLY_IDS: InstantiateReplyIdSeq =
    InstantiateReplyIdSeq::new("instantiate_reply_ids");
pub const PENDING_INSTANCE_CREATIONS: Map<InstantiateReplyId, Addr> =
    Map::new("pending_instance_creations");
pub const LEASES: Map<&Addr, Addr> = Map::new("leases");

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
