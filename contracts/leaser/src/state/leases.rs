use sdk::{
    cosmwasm_std::{Addr, Order, QuerierWrapper, StdResult, Storage},
    cw_storage_plus::{Bound, Item, Map},
};

use crate::{
    error::ContractError,
    migrate::{Customer, MaybeCustomer},
    msg::MaxLeases,
    result::ContractResult,
};

const IDS: InstantiateReplyIdSeq<'static> = InstantiateReplyIdSeq::new("instantiate_reply_ids");
const PENDING: Map<'static, InstantiateReplyId, Addr> = Map::new("pending_instance_creations");

pub type InstantiateReplyId = u64;
pub struct InstantiateReplyIdSeq<'a>(Item<'a, InstantiateReplyId>);

impl<'a> InstantiateReplyIdSeq<'a> {
    pub const fn new(namespace: &'a str) -> Self {
        InstantiateReplyIdSeq(Item::new(namespace))
    }

    pub fn next(&self, store: &mut dyn Storage) -> Result<InstantiateReplyId, ContractError> {
        let next_seq: InstantiateReplyId = self
            .0
            .may_load(store)?
            .map_or(0, |next_seq: InstantiateReplyId| next_seq.wrapping_add(1));

        self.0
            .save(store, &next_seq)
            .map(|()| next_seq)
            .map_err(Into::into)
    }
}

pub struct Leases;

impl Leases {
    // customer to leases
    const STORAGE: Map<'static, Addr, Vec<Addr>> = Map::new("loans");

    pub fn next(
        storage: &mut dyn Storage,
        sender: Addr,
    ) -> Result<InstantiateReplyId, ContractError> {
        let instance_reply_id = IDS.next(storage)?;

        PENDING.save(storage, instance_reply_id, &sender)?;

        Ok(instance_reply_id)
    }

    pub fn save(storage: &mut dyn Storage, msg_id: u64, lease_addr: Addr) -> StdResult<()> {
        let owner_addr: Addr = PENDING.load(storage, msg_id)?;

        Self::STORAGE
            .update(
                storage,
                owner_addr,
                |leases: Option<Vec<Addr>>| -> StdResult<Vec<Addr>> {
                    let mut leases: Vec<Addr> = leases.unwrap_or_default();

                    leases.push(lease_addr);

                    Ok(leases)
                },
            )
            .map(|_| PENDING.remove(storage, msg_id))
    }

    pub fn get(storage: &dyn Storage, owner_addr: Addr) -> StdResult<Vec<Addr>> {
        Self::STORAGE
            .may_load(storage, owner_addr)
            .map(|leases: Option<Vec<Addr>>| leases.unwrap_or_default())
    }

    pub fn iter(
        storage: &dyn Storage,
        next_customer: Option<Addr>,
    ) -> impl Iterator<Item = MaybeCustomer<impl ExactSizeIterator<Item = Addr>>> + '_ {
        let start_bound = next_customer.map(Bound::<Addr>::inclusive);
        Self::STORAGE
            .prefix(())
            .range(storage, start_bound, None, Order::Ascending)
            .map(|record| {
                record
                    .map(|(customer, leases)| Customer::from(customer, leases.into_iter()))
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

#[cfg(test)]
mod test_storage_compatibility {
    use std::collections::HashSet;

    use sdk::{
        cosmwasm_std::{testing::MockStorage, Addr},
        cw_storage_plus::Map,
    };

    use crate::state::leases::Leases;

    #[test]
    fn test_storage_compatibility() {
        // Tests for `HashSet <=> Vec` compatibility.
        let mut storage: MockStorage = MockStorage::new();

        let addr: Addr = Addr::unchecked("c1");

        let data: [Addr; 3] = {
            let mut data: [Addr; 3] = [
                Addr::unchecked("l1"),
                Addr::unchecked("l2"),
                Addr::unchecked("l3"),
            ];

            data.sort();

            data
        };

        Map::<'static, Addr, HashSet<Addr>>::new("loans")
            .save(
                &mut storage,
                addr.clone(),
                &data.clone().into_iter().collect(),
            )
            .unwrap();

        assert_eq!(
            {
                let mut leases: Vec<Addr> = Leases::STORAGE.load(&storage, addr).unwrap();

                leases.sort();

                leases
            }
            .as_slice(),
            &data
        );
    }
}
