use std::{fmt::Debug, marker::PhantomData};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currencies::PaymentGroup;
use currency::{
    AnyVisitor, AnyVisitorResult, Currency, GroupVisit, SymbolOwned, SymbolSlice, Tickers,
};
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};
use swap::SwapTarget;
use tree::{FindBy as _, NodeRef};

use crate::{
    error::{self, ContractError},
    result::ContractResult,
};

use super::{CurrencyPair, SwapLeg};

type Tree = tree::Tree<SwapTarget>;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct SupportedPairs<B>
where
    B: Currency,
{
    tree: Tree,
    _type: PhantomData<B>,
}

impl<'a, B> SupportedPairs<B>
where
    B: Currency,
{
    const DB_ITEM: Item<'a, SupportedPairs<B>> = Item::new("supported_pairs");

    pub fn new(tree: Tree) -> Result<Self, ContractError> {
        if tree.root().value().target != B::TICKER {
            return Err(ContractError::InvalidBaseCurrency(
                tree.root().value().target.clone(),
                ToOwned::to_owned(B::TICKER),
            ));
        }

        // check for duplicated nodes
        let mut supported_currencies: Vec<&SymbolOwned> =
            tree.iter().map(|node| &node.value().target).collect();

        supported_currencies.sort();

        if (0..supported_currencies.len() - 1)
            .any(|index| supported_currencies[index] == supported_currencies[index + 1])
        {
            return Err(ContractError::DuplicatedNodes {});
        }

        Ok(SupportedPairs {
            tree,
            _type: PhantomData,
        })
    }

    pub fn validate_tickers(&self) -> Result<&Self, ContractError> {
        struct TickerChecker;

        impl AnyVisitor for TickerChecker {
            type Output = ();
            type Error = ContractError;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency + Serialize + DeserializeOwned + 'static,
            {
                Ok(())
            }
        }

        for swap in self.tree.iter() {
            Tickers.visit_any::<PaymentGroup, _>(&swap.value().target, TickerChecker)?;
        }

        Ok(self)
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        Self::DB_ITEM
            .load(storage)
            .map_err(ContractError::LoadSupportedPairs)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        Self::DB_ITEM
            .save(storage, self)
            .map_err(ContractError::StoreSupportedPairs)
    }

    pub fn load_path(
        &self,
        query: &SymbolSlice,
    ) -> Result<impl Iterator<Item = &SymbolSlice> + DoubleEndedIterator + '_, ContractError> {
        self.internal_load_path(query)
            .map(|iter| iter.map(|node| node.value().target.as_str()))
    }

    pub fn load_swap_path(
        &self,
        from: &SymbolSlice,
        to: &SymbolSlice,
    ) -> Result<Vec<SwapTarget>, ContractError> {
        let path_from = self.internal_load_path(from)?;

        let mut path_to: Vec<_> = self.internal_load_path(to)?.collect();

        let mut path = vec![];

        path.extend(
            path_from
                .take_while(|node| {
                    if let Some((index, _)) = path_to
                        .iter()
                        .enumerate()
                        .rfind(|&(_, to_node)| node.value() == to_node.value())
                    {
                        path_to.truncate(index);

                        return false;
                    }

                    true
                })
                .filter_map(|node| {
                    Some(SwapTarget {
                        pool_id: node.value().pool_id,
                        target: node.parent()?.value().target.clone(),
                    })
                }),
        );

        path_to
            .into_iter()
            .rev()
            .for_each(|node| path.push(node.value().clone()));

        Ok(path)
    }

    pub fn load_affected(&self, pair: CurrencyPair<'_>) -> Result<Vec<SymbolOwned>, ContractError> {
        if let Some(node) = self.tree.find_by(|target| target.target == pair.0) {
            if node
                .parent()
                .map_or(false, |parent| parent.value().target == pair.1)
            {
                return Ok(node
                    .to_subtree()
                    .iter()
                    .map(|node| node.value().target.clone())
                    .collect());
            }
        }

        Err(ContractError::InvalidDenomPair(
            ToOwned::to_owned(pair.0),
            ToOwned::to_owned(pair.1),
        ))
    }

    pub fn swap_pairs_df(&self) -> impl Iterator<Item = SwapLeg> + '_ {
        self.tree
            .iter()
            .filter_map(|node: NodeRef<'_, SwapTarget>| {
                let parent: NodeRef<'_, SwapTarget> = node.parent()?;

                let SwapTarget {
                    pool_id,
                    target: child,
                } = node.value().clone();

                Some(SwapLeg {
                    from: child,
                    to: SwapTarget {
                        pool_id,
                        target: parent.value().target.clone(),
                    },
                })
            })
    }

    pub fn query_swap_tree(self) -> Tree {
        self.tree
    }

    fn internal_load_path(
        &self,
        query: &SymbolSlice,
    ) -> Result<
        impl Iterator<Item = NodeRef<'_, SwapTarget>> + DoubleEndedIterator + '_,
        ContractError,
    > {
        self.tree
            .find_by(|target| target.target == query)
            .map(|node| std::iter::once(node).chain(node.parents_iter()))
            .ok_or_else(|| error::unsupported_currency::<B>(query))
    }
}
