use std::{
    collections::{BTreeMap, BTreeSet},
    ops::ControlFlow,
};

use anyhow::{anyhow, Context as _, Result};

use topology::Topology;

use crate::protocol::Protocol;

pub(crate) struct CurrenciesTree<'parent_map, 'parent, 'children_map, 'child> {
    parents: BTreeMap<&'parent_map str, BTreeSet<&'parent str>>,
    children: BTreeMap<&'children_map str, BTreeSet<&'child str>>,
}

impl<'parent_map, 'parent, 'children_map, 'child>
    CurrenciesTree<'parent_map, 'parent, 'children_map, 'child>
{
    pub fn new<'r>(
        topology: &'r Topology,
        protocol: &Protocol,
    ) -> Result<CurrenciesTree<'r, 'r, 'r, 'r>> {
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

    pub fn parents<'r>(&'r self, ticker: &str) -> &'r BTreeSet<&'parent str> {
        self.parents
            .get(ticker)
            .unwrap_or(const { &BTreeSet::new() })
    }

    pub fn children<'r>(&'r self, ticker: &str) -> &'r BTreeSet<&'child str> {
        self.children
            .get(ticker)
            .unwrap_or(const { &BTreeSet::new() })
    }
}
