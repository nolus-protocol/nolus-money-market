use std::{
    collections::{btree_set, BTreeMap, BTreeSet},
    ops::ControlFlow,
};

use anyhow::{anyhow, Context as _, Result};

use topology::Topology;

use crate::protocol::Protocol;

pub(crate) struct CurrenciesTree<'parents_of, 'parent, 'children_of, 'child> {
    parents: BTreeMap<&'parents_of str, BTreeSet<&'parent str>>,
    children: BTreeMap<&'children_of str, BTreeSet<&'child str>>,
}

impl<'topology> CurrenciesTree<'topology, 'topology, 'topology, 'topology> {
    pub fn new(topology: &'topology Topology, protocol: &Protocol) -> Result<Self> {
        let result = topology
            .network_dexes(&protocol.dex_network)
            .context("Selected DEX network doesn't define any DEXes!")?
            .get(&protocol.dex)
            .context("Selected DEX network doesn't define such DEX!")?
            .swap_pairs()
            .iter()
            .try_fold(
                const {
                    CurrenciesTree {
                        parents: const { BTreeMap::<_, BTreeSet<_>>::new() },
                        children: const { BTreeMap::<_, BTreeSet<_>>::new() },
                    }
                },
                |CurrenciesTree {
                     mut parents,
                     mut children,
                 },
                 (from, targets)| {
                    if children
                        .insert(from.as_ref(), targets.iter().map(AsRef::as_ref).collect())
                        .is_some()
                    {
                        ControlFlow::Break(())
                    } else {
                        let result = targets.iter().map(AsRef::as_ref).try_for_each(|target| {
                            if parents.entry(target).or_default().insert(from.as_ref()) {
                                ControlFlow::Continue(())
                            } else {
                                ControlFlow::Break(())
                            }
                        });

                        match result {
                            ControlFlow::Continue(()) => {
                                ControlFlow::Continue(CurrenciesTree { parents, children })
                            }
                            ControlFlow::Break(()) => ControlFlow::Break(()),
                        }
                    }
                },
            );

        match result {
            ControlFlow::Continue(swap_tree) => Ok(swap_tree),
            ControlFlow::Break(()) => Err(anyhow!(
                "Currency ticker duplication detected in swap pairs!"
            )),
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
