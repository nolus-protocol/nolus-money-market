use std::collections::HashSet;

use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::{error::ContractResult, ContractError};

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

    pub fn iter(storage: &dyn Storage) -> impl Iterator<Item = ContractResult<Addr>> + '_ {
        Self::STORAGE
            .prefix(())
            .range_raw(storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|may_kv| may_kv.map(|kv| kv.1).map_err(Into::into))
            .flat_map(transpose)
    }
}

fn transpose<T, TI, E>(res: Result<TI, E>) -> impl Iterator<Item = Result<T, E>>
where
    TI: IntoIterator<Item = T>,
{
    enum ResultIter<I, E> {
        Ok(I),
        Err(Option<E>),
    }

    impl<I, T, E> Iterator for ResultIter<I, E>
    where
        I: Iterator<Item = T>,
    {
        type Item = Result<I::Item, E>;
        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Ok(i) => i.next().map(Result::Ok),
                Self::Err(e) => e.take().map(Result::Err),
            }
        }
    }

    match res {
        Ok(r) => ResultIter::Ok(r.into_iter()),
        Err(e) => ResultIter::Err(Some(e)),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Addr;
    use sdk::{cosmwasm_std::testing, cw_storage_plus::Item};

    use crate::{
        error::ContractResult,
        state::leases::{InstantiateReplyId, Leases},
    };

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

    #[test]
    fn transpose_ok() {
        let items = [Addr::unchecked("1"), Addr::unchecked("2")];
        let mut iter = super::transpose(ContractResult::Ok(items.clone()));
        assert_eq!(Some(ContractResult::Ok(items[0].clone())), iter.next());
        assert_eq!(Some(ContractResult::Ok(items[1].clone())), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn transpose_err() {
        let cause = access_control::Unauthorized;
        let input: ContractResult<[Addr; 0]> = ContractResult::Err(cause.into());
        let exp: ContractResult<Addr> = ContractResult::Err(cause.into());
        let mut iter = super::transpose::<Addr, [Addr; 0], _>(input);
        assert_eq!(Some(exp), iter.next());
        assert_eq!(None, iter.next());
    }
}
