use std::collections::HashSet;

use sdk::{
    cosmwasm_std::{Addr, Order, StdResult, Storage},
    cw_storage_plus::{Bound, Item, Map},
};

use crate::{
    ContractError,
    customer::{Customer, CustomerLeases},
    result::ContractResult,
};

pub(crate) struct Leases {}

impl Leases {
    const PENDING_CUSTOMER: Item<Addr> = Item::new("pending_customer");

    const CUSTOMER_LEASES: Map<Addr, HashSet<Addr>> = Map::new("loans");

    pub fn cache_open_req(storage: &mut dyn Storage, customer: &Addr) -> ContractResult<()> {
        Self::PENDING_CUSTOMER
            .save(storage, customer)
            .map_err(ContractError::SavePendingCustomerFailure)
    }

    /// Return true if the lease has been stored or false if there has already been the same lease
    pub fn save(storage: &mut dyn Storage, lease: Addr) -> ContractResult<bool> {
        let mut stored = false;

        let update_fn = |may_leases: Option<HashSet<Addr>>| -> StdResult<HashSet<Addr>> {
            let mut leases = may_leases.unwrap_or_default();

            stored = leases.insert(lease);

            Ok(leases)
        };

        Self::PENDING_CUSTOMER
            .load(storage)
            .inspect(|_| Self::PENDING_CUSTOMER.remove(storage))
            .and_then(|customer| Self::CUSTOMER_LEASES.update(storage, customer, update_fn))
            .map(|_| stored)
            .map_err(ContractError::SaveLeaseFailure)
    }

    pub fn load_by_customer(
        storage: &dyn Storage,
        customer: Addr,
    ) -> ContractResult<HashSet<Addr>> {
        Self::CUSTOMER_LEASES
            .may_load(storage, customer)
            .map(Option::unwrap_or_default)
            .map_err(ContractError::LoadLeasesFailure)
    }

    /// Return whether the lease was present before the removal
    pub fn remove(storage: &mut dyn Storage, customer: Addr, lease: &Addr) -> ContractResult<bool> {
        // not using cw_storage_plus::Map::load because it does not differentiate key-not-present
        // from value-cannot-be-deserialized case
        if let Some(value) = storage.get(&Self::CUSTOMER_LEASES.key(customer.clone())) {
            cosmwasm_std::from_json(value)
                .and_then(|mut leases: HashSet<Addr>| {
                    let removed = leases.remove(lease);
                    if leases.is_empty() {
                        Self::CUSTOMER_LEASES.remove(storage, customer);
                        Ok(())
                    } else {
                        Self::CUSTOMER_LEASES.save(storage, customer, &leases)
                    }
                    .map(|()| removed)
                })
                .map_err(ContractError::RemoveLeaseFailure)
        } else {
            Ok(false)
        }
    }

    pub fn iter<'store>(
        storage: &'store dyn Storage,
        next_customer: Option<Addr>,
    ) -> impl CustomerLeases + use<'store> {
        let start_bound = next_customer.map(Bound::<Addr>::inclusive);

        Self::CUSTOMER_LEASES
            .prefix(())
            .range(storage, start_bound, None, Order::Ascending)
            .map(|record| {
                record
                    .map(|(customer, leases)| Customer::from(customer, leases.into_iter()))
                    .map_err(ContractError::IterateLeasesFailure)
            })
    }

    /// Check whether there is no lease
    ///
    /// Equivalent to "there is no client with leases" statement
    pub fn empty(storage: &dyn Storage) -> bool {
        // ExactSizeIterator::is_empty() is not stable yet
        Self::iter(storage, None).next().is_none()
    }

    pub fn migrate_v0_8_12(storage: &mut dyn Storage) -> ContractResult<()> {
        const MAX_BATCH: u8 = u8::MAX;

        let mut next_customer = None;
        loop {
            let may_customers_no_leases = Self::iter(storage, next_customer)
                .filter_map(|maybe_customer| {
                    maybe_customer.map_or_else(
                        |err| Some(Err(err)),
                        |customer| (customer.leases.len() == 0).then(|| Ok(customer.customer)),
                    )
                })
                .take(MAX_BATCH.into())
                .collect::<ContractResult<Vec<Addr>>>();

            next_customer = match may_customers_no_leases {
                Ok(customers_no_leases) => customers_no_leases
                    .into_iter()
                    .inspect(|customer| Self::CUSTOMER_LEASES.remove(storage, customer.clone()))
                    .last(),
                Err(e) => break Err(e),
            };

            if next_customer.is_none() {
                break Ok(());
            }
        }
    }
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use std::collections::HashSet;

    use sdk::cosmwasm_std::{Addr, Storage, testing::MockStorage};

    use crate::{ContractError, state::leases::Leases};

    #[test]
    fn test_save_customer_not_cached() {
        let mut storage = MockStorage::default();
        assert!(matches!(
            Leases::save(&mut storage, test_lease(),),
            Err(ContractError::SaveLeaseFailure { .. })
        ));
        assert_lease_not_exist(&storage);
        assert!(Leases::empty(&storage));
    }

    #[test]
    fn test_save_first_lease() {
        let mut storage = MockStorage::default();
        assert_lease_not_exist(&storage);
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();

        assert_eq!(Ok(true), Leases::save(&mut storage, test_lease()));
        assert_lease_exist(&storage);
        assert!(!Leases::empty(&storage));
    }

    #[test]
    fn test_save_same_lease() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(Ok(true), Leases::save(&mut storage, test_lease()));
        assert_lease_exist(&storage);

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(Ok(false), Leases::save(&mut storage, test_lease()));
        assert_lease_exist(&storage);
    }

    #[test]
    fn test_save_another_lease() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert!(Leases::empty(&storage));
        assert_eq!(Ok(true), Leases::save(&mut storage, test_lease()));
        assert_lease_exist(&storage);
        assert!(!Leases::empty(&storage));

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(Ok(true), Leases::save(&mut storage, test_another_lease()));
        assert_lease_exist(&storage);
        assert!(lease_exist(
            &storage,
            test_customer(),
            &test_another_lease()
        ));
        assert!(!Leases::empty(&storage));
    }

    #[test]
    fn test_remove_not_exist() {
        let mut storage = MockStorage::default();
        assert_lease_not_exist(&storage);
        assert!(Leases::empty(&storage));

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
        assert!(Leases::empty(&storage));

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        Leases::save(&mut storage, test_lease()).unwrap();
        assert_lease_exist(&storage);
        assert!(!Leases::empty(&storage));

        assert_eq!(
            Ok(true),
            Leases::remove(&mut storage, test_customer(), &test_lease(),)
        );
        assert_lease_not_exist(&storage);
        assert!(Leases::empty(&storage));
    }

    #[test]
    fn test_remove_multiple_leases() {
        let mut storage = MockStorage::default();
        assert!(Leases::empty(&storage));

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        Leases::save(&mut storage, test_lease()).unwrap();
        assert_lease_exist(&storage);
        assert!(!Leases::empty(&storage));

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        Leases::save(&mut storage, test_another_lease()).unwrap();
        assert!(lease_exist(
            &storage,
            test_customer(),
            &test_another_lease()
        ));
        assert!(!Leases::empty(&storage));

        Leases::cache_open_req(&mut storage, &test_another_customer()).unwrap();
        Leases::save(&mut storage, test_another_lease()).unwrap();
        assert_another_lease_exist(&storage);
        assert!(!Leases::empty(&storage));

        assert_eq!(
            Ok(true),
            Leases::remove(&mut storage, test_customer(), &test_lease(),)
        );
        assert_lease_not_exist(&storage);
        assert!(!Leases::empty(&storage));

        assert_eq!(
            Ok(true),
            Leases::remove(&mut storage, test_another_customer(), &test_another_lease(),)
        );
        assert!(!lease_exist(
            &storage,
            test_another_customer(),
            &test_another_lease()
        ));
        assert!(!Leases::empty(&storage));

        assert_eq!(
            Ok(false),
            Leases::remove(&mut storage, test_customer(), &test_lease(),)
        );
        assert_eq!(
            Ok(true),
            Leases::remove(&mut storage, test_customer(), &test_another_lease(),)
        );
        assert!(!lease_exist(
            &storage,
            test_customer(),
            &test_another_lease()
        ));

        assert!(Leases::empty(&storage));
    }

    #[test]
    fn test_migration_simple() {
        let mut storage = MockStorage::default();

        Leases::CUSTOMER_LEASES
            .save(&mut storage, test_customer(), &HashSet::default())
            .unwrap();
        assert!(!Leases::empty(&storage));
        assert_lease_not_exist(&storage);
        save_customer_lease(&mut storage, test_another_customer(), test_another_lease());
        assert!(!Leases::empty(&storage));
        assert_another_lease_exist(&storage);

        Leases::migrate_v0_8_12(&mut storage).unwrap();
        assert!(!Leases::empty(&storage));
        assert_another_lease_exist(&storage);

        let mut customers = Leases::iter(&storage, None);
        let first_customer = customers.next().unwrap().unwrap();
        assert_eq!(test_another_customer(), first_customer.customer);
        assert_eq!(
            vec![test_another_lease()],
            first_customer.leases.collect::<Vec<_>>()
        );
        assert!(customers.next().is_none());
    }

    #[test]
    fn test_migration_multipage() {
        let mut storage = MockStorage::default();

        save_empty_customer_leases(&mut storage, 0, 100);

        let customer_1 = test_index_customer(100);
        save_customer_lease(&mut storage, customer_1.clone(), test_lease());
        assert!(!Leases::empty(&storage));
        assert!(lease_exist(&storage, customer_1.clone(), &test_lease()));

        save_empty_customer_leases(&mut storage, 101, 500);
        let customer_2 = test_index_customer(500);
        save_customer_lease(&mut storage, customer_2.clone(), test_another_lease());
        assert!(!Leases::empty(&storage));
        assert!(lease_exist(
            &storage,
            customer_2.clone(),
            &test_another_lease()
        ));

        let last_customer = test_index_customer(700);
        save_empty_customer_leases(&mut storage, 501, 700);
        save_customer_lease(&mut storage, last_customer.clone(), test_lease());

        Leases::migrate_v0_8_12(&mut storage).unwrap();
        assert!(!Leases::empty(&storage));
        assert!(lease_exist(&storage, customer_1.clone(), &test_lease()));
        assert!(lease_exist(
            &storage,
            customer_2.clone(),
            &test_another_lease()
        ));
        assert!(lease_exist(&storage, last_customer.clone(), &test_lease()));

        let mut customers = Leases::iter(&storage, None);
        let first_customer = customers.next().unwrap().unwrap();
        assert_eq!(customer_1, first_customer.customer);
        assert_eq!(
            vec![test_lease()],
            first_customer.leases.collect::<Vec<_>>()
        );
        let second_customer = customers.next().unwrap().unwrap();
        assert_eq!(customer_2, second_customer.customer);
        assert_eq!(
            vec![test_another_lease()],
            second_customer.leases.collect::<Vec<_>>()
        );
        let third_customer = customers.next().unwrap().unwrap();
        assert_eq!(last_customer, third_customer.customer);
        assert_eq!(
            vec![test_lease()],
            third_customer.leases.collect::<Vec<_>>()
        );
        assert!(customers.next().is_none());
    }

    fn save_customer_lease(storage: &mut dyn Storage, customer: Addr, lease: Addr) {
        Leases::CUSTOMER_LEASES
            .save(storage, customer, &HashSet::from_iter([lease]))
            .expect("saving succeeded");
    }

    fn save_empty_customer_leases(storage: &mut dyn Storage, index_from: u32, index_to: u32) {
        for i in index_from..index_to {
            Leases::CUSTOMER_LEASES
                .save(storage, test_index_customer(i), &HashSet::default())
                .expect("saving succeeded");
        }
    }

    fn test_customer() -> Addr {
        const CUSTOMER: &str = "customerX";
        Addr::unchecked(CUSTOMER)
    }

    fn test_another_customer() -> Addr {
        const CUSTOMER: &str = "customerY";
        Addr::unchecked(CUSTOMER)
    }

    fn test_index_customer(index: u32) -> Addr {
        const CUSTOMER: &str = "customer";
        let mut cust = CUSTOMER.to_owned();
        cust.push_str(&index.to_string());
        Addr::unchecked(cust)
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
        assert!(lease_exist(storage, test_customer(), &test_lease()));
    }

    #[track_caller]
    fn assert_lease_not_exist(storage: &dyn Storage) {
        assert!(!lease_exist(storage, test_customer(), &test_lease()));
    }

    #[track_caller]
    fn assert_another_lease_exist(storage: &dyn Storage) {
        assert!(lease_exist(
            storage,
            test_another_customer(),
            &test_another_lease()
        ));
    }

    fn lease_exist(storage: &dyn Storage, customer: Addr, lease: &Addr) -> bool {
        Leases::load_by_customer(storage, customer)
            .expect("Customer leases map should deserialize")
            .contains(lease)
    }
}
