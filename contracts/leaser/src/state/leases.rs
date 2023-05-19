use std::collections::{hash_set::IntoIter, HashSet};

use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::{Bound, Item, Map},
};

use crate::{error::ContractError, migrate::Customer, result::ContractResult};

const IDS: InstantiateReplyIdSeq<'static> = InstantiateReplyIdSeq::new("instantiate_reply_ids");
const PENDING: Map<'static, InstantiateReplyId, Addr> = Map::new("pending_instance_creations");

pub type InstantiateReplyId = u64;
pub struct InstantiateReplyIdSeq<'a>(Item<'a, InstantiateReplyId>);

impl<'a> InstantiateReplyIdSeq<'a> {
    pub const fn new(namespace: &'a str) -> Self {
        InstantiateReplyIdSeq(Item::new(namespace))
    }

    pub fn next(&self, store: &mut dyn Storage) -> Result<InstantiateReplyId, ContractError> {
        let mut next_seq = self.0.load(store).unwrap_or(0);
        next_seq = next_seq.wrapping_add(1);
        self.0.save(store, &next_seq)?;
        Ok(next_seq)
    }
}

pub struct Leases {}

impl Leases {
    // customer to leases
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

    pub fn iter(
        storage: &dyn Storage,
        next_customer: Option<Addr>,
    ) -> impl Iterator<Item = ContractResult<Customer<IntoIter<Addr>>>> + '_ {
        let start_bound = next_customer.map(Bound::<Addr>::inclusive);
        Self::STORAGE
            .prefix(())
            .range(storage, start_bound, None, cosmwasm_std::Order::Ascending)
            .map(|e| {
                e.map(|(customer, leases)| Customer::from(customer, leases.into_iter()))
                    .map_err(Into::into)
            })
    }
}

#[cfg(test)]
mod test {
    use sdk::{
        cosmwasm_std::{testing, Addr},
        cw_storage_plus::Item,
    };

    use crate::state::leases::{InstantiateReplyId, Leases};

    #[test]
    fn test_id_overflow() {
        let mut deps = testing::mock_dependencies();
        let id_item: Item<'_, InstantiateReplyId> = Item::new("instantiate_reply_ids");
        id_item
            .save(&mut deps.storage, &(InstantiateReplyId::MAX - 1))
            .unwrap();

        let id = Leases::next(&mut deps.storage, Addr::unchecked("test")).unwrap();
        assert_eq!(id, InstantiateReplyId::MAX);

        // overflow
        let id = Leases::next(&mut deps.storage, Addr::unchecked("test")).unwrap();
        assert_eq!(id, 0);
    }
}
