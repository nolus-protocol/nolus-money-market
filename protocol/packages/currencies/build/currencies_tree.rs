use std::{
    borrow::Borrow,
    collections::{
        btree_map::{self, BTreeMap},
        btree_set::{self, BTreeSet},
    },
    marker::PhantomData,
};

use anyhow::Result;

use topology::HostCurrency;

use crate::{protocol::Protocol, swap_pairs::PairTargets};

pub(crate) struct CurrenciesTree<'parents_of, 'parent, 'children_of, 'child> {
    parents: BTreeMap<&'parents_of str, Parents<'parent>>,
    children: BTreeMap<&'children_of str, Children<'child>>,
}

impl<'parents_of, 'parent, 'children_of, 'child>
    CurrenciesTree<'parents_of, 'parent, 'children_of, 'child>
{
    const EMPTY: Self = Self {
        parents: BTreeMap::new(),
        children: BTreeMap::new(),
    };

    pub fn new<'protocol>(
        protocol: &'protocol Protocol,
        host_currency: &HostCurrency,
    ) -> Result<Self>
    where
        'child: 'parents_of,
        'protocol: 'parents_of + 'parent + 'children_of + 'child,
    {
        protocol
            .swap_pairs
            .iter()
            .map(|(ticker, targets)| (ticker.borrow(), targets))
            .filter(|&(ticker, _)| protocol.is_protocol_currency(host_currency, ticker))
            .try_fold(Self::EMPTY, |currencies_tree, (ticker, targets)| {
                currencies_tree.process_targets(protocol, host_currency, ticker, targets)
            })
    }

    fn process_targets<'ticker, 'targets>(
        mut self,
        protocol: &Protocol,
        host_currency: &HostCurrency,
        ticker: &'ticker str,
        targets: &'targets PairTargets,
    ) -> Result<Self>
    where
        'child: 'parents_of,
        'ticker: 'parent + 'children_of,
        'targets: 'child,
    {
        const DUPLICATED_TICKER_ERROR: &str = "Currency ticker duplication detected in swap pairs!";

        let btree_map::Entry::Vacant(entry) = self.children.entry(ticker) else {
            return Err(anyhow::Error::msg(DUPLICATED_TICKER_ERROR));
        };

        entry
            .insert(Children::new(
                targets
                    .iter()
                    .map(Borrow::<str>::borrow)
                    .filter(|&ticker| protocol.is_protocol_currency(host_currency, ticker))
                    .collect(),
            ))
            .iter()
            .try_fold(self.parents, |mut parents, target| {
                if parents
                    .entry(target)
                    .or_insert(const { Parents::new(BTreeSet::new()) })
                    .set
                    .insert(ticker)
                {
                    Ok(parents)
                } else {
                    Err(anyhow::Error::msg(DUPLICATED_TICKER_ERROR))
                }
            })
            .map(|parents| Self { parents, ..self })
    }
}

impl<'parent, 'child> CurrenciesTree<'_, 'parent, '_, 'child> {
    pub fn parents<'r>(&'r self, ticker: &str) -> &'r Parents<'parent> {
        self.parents
            .get(ticker)
            .unwrap_or(const { &Parents::new(BTreeSet::new()) })
    }

    pub fn children<'r>(&'r self, ticker: &str) -> &'r Children<'child> {
        self.children
            .get(ticker)
            .unwrap_or(const { &Children::new(BTreeSet::new()) })
    }
}

pub(crate) struct SetNewtype<'ticker, Marker> {
    set: BTreeSet<&'ticker str>,
    _marker: PhantomData<Marker>,
}

impl<'ticker, Marker> SetNewtype<'ticker, Marker> {
    const fn new(set: BTreeSet<&'ticker str>) -> Self {
        Self {
            set,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn iter(&self) -> btree_set::Iter<'_, &'ticker str> {
        self.set.iter()
    }
}

impl<'ticker, Marker> AsRef<BTreeSet<&'ticker str>> for SetNewtype<'ticker, Marker> {
    #[inline]
    fn as_ref(&self) -> &BTreeSet<&'ticker str> {
        &self.set
    }
}

pub(crate) enum ParentMarker {}

pub(crate) type Parents<'parent> = SetNewtype<'parent, ParentMarker>;

pub(crate) enum ChildMarker {}

pub(crate) type Children<'child> = SetNewtype<'child, ChildMarker>;
