use std::{fmt::Debug, marker::PhantomData};

use serde::{Deserialize, Serialize};

use currencies::PaymentGroup;
use currency::{
    never::{self, Never},
    AnyVisitor, AnyVisitorResult, Currency, GroupVisit, SymbolOwned, SymbolSlice, Tickers,
};
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};
use tree::{FindBy as _, NodeRef};

use crate::{
    api::{self, swap::SwapTarget, SwapLeg},
    error::{self, ContractError},
    result::ContractResult,
};

type Tree = tree::Tree<SwapTarget>;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub(crate) struct SupportedPairs<B> {
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

            fn on<C>(self) -> AnyVisitorResult<Self> {
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

    pub fn currencies(&self) -> impl Iterator<Item = api::Currency> + '_ {
        self.tree.iter().map(|node| {
            if let Ok(currency) = currency::Tickers
                .maybe_visit_any::<currencies::Native, _>(
                    &node.value().target,
                    crate::state::supported_pairs::CurrencyVisitor(api::CurrencyGroup::Native),
                )
                .or_else(|_| {
                    Tickers.maybe_visit_any::<currencies::Lpns, _>(
                        &node.value().target,
                        crate::state::supported_pairs::CurrencyVisitor(api::CurrencyGroup::Lpn),
                    )
                })
                .or_else(|_| {
                    Tickers.maybe_visit_any::<currencies::LeaseGroup, _>(
                        &node.value().target,
                        crate::state::supported_pairs::CurrencyVisitor(api::CurrencyGroup::Lease),
                    )
                })
                .or_else(|_| {
                    Tickers.maybe_visit_any::<currencies::PaymentOnlyGroup, _>(
                        &node.value().target,
                        crate::state::supported_pairs::CurrencyVisitor(
                            api::CurrencyGroup::PaymentOnly,
                        ),
                    )
                })
                .map(never::safe_unwrap)
            {
                currency
            } else {
                unreachable!("Groups didn't cover all available currencies!")
            }
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

struct CurrencyVisitor(api::CurrencyGroup);

impl AnyVisitor for CurrencyVisitor {
    type Output = api::Currency;

    type Error = Never;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        Ok(api::Currency::new::<C>(self.0))
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use currencies::test::{LeaseC1, LeaseC2, NativeC, StableC};
    use currency::Currency;
    use sdk::cosmwasm_std::{self, testing};
    use tree::HumanReadableTree;

    use super::*;

    type TheCurrency = StableC;

    fn test_case() -> HumanReadableTree<SwapTarget> {
        let base = TheCurrency::TICKER;

        cosmwasm_std::from_json(format!(
            r#"
            {{
                "value":[0,"{base}"],
                "children":[
                    {{
                        "value":[4,"token4"],
                        "children":[
                            {{"value":[3,"token3"]}}
                        ]
                    }},
                    {{
                        "value":[2,"token2"],
                        "children":[
                            {{
                                "value":[1,"token1"],
                                "children":[
                                    {{"value":[5,"token5"]}},
                                    {{"value":[6,"token6"]}}
                                ]
                            }}
                        ]
                    }}
                ]
            }}"#
        ))
        .unwrap()
    }

    #[test]
    fn test_storage() {
        let tree = test_case();
        let sp = SupportedPairs::<StableC>::new(tree.into_tree()).unwrap();
        let mut deps = testing::mock_dependencies();

        sp.save(deps.as_mut().storage).unwrap();
        let restored = SupportedPairs::load(deps.as_ref().storage).unwrap();

        assert_eq!(restored, sp);
    }

    #[test]
    #[should_panic]
    fn test_invalid_base() {
        let tree = cosmwasm_std::from_json(
            r#"{"value":[0,"invalid"],"children":[{"value":[1,"token1"]}]}"#,
        )
        .unwrap();

        SupportedPairs::<TheCurrency>::new(tree).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_duplicated_nodes() {
        let tree = cosmwasm_std::from_json(format!(
            r#"{{
                "value":[0,"{ticker}"],
                "children":[
                    {{"value":[1,"token1"]}},
                    {{
                        "value":[2,"token2"],
                        "children":[
                            {{"value":[1,"token1"]}}
                        ]
                    }}
                ]
            }}"#,
            ticker = TheCurrency::TICKER,
        ))
        .unwrap();

        SupportedPairs::<TheCurrency>::new(tree).unwrap();
    }

    #[test]
    fn test_load_path() {
        let tree = SupportedPairs::<StableC>::new(test_case().into_tree()).unwrap();

        let resp: Vec<_> = tree.load_path("token5").unwrap().collect();
        assert_eq!(
            resp,
            vec![
                "token5".to_string(),
                "token1".to_string(),
                "token2".to_string(),
                TheCurrency::TICKER.to_string(),
            ]
        );
    }

    #[test]
    fn test_load_swap_path() {
        let tree = SupportedPairs::<StableC>::new(test_case().into_tree()).unwrap();

        assert!(tree.load_swap_path("token5", "token5").unwrap().is_empty());

        let resp = tree.load_swap_path("token5", TheCurrency::TICKER).unwrap();
        let expect = vec![
            SwapTarget {
                pool_id: 5,
                target: "token1".into(),
            },
            SwapTarget {
                pool_id: 1,
                target: "token2".into(),
            },
            SwapTarget {
                pool_id: 2,
                target: TheCurrency::TICKER.into(),
            },
        ];

        assert_eq!(resp, expect);

        let resp = tree.load_swap_path("token6", "token5").unwrap();
        let expect = vec![
            SwapTarget {
                pool_id: 6,
                target: "token1".into(),
            },
            SwapTarget {
                pool_id: 5,
                target: "token5".into(),
            },
        ];
        assert_eq!(resp, expect);

        let resp = tree.load_swap_path("token2", "token4").unwrap();
        let expect = vec![
            SwapTarget {
                pool_id: 2,
                target: TheCurrency::TICKER.into(),
            },
            SwapTarget {
                pool_id: 4,
                target: "token4".into(),
            },
        ];
        assert_eq!(resp, expect);
    }

    #[test]
    fn test_query_supported_pairs() {
        let paths = test_case();
        let tree = SupportedPairs::<StableC>::new(paths.into_tree()).unwrap();

        fn leg_cmp(a: &SwapLeg, b: &SwapLeg) -> Ordering {
            a.from.cmp(&b.from)
        }

        let mut response: Vec<_> = tree.swap_pairs_df().collect();
        response.sort_by(leg_cmp);

        let mut expected = vec![
            SwapLeg {
                from: "token2".into(),
                to: SwapTarget {
                    pool_id: 2,
                    target: TheCurrency::TICKER.into(),
                },
            },
            SwapLeg {
                from: "token4".into(),
                to: SwapTarget {
                    pool_id: 4,
                    target: TheCurrency::TICKER.into(),
                },
            },
            SwapLeg {
                from: "token1".into(),
                to: SwapTarget {
                    pool_id: 1,
                    target: "token2".into(),
                },
            },
            SwapLeg {
                from: "token6".into(),
                to: SwapTarget {
                    pool_id: 6,
                    target: "token1".into(),
                },
            },
            SwapLeg {
                from: "token5".into(),
                to: SwapTarget {
                    pool_id: 5,
                    target: "token1".into(),
                },
            },
            SwapLeg {
                from: "token3".into(),
                to: SwapTarget {
                    pool_id: 3,
                    target: "token4".into(),
                },
            },
        ];
        expected.sort_by(leg_cmp);

        assert_eq!(response, expected);
    }

    #[test]
    fn currencies() {
        let listed_currencies: Vec<_> = SupportedPairs::<StableC>::new(
            cosmwasm_std::from_json::<HumanReadableTree<_>>(format!(
                r#"{{
                    "value":[0,{0:?}],
                    "children":[
                        {{
                            "value":[1,{1:?}],
                            "children":[
                                {{"value":[2,{2:?}]}}
                            ]
                        }},
                        {{"value":[3,{3:?}]}}
                    ]
                }}"#,
                <StableC as Currency>::TICKER,
                <LeaseC1 as Currency>::TICKER,
                <NativeC as Currency>::TICKER,
                <LeaseC2 as Currency>::TICKER,
            ))
            .unwrap()
            .into_tree(),
        )
        .unwrap()
        .currencies()
        .collect();

        assert_eq!(
            listed_currencies.as_slice(),
            &[
                api::Currency::new::<StableC>(api::CurrencyGroup::Lpn),
                api::Currency::new::<LeaseC1>(api::CurrencyGroup::Lease),
                api::Currency::new::<NativeC>(api::CurrencyGroup::Native),
                api::Currency::new::<LeaseC2>(api::CurrencyGroup::Lease),
            ]
        );
    }
}
