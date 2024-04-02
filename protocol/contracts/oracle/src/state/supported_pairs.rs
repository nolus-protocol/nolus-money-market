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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SupportedPairs<B> {
    tree: Tree,
    stable_currency: SymbolOwned,
    #[serde(skip)]
    _type: PhantomData<B>,
}

impl<'a, B> SupportedPairs<B>
where
    B: Currency,
{
    const DB_ITEM: Item<'a, SupportedPairs<B>> = Item::new("supported_pairs");

    pub fn new(tree: Tree, stable_currency: SymbolOwned) -> Result<Self, ContractError> {
        check_tree(&tree, B::TICKER, &stable_currency).map(|()| SupportedPairs {
            tree,
            stable_currency,
            _type: PhantomData,
        })
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

    pub fn stable_currency(&self) -> &SymbolSlice {
        &self.stable_currency
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

    pub(crate) fn migrate(storage: &mut dyn Storage) -> ContractResult<()> {
        #[derive(Serialize, Deserialize)]
        struct SupportedPairs {
            tree: Tree,
        }

        Item::<'_, SupportedPairs>::new("supported_pairs")
            .load(storage)
            .map_err(ContractError::LoadSupportedPairs)
            .and_then(|SupportedPairs { tree }| {
                Self::DB_ITEM
                    .save(
                        storage,
                        &Self {
                            tree,
                            stable_currency: B::TICKER.into(),
                            _type: PhantomData,
                        },
                    )
                    .map_err(ContractError::StoreSupportedPairs)
            })
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

fn check_tree(
    tree: &Tree,
    base_currency: &SymbolSlice,
    stable_currency: &SymbolSlice,
) -> Result<(), ContractError> {
    if tree.root().value().target != base_currency {
        Err(ContractError::InvalidBaseCurrency(
            tree.root().value().target.clone(),
            ToOwned::to_owned(base_currency),
        ))
    } else {
        let mut supported_currencies: Vec<_> = tree
            .iter()
            .map(|node| node.value().target.as_ref())
            .collect();

        supported_currencies.sort_unstable();

        if supported_currencies
            .binary_search(&stable_currency)
            .is_err()
        {
            Err(ContractError::StableCurrencyNotInTree {})
        } else if supported_currencies
            .windows(2)
            .any(|window| window[0] == window[1])
        {
            Err(ContractError::DuplicatedNodes {})
        } else {
            tree.iter()
                .map(|node| node.value().target.as_ref())
                .try_for_each(currency::validate::<PaymentGroup>)
                .map_err(Into::into)
        }
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

    use currencies::test::{LeaseC1, LeaseC2, LeaseC3, LeaseC4, LeaseC5, NativeC, StableC};
    use currency::Currency;
    use sdk::cosmwasm_std::{self, testing};
    use tree::HumanReadableTree;

    use super::*;

    type TheCurrency = StableC;

    fn test_case() -> HumanReadableTree<SwapTarget> {
        cosmwasm_std::from_json(format!(
            r#"{{
                "value":[0,"{base}"],
                "children":[
                    {{
                        "value":[4,"{lease4}"],
                        "children":[
                            {{"value":[3,"{lease3}"]}}
                        ]
                    }},
                    {{
                        "value":[2,"{lease2}"],
                        "children":[
                            {{
                                "value":[1,"{lease1}"],
                                "children":[
                                    {{"value":[5,"{lease5}"]}},
                                    {{"value":[6,"{native}"]}}
                                ]
                            }}
                        ]
                    }}
                ]
            }}"#,
            base = TheCurrency::TICKER,
            lease1 = LeaseC1::TICKER,
            lease2 = LeaseC2::TICKER,
            lease3 = LeaseC3::TICKER,
            lease4 = LeaseC4::TICKER,
            lease5 = LeaseC5::TICKER,
            native = NativeC::TICKER,
        ))
        .expect("123")
    }

    #[test]
    fn test_storage() {
        let tree = test_case();
        let sp =
            SupportedPairs::<StableC>::new(tree.into_tree(), TheCurrency::TICKER.into()).unwrap();
        let mut deps = testing::mock_dependencies();

        sp.save(deps.as_mut().storage).unwrap();
        let restored = SupportedPairs::load(deps.as_ref().storage).unwrap();

        assert_eq!(restored, sp);
    }

    #[test]
    fn test_invalid_base() {
        let tree: HumanReadableTree<_> = cosmwasm_std::from_json(format!(
            r#"{{
                "value": [0, {lease1:?}],
                "children": [
                    {{
                        "value": [1, {stable:?}]
                    }}
                ]
            }}"#,
            lease1 = LeaseC1::TICKER,
            stable = StableC::TICKER,
        ))
        .unwrap();

        assert_eq!(
            SupportedPairs::<TheCurrency>::new(tree.into_tree(), StableC::TICKER.into()),
            Err(ContractError::InvalidBaseCurrency(
                LeaseC1::TICKER.into(),
                StableC::TICKER.into()
            ))
        );
    }

    #[test]
    fn test_duplicated_nodes() {
        let tree: HumanReadableTree<_> = cosmwasm_std::from_json(format!(
            r#"{{
                "value": [0, {base:?}],
                "children":[
                    {{
                        "value": [1, {lease1:?}]
                    }},
                    {{
                        "value": [2, {lease2:?}],
                        "children": [
                            {{
                                "value": [1, {lease1:?}]
                            }}
                        ]
                    }}
                ]
            }}"#,
            base = TheCurrency::TICKER,
            lease1 = LeaseC1::TICKER,
            lease2 = LeaseC2::TICKER,
        ))
        .unwrap();

        assert_eq!(
            SupportedPairs::<TheCurrency>::new(tree.into_tree(), TheCurrency::TICKER.into()),
            Err(ContractError::DuplicatedNodes {})
        );
    }

    #[test]
    fn test_not_included_stable_currency() {
        let tree: HumanReadableTree<_> = cosmwasm_std::from_json(format!(
            r#"{{
                "value": [0, {base:?}],
                "children":[
                    {{
                        "value": [1, {lease1:?}]
                    }}
                ]
            }}"#,
            base = TheCurrency::TICKER,
            lease1 = LeaseC1::TICKER,
        ))
        .unwrap();

        assert_eq!(
            SupportedPairs::<TheCurrency>::new(tree.into_tree(), NativeC::TICKER.into()),
            Err(ContractError::StableCurrencyNotInTree {})
        );
    }

    #[test]
    fn test_load_path() {
        let tree =
            SupportedPairs::<StableC>::new(test_case().into_tree(), TheCurrency::TICKER.into())
                .unwrap();

        let resp: Vec<_> = tree.load_path(LeaseC5::TICKER).unwrap().collect();
        assert_eq!(
            resp,
            vec![
                LeaseC5::TICKER,
                LeaseC1::TICKER,
                LeaseC2::TICKER,
                TheCurrency::TICKER,
            ]
        );
    }

    #[test]
    fn test_load_swap_path() {
        let tree =
            SupportedPairs::<StableC>::new(test_case().into_tree(), TheCurrency::TICKER.into())
                .unwrap();

        assert!(tree
            .load_swap_path(LeaseC5::TICKER, LeaseC5::TICKER)
            .unwrap()
            .is_empty());

        let resp = tree
            .load_swap_path(LeaseC5::TICKER, TheCurrency::TICKER)
            .unwrap();
        let expect = vec![
            SwapTarget {
                pool_id: 5,
                target: LeaseC1::TICKER.into(),
            },
            SwapTarget {
                pool_id: 1,
                target: LeaseC2::TICKER.into(),
            },
            SwapTarget {
                pool_id: 2,
                target: TheCurrency::TICKER.into(),
            },
        ];

        assert_eq!(resp, expect);

        let resp = tree
            .load_swap_path(NativeC::TICKER, LeaseC5::TICKER)
            .unwrap();
        let expect = vec![
            SwapTarget {
                pool_id: 6,
                target: LeaseC1::TICKER.into(),
            },
            SwapTarget {
                pool_id: 5,
                target: LeaseC5::TICKER.into(),
            },
        ];
        assert_eq!(resp, expect);

        let resp = tree
            .load_swap_path(LeaseC2::TICKER, LeaseC4::TICKER)
            .unwrap();
        let expect = vec![
            SwapTarget {
                pool_id: 2,
                target: TheCurrency::TICKER.into(),
            },
            SwapTarget {
                pool_id: 4,
                target: LeaseC4::TICKER.into(),
            },
        ];
        assert_eq!(resp, expect);
    }

    #[test]
    fn test_query_supported_pairs() {
        let paths = test_case();
        let tree =
            SupportedPairs::<StableC>::new(paths.into_tree(), TheCurrency::TICKER.into()).unwrap();

        fn leg_cmp(a: &SwapLeg, b: &SwapLeg) -> Ordering {
            a.from.cmp(&b.from)
        }

        let mut response: Vec<_> = tree.swap_pairs_df().collect();
        response.sort_by(leg_cmp);

        let mut expected = vec![
            SwapLeg {
                from: LeaseC2::TICKER.into(),
                to: SwapTarget {
                    pool_id: 2,
                    target: TheCurrency::TICKER.into(),
                },
            },
            SwapLeg {
                from: LeaseC4::TICKER.into(),
                to: SwapTarget {
                    pool_id: 4,
                    target: TheCurrency::TICKER.into(),
                },
            },
            SwapLeg {
                from: LeaseC1::TICKER.into(),
                to: SwapTarget {
                    pool_id: 1,
                    target: LeaseC2::TICKER.into(),
                },
            },
            SwapLeg {
                from: NativeC::TICKER.into(),
                to: SwapTarget {
                    pool_id: 6,
                    target: LeaseC1::TICKER.into(),
                },
            },
            SwapLeg {
                from: LeaseC5::TICKER.into(),
                to: SwapTarget {
                    pool_id: 5,
                    target: LeaseC1::TICKER.into(),
                },
            },
            SwapLeg {
                from: LeaseC3::TICKER.into(),
                to: SwapTarget {
                    pool_id: 3,
                    target: LeaseC4::TICKER.into(),
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
                StableC::TICKER,
                LeaseC1::TICKER,
                NativeC::TICKER,
                LeaseC2::TICKER,
            ))
            .unwrap()
            .into_tree(),
            TheCurrency::TICKER.into(),
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
