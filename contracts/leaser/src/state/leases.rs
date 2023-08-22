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

    pub fn purge_closed(
        storage: &mut dyn Storage,
        querier: &QuerierWrapper<'_>,
        max_leases: MaxLeases,
        mut next_key: Option<Addr>,
    ) -> ContractResult<Option<Addr>> {
        let mut max_leases: usize = usize::try_from(max_leases)?;

        while let Some((customer, mut leases)) = {
            let mut entries_iter: Box<dyn Iterator<Item = StdResult<(Addr, Vec<Addr>)>>> =
                Self::STORAGE.range(
                    storage,
                    next_key.take().map(Bound::exclusive),
                    None,
                    Order::Ascending,
                );

            entries_iter.next().transpose()?
        } {
            if max_leases != 0 && Self::retain_opened(&mut leases, querier, &mut max_leases)? {
                let customer: Addr = customer.clone();

                if leases.is_empty() {
                    Self::STORAGE.remove(storage, customer)
                } else {
                    Self::STORAGE.save(storage, customer, &leases)?;
                }
            }

            next_key = Some(customer);

            if max_leases == 0 {
                break;
            }
        }

        Ok(next_key)
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

    fn retain_opened(
        leases: &mut Vec<Addr>,
        querier: &QuerierWrapper<'_>,
        max_leases: &mut usize,
    ) -> StdResult<bool> {
        // Iterating in reverse order to prevent bugs
        // causing out-of-bounds indexing and skipping
        // over elements which haven't been checked.
        let mut iter = (0..leases.len()).rev().take(*max_leases);

        *max_leases = max_leases.saturating_sub(leases.len());

        let mut changed: bool = false;

        iter.try_for_each(|index: usize| -> StdResult<()> {
            querier
                .query_wasm_smart(
                    // Call requires `Into<String>`. Explicit
                    // `clone` call to prevent hidden control-flow.
                    leases[index].clone(),
                    &::lease::api::QueryMsg::IsClosed {},
                )
                .map(|is_closed: bool| {
                    if is_closed {
                        // Safety: Safe to remove element via swap,
                        // because last element replaces it. Iterating
                        // from the last to the first element makes it
                        // safe to continue down, because the last element
                        // is already guaranteed to have been iterated
                        // through.
                        leases.swap_remove(index);

                        changed = true;
                    }
                })
        })
        .map(|()| changed)
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

#[cfg(test)]
mod test_purge_closed {
    use std::iter;

    use sdk::cosmwasm_std::{
        from_binary,
        testing::{MockQuerier, MockStorage},
        to_binary, Addr, QuerierWrapper, WasmQuery,
    };

    use crate::state::leases::Leases;

    const CLOSED_SUFFIX: &str = "_closed";
    const _: () = if CLOSED_SUFFIX.is_empty() {
        panic!()
    };

    fn mock_querier() -> MockQuerier {
        let mut querier: MockQuerier = MockQuerier::new(&[]);

        querier.update_wasm(move |query: &WasmQuery| {
            if let WasmQuery::Smart { contract_addr, msg } = query {
                if let Ok(::lease::api::QueryMsg::IsClosed {}) = from_binary(msg) {
                    return Ok(to_binary(&contract_addr.contains(CLOSED_SUFFIX)).into()).into();
                }
            }

            unimplemented!();
        });

        querier
    }

    fn generate_leases(leases_count: usize, close_every_nth: usize) -> Vec<Addr> {
        let leases: Vec<Addr> = iter::from_fn({
            let mut counter: usize = 0;

            move || -> Option<Addr> {
                Some(Addr::unchecked({
                    counter += 1;

                    let is_closed: bool = counter % close_every_nth == 0;

                    format!(
                        "lease_{counter}{}",
                        if is_closed { CLOSED_SUFFIX } else { "" }
                    )
                }))
            }
        })
        .take(leases_count)
        .collect();

        assert_eq!(leases.len(), leases_count);
        assert_eq!(
            leases
                .iter()
                .filter(|lease: &&Addr| lease.as_str().contains(CLOSED_SUFFIX))
                .count(),
            leases_count / close_every_nth,
        );

        leases
    }

    #[test]
    fn test_retain_opened() {
        const LEASES_COUNT: usize = 30;
        const CLOSE_EVERY_NTH: usize = 3;
        const _: () = if CLOSE_EVERY_NTH == 0 {
            panic!()
        };
        const _: () = if LEASES_COUNT % CLOSE_EVERY_NTH != 0 {
            panic!()
        };

        let querier: MockQuerier = mock_querier();

        let mut leases: Vec<Addr> = generate_leases(LEASES_COUNT, CLOSE_EVERY_NTH);

        let mut max_leases: usize = (LEASES_COUNT / CLOSE_EVERY_NTH) / 2;
        assert_ne!(max_leases, 0);

        {
            let max_leases_shadow: usize = max_leases;

            assert_eq!(
                Leases::retain_opened(&mut leases, &QuerierWrapper::new(&querier), &mut max_leases),
                Ok(true)
            );

            assert_eq!(max_leases, 0);

            assert_eq!(
                leases
                    .iter()
                    .filter(|lease: &&Addr| lease.as_str().contains(CLOSED_SUFFIX))
                    .count(),
                (LEASES_COUNT / CLOSE_EVERY_NTH)
                    - (max_leases_shadow / CLOSE_EVERY_NTH
                        + usize::from(max_leases_shadow % CLOSE_EVERY_NTH != 0)),
            );
        }

        assert_eq!(
            Leases::retain_opened(&mut leases, &QuerierWrapper::new(&querier), &mut max_leases),
            Ok(false)
        );

        max_leases = usize::MAX;

        assert_eq!(
            Leases::retain_opened(&mut leases, &QuerierWrapper::new(&querier), &mut max_leases),
            Ok(true)
        );

        assert_eq!(
            leases.len(),
            LEASES_COUNT * (CLOSE_EVERY_NTH - 1) / CLOSE_EVERY_NTH
        );
        assert!(!leases
            .iter()
            .any(|lease: &Addr| lease.as_str().contains(CLOSED_SUFFIX)));

        assert_eq!(
            Leases::retain_opened(&mut leases, &QuerierWrapper::new(&querier), &mut max_leases),
            Ok(false)
        );

        assert_eq!(
            leases.len(),
            LEASES_COUNT * (CLOSE_EVERY_NTH - 1) / CLOSE_EVERY_NTH
        );
    }

    #[test]
    fn test_purge_closed() {
        /// ({Lease count}, {Some => Close every N-th}, {Expected lease count left after purge})
        const CUSTOMER_LEASES_CONFIG: [(usize, Option<usize>, usize); 3] =
            [(10, None, 10), (20, Some(2), 10), (10, Some(1), 0)];
        const _: () = {
            let mut index: usize = 0;

            while index < CUSTOMER_LEASES_CONFIG.len() {
                if let Some(modulo) = CUSTOMER_LEASES_CONFIG[index].1 {
                    if CUSTOMER_LEASES_CONFIG[index].0 % modulo != 0 {
                        panic!()
                    }
                }

                index += 1;
            }
        };

        let mut storage: MockStorage = MockStorage::new();
        let querier: MockQuerier = mock_querier();
        let querier: QuerierWrapper<'_> = QuerierWrapper::new(&querier);

        for (index, (leases_count, close_every_nth, _)) in
            CUSTOMER_LEASES_CONFIG.into_iter().enumerate()
        {
            Leases::STORAGE
                .save(
                    &mut storage,
                    Addr::unchecked(format!("customer{index}")),
                    &generate_leases(leases_count, close_every_nth.unwrap_or(usize::MAX)),
                )
                .unwrap();
        }

        assert_eq!(
            Leases::purge_closed(&mut storage, &querier, u32::MAX, None),
            Ok(None)
        );

        for (index, (_, _, leases_left)) in CUSTOMER_LEASES_CONFIG.into_iter().enumerate() {
            assert_eq!(
                Leases::STORAGE
                    .may_load(&storage, Addr::unchecked(format!("customer{index}")))
                    .unwrap()
                    .map(|leases: Vec<Addr>| leases.len()),
                if leases_left == 0 {
                    None
                } else {
                    Some(leases_left)
                }
            );
        }
    }
}
