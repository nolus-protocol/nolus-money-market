use std::{
    borrow::Borrow,
    collections::{
        btree_map::{self, BTreeMap},
        btree_set::{self, BTreeSet},
    },
    ops::ControlFlow,
};

use anyhow::{anyhow, Context as _, Result};

use topology::{swap_pairs::PairTargets, Topology};

use crate::protocol::Protocol;

pub(crate) struct CurrenciesTree<'parents_of, 'parent, 'children_of, 'child> {
    parents: BTreeMap<&'parents_of str, BTreeSet<&'parent str>>,
    children: BTreeMap<&'children_of str, BTreeSet<&'child str>>,
}

impl<'topology> CurrenciesTree<'topology, 'topology, 'topology, 'topology> {
    pub fn new(
        topology: &'topology Topology,
        protocol: &Protocol,
        host_currency_ticker: &str,
    ) -> Result<Self> {
        let result = topology
            .network_dexes(&protocol.dex_network)
            .context("Selected DEX network doesn't define any DEXes!")?
            .get(&protocol.dex)
            .context("Selected DEX network doesn't define such DEX!")?
            .swap_pairs()
            .iter()
            .map(|(ticker, targets)| (ticker.borrow(), targets))
            .filter(|&(ticker, _)| {
                super::filter_selected_currencies(protocol, host_currency_ticker, ticker)
            })
            .try_fold(
                const {
                    Self {
                        parents: const { BTreeMap::<_, BTreeSet<_>>::new() },
                        children: const { BTreeMap::<_, BTreeSet<_>>::new() },
                    }
                },
                |currencies_tree, (ticker, targets)| {
                    currencies_tree.process_targets(protocol, host_currency_ticker, ticker, targets)
                },
            );

        match result {
            ControlFlow::Continue(currencies_tree) => Ok(currencies_tree),
            ControlFlow::Break(()) => Err(anyhow!(
                "Currency ticker duplication detected in swap pairs!"
            )),
        }
    }

    fn process_targets(
        mut self,
        protocol: &Protocol,
        host_currency_ticker: &str,
        ticker: &'topology str,
        targets: &'topology PairTargets,
    ) -> ControlFlow<(), Self> {
        let btree_map::Entry::Vacant(entry) = self.children.entry(ticker) else {
            return ControlFlow::Break(());
        };

        let inverse_targets = entry.insert(
            targets
                .iter()
                .map(Borrow::<str>::borrow)
                .filter(|&ticker| {
                    super::filter_selected_currencies(protocol, host_currency_ticker, ticker)
                })
                .collect(),
        );

        let result = inverse_targets
            .iter()
            .try_fold(self.parents, |mut parents, target| {
                if parents.entry(target).or_default().insert(ticker) {
                    ControlFlow::Continue(parents)
                } else {
                    ControlFlow::Break(())
                }
            });

        match result {
            ControlFlow::Continue(parents) => ControlFlow::Continue(Self { parents, ..self }),
            ControlFlow::Break(()) => ControlFlow::Break(()),
        }
    }
}

impl<'parent, 'child> CurrenciesTree<'_, 'parent, '_, 'child> {
    pub fn parents<'r>(&'r self, ticker: &str) -> Parents<'r, 'parent> {
        Parents(
            self.parents
                .get(ticker)
                .unwrap_or(const { &BTreeSet::new() }),
        )
    }

    pub fn children<'r>(&'r self, ticker: &str) -> Children<'r, 'child> {
        Children(
            self.children
                .get(ticker)
                .unwrap_or(const { &BTreeSet::new() }),
        )
    }
}

pub(crate) struct Parents<'r, 'parent>(&'r BTreeSet<&'parent str>);

impl<'r, 'parent> Parents<'r, 'parent> {
    #[inline]
    pub fn iter(&self) -> btree_set::Iter<'r, &'parent str> {
        self.0.iter()
    }
}

impl<'r, 'parent> AsRef<BTreeSet<&'parent str>> for Parents<'r, 'parent> {
    #[inline]
    fn as_ref(&self) -> &BTreeSet<&'parent str> {
        self.0
    }
}

pub(crate) struct Children<'r, 'child>(&'r BTreeSet<&'child str>);

impl<'r, 'child> Children<'r, 'child> {
    #[inline]
    pub fn iter(&self) -> btree_set::Iter<'r, &'child str> {
        self.0.iter()
    }
}

impl<'r, 'child> AsRef<BTreeSet<&'child str>> for Children<'r, 'child> {
    #[inline]
    fn as_ref(&self) -> &BTreeSet<&'child str> {
        self.0
    }
}
