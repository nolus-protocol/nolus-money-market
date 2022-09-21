use std::collections::HashSet;

use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

use crate::ContractError;

const IDS: InstantiateReplyIdSeq = InstantiateReplyIdSeq::new("instantiate_reply_ids");
const PENDING: Map<InstantiateReplyId, Addr> = Map::new("pending_instance_creations");

pub type InstantiateReplyId = u64;
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

pub struct Loans {}

impl Loans {
    const STORAGE: Map<'static, Addr, HashSet<Addr>> = Map::new("loans");

    pub fn next(
        storage: &mut dyn Storage,
        sender: Addr,
    ) -> Result<InstantiateReplyId, ContractError> {
        let instance_reply_id = IDS.next(storage)?;

        PENDING.save(storage, instance_reply_id, &sender)?;

        Ok(instance_reply_id)
    }

    pub fn save(storage: &mut dyn Storage, msg_id: u64, lease_addr: Addr) -> StdResult<()> {
        let owner_addr = PENDING.load(storage, msg_id)?;

        // update function for new or existing keys
        let update = |d: Option<HashSet<Addr>>| -> StdResult<HashSet<Addr>> {
            match d {
                Some(mut loans) => {
                    loans.insert(lease_addr);
                    Ok(loans)
                }
                None => {
                    let mut loans = HashSet::new();
                    loans.insert(lease_addr);
                    Ok(loans)
                }
            }
        };

        Self::STORAGE.update(storage, owner_addr, update)?;
        PENDING.remove(storage, msg_id);

        Ok(())
    }

    pub fn get(storage: &dyn Storage, owner_addr: Addr) -> StdResult<HashSet<Addr>> {
        Ok(match Self::STORAGE.load(storage, owner_addr) {
            Ok(loans) => loans,
            Err(_) => HashSet::new(), //return empty list of addresses
        })
    }

    pub fn remove(storage: &mut dyn Storage, msg_id: u64) {
        PENDING.remove(storage, msg_id);
    }
}
