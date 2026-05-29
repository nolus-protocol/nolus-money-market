use std::collections::HashSet;

use serde::{Deserialize, Serialize};

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

/// The single lease open the leaser is coordinating between the customer's
/// `OpenLease` execute and the lease-instantiate reply.
///
/// Held as a singleton because the instantiate reply correlates the new
/// lease back to its open only by "the one open in flight" — it carries the
/// lease address but never the customer. A `Map` keyed by customer cannot
/// satisfy that correlation (the reply has no key to look up), so the
/// single-in-flight invariant must live in the storage shape itself.
///
/// Replaces the earlier `PENDING_CUSTOMER: Item<Addr>` + `CANCELLED_PENDING:
/// Item<()>` pair: a single typed `Item` carrying the customer and the
/// lifecycle phase, rather than a data item plus a bare armed/disarmed gate.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct PendingOpen {
    customer: Addr,
    phase: PendingPhase,
}

/// Lifecycle phase of the in-flight open held in [`Leases::PENDING_OPENS`].
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PendingPhase {
    /// `cache_open_req` set this; the leaser expects the instantiate reply
    /// to register the lease.
    Cached,
    /// `Leases::remove` cancelled the open mid-cascade (the OpenLease
    /// auto-refund finalised the lease before the leaser's reply landed).
    /// The subsequent `Leases::save` consumes this phase and no-ops.
    Cancelled,
}

impl Leases {
    const PENDING_OPENS: Item<PendingOpen> = Item::new("pending_opens");

    const CUSTOMER_LEASES: Map<Addr, HashSet<Addr>> = Map::new("loans");

    /// Record the customer whose lease open is starting.
    ///
    /// Guards the single-in-flight invariant the instantiate-reply correlator
    /// depends on: the open path consumes the entry within the same
    /// transaction (the `reply_on_success` reply runs `take_pending` before
    /// the tx commits), so a non-empty slot here means the invariant was
    /// violated — refuse rather than silently overwrite the prior open.
    pub fn cache_open_req(storage: &mut dyn Storage, customer: &Addr) -> ContractResult<()> {
        Self::PENDING_OPENS
            .may_load(storage)
            .map_err(ContractError::SavePendingCustomerFailure)
            .and_then(|in_flight| match in_flight {
                Some(_) => Err(ContractError::PendingOpenAlreadyInFlight),
                None => Self::PENDING_OPENS
                    .save(
                        storage,
                        &PendingOpen {
                            customer: customer.clone(),
                            phase: PendingPhase::Cached,
                        },
                    )
                    .map_err(ContractError::SavePendingCustomerFailure),
            })
    }

    /// See [`SaveOutcome`]. `Err` only surfaces real failures: a missing
    /// `cache_open_req` upstream, or a storage error.
    pub fn save(storage: &mut dyn Storage, lease: Addr) -> ContractResult<SaveOutcome> {
        Self::take_pending(storage).and_then(|maybe_pending| match maybe_pending {
            Some(PendingOpen {
                customer,
                phase: PendingPhase::Cached,
            }) => Self::save_under(storage, customer, lease),
            Some(PendingOpen {
                phase: PendingPhase::Cancelled,
                ..
            }) => Ok(SaveOutcome::Cancelled),
            None => Err(ContractError::PendingCustomerNotCached),
        })
    }

    /// Read and clear the single in-flight open. Returns it if present,
    /// `None` if nothing is pending. The singleton shape enforces the
    /// single-in-flight invariant the instantiate reply relies on — it
    /// correlates by "the one open in flight", never by customer.
    fn take_pending(storage: &mut dyn Storage) -> ContractResult<Option<PendingOpen>> {
        Self::PENDING_OPENS
            .may_load(storage)
            .inspect(|maybe_pending| {
                if maybe_pending.is_some() {
                    Self::PENDING_OPENS.remove(storage);
                }
            })
            .map_err(ContractError::SaveLeaseFailure)
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
    ///
    /// The cancellation must therefore be reached whenever `lease` is not
    /// one of the customer's *registered* leases — independently of whether
    /// the customer holds other registered leases. A customer with an
    /// existing lease that opens a second one which fails before its reply
    /// lands still has its pending open recorded only in `PENDING_OPENS`.
    pub fn remove(storage: &mut dyn Storage, customer: Addr, lease: &Addr) -> ContractResult<bool> {
        // not using cw_storage_plus::Map::load because it does not differentiate key-not-present
        // from value-cannot-be-deserialized case
        match storage.get(&Self::CUSTOMER_LEASES.key(customer.clone())) {
            Some(value) => cosmwasm_std::from_json(value)
                .map_err(ContractError::RemoveLeaseFailure)
                .and_then(|mut leases: HashSet<Addr>| {
                    if leases.remove(lease) {
                        Self::store_customer_leases(storage, customer, leases).map(|()| true)
                    } else {
                        // `lease` is not registered for this customer — it may
                        // be their in-flight open awaiting the reply.
                        Self::cancel_pending_if_matches(storage, customer)
                    }
                }),
            None => Self::cancel_pending_if_matches(storage, customer),
        }
    }

    /// Persist the customer's lease set after a removal, deleting the entry
    /// entirely once the customer holds no more leases.
    fn store_customer_leases(
        storage: &mut dyn Storage,
        customer: Addr,
        leases: HashSet<Addr>,
    ) -> ContractResult<()> {
        if leases.is_empty() {
            Self::CUSTOMER_LEASES.remove(storage, customer);
            Ok(())
        } else {
            Self::CUSTOMER_LEASES
                .save(storage, customer, &leases)
                .map_err(ContractError::RemoveLeaseFailure)
        }
    }

    fn cancel_pending_if_matches(
        storage: &mut dyn Storage,
        customer: Addr,
    ) -> ContractResult<bool> {
        Self::PENDING_OPENS
            .may_load(storage)
            .map_err(ContractError::RemoveLeaseFailure)
            .and_then(|maybe_pending| match maybe_pending {
                Some(PendingOpen {
                    customer: pending_customer,
                    phase: PendingPhase::Cached,
                }) if pending_customer == customer => Self::PENDING_OPENS
                    .save(
                        storage,
                        &PendingOpen {
                            customer,
                            phase: PendingPhase::Cancelled,
                        },
                    )
                    .map(|()| true)
                    .map_err(ContractError::RemoveLeaseFailure),
                Some(_) | None => Ok(false),
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
    fn test_remove_other_customer_keeps_pending() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        // A removal for a different customer must not touch the in-flight
        // open — the singleton correlates the cancellation to its customer.
        assert!(!Leases::remove(&mut storage, test_another_customer(), &test_lease()).unwrap());
        // The original customer's open is still `Cached` and registers.
        assert_eq!(
            SaveOutcome::Registered,
            Leases::save(&mut storage, test_lease()).unwrap()
        );
        assert_lease_exist(&storage);
    }

    #[test]
    fn test_cache_open_req_rejects_a_second_in_flight_open() {
        let mut storage = MockStorage::default();
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        // A second cache before the first open is consumed must fail loudly
        // rather than silently overwriting the in-flight entry.
        assert!(matches!(
            Leases::cache_open_req(&mut storage, &test_another_customer()),
            Err(ContractError::PendingOpenAlreadyInFlight)
        ));
        // The original open is intact and still registers.
        assert_eq!(
            SaveOutcome::Registered,
            Leases::save(&mut storage, test_lease()).unwrap()
        );
        assert_lease_exist(&storage);
    }

    #[test]
    fn test_remove_pending_open_while_customer_has_a_registered_lease() {
        let mut storage = MockStorage::default();
        // The customer already has one registered lease from a prior open.
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        Leases::save(&mut storage, test_lease()).unwrap();
        assert_lease_exist(&storage);

        // A second open is cached; before its instantiate reply lands, the
        // remote-lease auto-refund finalises it -> remove(customer, lease2).
        // The pending open must be cancelled even though the customer still
        // has another registered lease.
        Leases::cache_open_req(&mut storage, &test_customer()).unwrap();
        assert!(
            Leases::remove(&mut storage, test_customer(), &test_another_lease()).unwrap(),
            "removing the pending open must report it was present and cancel it",
        );

        // The late instantiate reply must read `Cancelled` and no-op rather
        // than registering the already-finalised ghost lease.
        assert_eq!(
            SaveOutcome::Cancelled,
            Leases::save(&mut storage, test_another_lease()).unwrap(),
        );
        assert!(!lease_exist(
            &storage,
            test_customer(),
            &test_another_lease()
        ));
        // The pre-existing lease is untouched.
        assert_lease_exist(&storage);
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
