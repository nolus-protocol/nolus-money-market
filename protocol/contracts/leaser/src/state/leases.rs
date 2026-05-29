use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Order, StdResult, Storage},
    cw_storage_plus::{Bound, Map},
};

use crate::{
    ContractError,
    customer::{Customer, CustomerLeases},
    result::ContractResult,
};

pub(crate) struct Leases {}

/// Outcome of [`Leases::save`].
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    /// The lease has been added to the customer's set.
    Registered,
    /// The lease was already present in the customer's set. No state
    /// change.
    AlreadyRegistered,
    /// The open request was cancelled before the leaser's instantiate
    /// reply landed (the OpenLease auto-refund batch fires inside the
    /// same cascade). `save` consumes the cancel marker and no-ops.
    Cancelled,
}

/// Per-customer state machine for in-flight opens. Keyed by customer in
/// [`Leases::PENDING_OPENS`]; the open never goes through `CUSTOMER_LEASES`
/// while in either state. The two-arm enum replaces the previous
/// `PENDING_CUSTOMER: Item<Addr>` + `CANCELLED_PENDING: Item<()>` split,
/// keeping all coordination state inside per-customer storage rather than
/// module-level singletons.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PendingState {
    /// `cache_open_req` set this entry; the leaser is expecting the
    /// instantiate reply to register the lease.
    Cached,
    /// `Leases::remove` cancelled the open mid-cascade (the OpenLease
    /// auto-refund finalised the lease before the leaser's reply landed).
    /// The subsequent `Leases::save` consumes this state and no-ops.
    Cancelled,
}

impl Leases {
    const PENDING_OPENS: Map<Addr, PendingState> = Map::new("pending_opens");

    const CUSTOMER_LEASES: Map<Addr, HashSet<Addr>> = Map::new("loans");

    pub fn cache_open_req(storage: &mut dyn Storage, customer: &Addr) -> ContractResult<()> {
        Self::PENDING_OPENS
            .save(storage, customer.clone(), &PendingState::Cached)
            .map_err(ContractError::SavePendingCustomerFailure)
    }

    /// See [`SaveOutcome`]. `Err` only surfaces real failures: a missing
    /// `cache_open_req` upstream, or a storage error.
    pub fn save(storage: &mut dyn Storage, lease: Addr) -> ContractResult<SaveOutcome> {
        Self::take_pending(storage).and_then(|maybe_pending| match maybe_pending {
            Some((customer, PendingState::Cached)) => Self::save_under(storage, customer, lease),
            Some((_customer, PendingState::Cancelled)) => Ok(SaveOutcome::Cancelled),
            None => Err(ContractError::PendingCustomerNotCached),
        })
    }

    /// Read and remove the single in-flight open. Returns the customer +
    /// state if present, `None` if nothing is pending. Singleton-flavoured
    /// usage (only one open at a time) lives in the caller; the map shape
    /// keeps all state per-customer.
    fn take_pending(storage: &mut dyn Storage) -> ContractResult<Option<(Addr, PendingState)>> {
        let first = Self::PENDING_OPENS
            .range(storage, None, None, Order::Ascending)
            .next();
        match first {
            None => Ok(None),
            Some(Err(err)) => Err(ContractError::SaveLeaseFailure(err)),
            Some(Ok((customer, state))) => {
                Self::PENDING_OPENS.remove(storage, customer.clone());
                Ok(Some((customer, state)))
            }
        }
    }

    fn save_under(
        storage: &mut dyn Storage,
        customer: Addr,
        lease: Addr,
    ) -> ContractResult<SaveOutcome> {
        let mut stored = false;
        let update_fn = |may_leases: Option<HashSet<Addr>>| -> StdResult<HashSet<Addr>> {
            let mut leases = may_leases.unwrap_or_default();
            stored = leases.insert(lease.clone());
            Ok(leases)
        };
        Self::CUSTOMER_LEASES
            .update(storage, customer, update_fn)
            .map(|_leases| {
                if stored {
                    SaveOutcome::Registered
                } else {
                    SaveOutcome::AlreadyRegistered
                }
            })
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

    /// Return whether the lease (or its still-pending open request) was
    /// present before the removal.
    ///
    /// The remote-lease open lifecycle may finalise a lease before it has
    /// been moved from [`Self::PENDING_OPENS`] into `CUSTOMER_LEASES` (the
    /// auto-refund batch in `OpenLease::on_remote_lease_callback` fires
    /// inside the same cascade as the leaser's instantiate reply). In that
    /// case the lease is still recorded only as the in-flight open —
    /// flipping the customer's entry from `Cached` to `Cancelled` cancels
    /// it, and the subsequent `Leases::save` reads the `Cancelled` arm.
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
            Self::cancel_pending_if_matches(storage, customer)
        }
    }

    fn cancel_pending_if_matches(
        storage: &mut dyn Storage,
        customer: Addr,
    ) -> ContractResult<bool> {
        Self::PENDING_OPENS
            .may_load(storage, customer.clone())
            .map_err(ContractError::RemoveLeaseFailure)
            .and_then(|maybe_state| match maybe_state {
                Some(PendingState::Cached) => Self::PENDING_OPENS
                    .save(storage, customer, &PendingState::Cancelled)
                    .map(|()| true)
                    .map_err(ContractError::RemoveLeaseFailure),
                Some(PendingState::Cancelled) | None => Ok(false),
            })
    }

    pub fn iter(
        storage: &dyn Storage,
        next_customer: Option<Addr>,
    ) -> impl CustomerLeases + use<'_> {
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
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use sdk::cosmwasm_std::{Addr, Storage, testing::MockStorage};

    use crate::{
        ContractError,
        state::leases::{Leases, SaveOutcome},
    };

    #[test]
    fn test_save_customer_not_cached() {
        let mut storage = MockStorage::default();
        assert!(matches!(
            Leases::save(&mut storage, test_lease()),
            Err(ContractError::PendingCustomerNotCached)
        ));
        assert_lease_not_exist(&storage);
        assert!(Leases::empty(&storage));
    }

    #[test]
    fn test_save_first_lease() {
        let mut storage = MockStorage::default();
        assert_lease_not_exist(&storage);
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();

        assert_eq!(
            SaveOutcome::Registered,
            Leases::save(&mut storage, test_lease()).unwrap()
        );
        assert_lease_exist(&storage);
        assert!(!Leases::empty(&storage));
    }

    #[test]
    fn test_save_same_lease() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(
            SaveOutcome::Registered,
            Leases::save(&mut storage, test_lease()).unwrap()
        );
        assert_lease_exist(&storage);

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(
            SaveOutcome::AlreadyRegistered,
            Leases::save(&mut storage, test_lease()).unwrap()
        );
        assert_lease_exist(&storage);
    }

    #[test]
    fn test_save_another_lease() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert!(Leases::empty(&storage));
        assert_eq!(
            SaveOutcome::Registered,
            Leases::save(&mut storage, test_lease()).unwrap()
        );
        assert_lease_exist(&storage);
        assert!(!Leases::empty(&storage));

        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert_eq!(
            SaveOutcome::Registered,
            Leases::save(&mut storage, test_another_lease()).unwrap()
        );
        assert_lease_exist(&storage);
        assert!(lease_exist(
            &storage,
            test_customer(),
            &test_another_lease()
        ));
        assert!(!Leases::empty(&storage));
    }

    #[test]
    fn test_save_after_pending_cancelled() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        // OpenLease auto-refund cancels the pending open before the
        // leaser's instantiate reply lands.
        assert!(Leases::remove(&mut storage, test_customer(), &test_lease()).unwrap());
        assert_eq!(
            SaveOutcome::Cancelled,
            Leases::save(&mut storage, test_lease()).unwrap()
        );
        assert_lease_not_exist(&storage);
        // The cancel sentinel must be consumed; a follow-up bug-free
        // save must surface `PendingCustomerNotCached` again.
        assert!(matches!(
            Leases::save(&mut storage, test_lease()),
            Err(ContractError::PendingCustomerNotCached)
        ));
    }

    #[test]
    fn test_remove_not_exist() {
        let mut storage = MockStorage::default();
        assert_lease_not_exist(&storage);
        assert!(Leases::empty(&storage));

        assert!(
            !Leases::remove(
                &mut storage,
                Addr::unchecked("customer"),
                &Addr::unchecked("lease1"),
            )
            .unwrap()
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

        assert!(Leases::remove(&mut storage, test_customer(), &test_lease(),).unwrap());
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

        assert!(Leases::remove(&mut storage, test_customer(), &test_lease()).unwrap());
        assert_lease_not_exist(&storage);
        assert!(!Leases::empty(&storage));

        assert!(
            Leases::remove(&mut storage, test_another_customer(), &test_another_lease(),).unwrap()
        );
        assert!(!lease_exist(
            &storage,
            test_another_customer(),
            &test_another_lease()
        ));
        assert!(!Leases::empty(&storage));

        assert!(!Leases::remove(&mut storage, test_customer(), &test_lease(),).unwrap());
        assert!(Leases::remove(&mut storage, test_customer(), &test_another_lease()).unwrap(),);
        assert!(!lease_exist(
            &storage,
            test_customer(),
            &test_another_lease()
        ));

        assert!(Leases::empty(&storage));
    }

    fn test_customer() -> Addr {
        const CUSTOMER: &str = "customerX";
        Addr::unchecked(CUSTOMER)
    }

    fn test_another_customer() -> Addr {
        const CUSTOMER: &str = "customerY";
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
