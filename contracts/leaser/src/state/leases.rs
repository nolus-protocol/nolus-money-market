use std::collections::{hash_set::IntoIter, HashSet};

use sdk::{
    cosmwasm_std::{Addr, StdError, StdResult, Storage},
    cw_storage_plus::{Bound, Item, Map},
};

use crate::{
    error::ContractError,
    migrate::{Customer, MaybeCustomer},
};

const IDS: InstantiateReplyIdSeq<'static> = InstantiateReplyIdSeq::new("instantiate_reply_ids");

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
    const PENDING: Map<'static, InstantiateReplyId, Addr> = Map::new("pending_instance_creations");
    const CUSTOMER_LEASES: Map<'static, Addr, HashSet<Addr>> = Map::new("loans");

    pub fn cache_open_req(
        storage: &mut dyn Storage,
        customer: &Addr,
    ) -> Result<InstantiateReplyId, ContractError> {
        let instance_reply_id = IDS.next(storage)?;

        Self::PENDING.save(storage, instance_reply_id, customer)?;

        Ok(instance_reply_id)
    }

    /// Return an error if the same lease exists
    pub fn save(storage: &mut dyn Storage, msg_id: u64, lease: Addr) -> StdResult<()> {
        let customer = Self::PENDING.load(storage, msg_id)?;
        Self::PENDING.remove(storage, msg_id);

        let update_fn = |may_leases: Option<HashSet<Addr>>| -> StdResult<HashSet<Addr>> {
            let mut leases = may_leases.unwrap_or_default();
            if leases.insert(lease) {
                Ok(leases)
            } else {
                Err(StdError::generic_err("the lease already exists"))
            }
        };
        Self::CUSTOMER_LEASES
            .update(storage, customer, update_fn)
            .map(|_| ())
    }

    pub fn load_by_customer(storage: &dyn Storage, customer: Addr) -> StdResult<HashSet<Addr>> {
        Self::CUSTOMER_LEASES
            .may_load(storage, customer)
            .map(Option::unwrap_or_default)
    }

    /// Return whether the lease was present before the removal
    pub fn remove(storage: &mut dyn Storage, customer: Addr, lease: &Addr) -> StdResult<bool> {
        let mut removed = false;
        let update_fn = |may_leases: Option<HashSet<Addr>>| -> StdResult<HashSet<Addr>> {
            let mut leases = may_leases.unwrap_or_default();
            removed = leases.remove(lease);
            Ok(leases)
        };

        Self::CUSTOMER_LEASES
            .update(storage, customer, update_fn)
            .map(|_| removed)
    }

    pub fn iter(
        storage: &dyn Storage,
        next_customer: Option<Addr>,
    ) -> impl Iterator<Item = MaybeCustomer<IntoIter<Addr>>> + '_ {
        let start_bound = next_customer.map(Bound::<Addr>::inclusive);
        Self::CUSTOMER_LEASES
            .prefix(())
            .range(storage, start_bound, None, cosmwasm_std::Order::Ascending)
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
        cosmwasm_std::{
            testing::{self, MockStorage},
            Addr, StdError, Storage,
        },
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

        let id = Leases::cache_open_req(&mut deps.storage, &Addr::unchecked("test")).unwrap();
        assert_eq!(id, InstantiateReplyId::MAX);

        // overflow
        let id = Leases::cache_open_req(&mut deps.storage, &Addr::unchecked("test")).unwrap();
        assert_eq!(id, 0);
    }

    #[test]
    fn test_save_customer_not_cached() {
        let mut storage = MockStorage::default();
        assert!(matches!(
            Leases::save(&mut storage, 1, test_lease(),),
            Err(StdError::NotFound { .. })
        ));
        assert_lease_not_exist(&storage);
    }

    #[test]
    fn test_save_first_lease() {
        let mut storage = MockStorage::default();
        assert_lease_not_exist(&storage);
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();

        assert_eq!(Ok(()), Leases::save(&mut storage, 1, test_lease()));
        assert_lease_exist(&storage);
    }

    #[test]
    fn test_save_same_lease() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(Ok(()), Leases::save(&mut storage, 1, test_lease()));
        assert_lease_exist(&storage);

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert!(matches!(
            Leases::save(&mut storage, 2, test_lease()),
            Err(StdError::GenericErr { .. })
        ));
        assert_lease_exist(&storage);
    }

    #[test]
    fn test_save_another_lease() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(Ok(()), Leases::save(&mut storage, 1, test_lease()));
        assert_lease_exist(&storage);

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(Ok(()), Leases::save(&mut storage, 2, test_another_lease()));
        assert_lease_exist(&storage);
        assert!(lease_exist(&storage, &test_another_lease()));
    }

    #[test]
    fn test_remove_not_exist() {
        let mut storage = MockStorage::default();
        assert_lease_not_exist(&storage);
        assert_eq!(
            Ok(false),
            Leases::remove(
                &mut storage,
                Addr::unchecked("customer"),
                &Addr::unchecked("lease1"),
            )
        );
    }

    #[test]
    fn test_remove_exist() {
        let mut storage = MockStorage::default();
        let msg_id = 1;

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        Leases::save(&mut storage, msg_id, test_lease()).unwrap();
        assert_lease_exist(&storage);

        assert_eq!(
            Ok(true),
            Leases::remove(&mut storage, test_customer(), &test_lease(),)
        );
        assert_lease_not_exist(&storage);
    }

    fn test_customer() -> Addr {
        const CUSTOMER: &str = "customerX";
        Addr::unchecked(CUSTOMER)
    }

    fn test_lease() -> Addr {
        const LEASE: &str = "lease1";
        Addr::unchecked(LEASE)
    }

    fn test_another_lease() -> Addr {
        const LEASE: &str = "lease2";
        Addr::unchecked(LEASE)
    }

    #[track_caller]
    fn assert_lease_exist(storage: &dyn Storage) {
        assert!(lease_exist(storage, &test_lease()));
    }

    #[track_caller]
    fn assert_lease_not_exist(storage: &dyn Storage) {
        assert!(!lease_exist(storage, &test_lease()));
    }

    fn lease_exist(storage: &dyn Storage, lease: &Addr) -> bool {
        Leases::load_by_customer(storage, test_customer())
            .unwrap()
            .contains(lease)
    }
}
