use std::{fmt::Debug, marker::PhantomData};

use ::serde::{Deserialize, Serialize};
use trees::{walk::Visit, Node as TreeNode, TreeWalk};

use finance::{
    coin::serde::{deserialize as deserialize_currency, serialize as serialize_currency},
    currency::{Currency, SymbolOwned},
};
use sdk::{
    cosmwasm_std::{StdError, StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::error::{self, ContractError};

use self::serde::Leg;
pub use self::serde::TreeStore;

mod serde;

pub type ResolutionPath = Vec<SymbolOwned>;
pub type CurrencyPair = (SymbolOwned, SymbolOwned);

// TODO: move to finance/currency
pub type PoolId = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SwapLeg {
    pub from: SymbolOwned,
    pub to: SwapTarget,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SwapTarget {
    pub pool_id: PoolId,
    pub target: SymbolOwned,
}

type Node = TreeNode<Leg>;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct SupportedPairs<B>
where
    B: Currency,
{
    tree: TreeStore,
    #[serde(serialize_with = "serialize_currency")]
    #[serde(deserialize_with = "deserialize_currency")]
    _type: PhantomData<B>,
}

impl<'a, B> SupportedPairs<B>
where
    B: Currency,
{
    const DB_ITEM: Item<'a, SupportedPairs<B>> = Item::new("supported_pairs");

    // TODO: add checks for empty paths
    pub fn new(tree: TreeStore) -> Result<Self, ContractError> {
        if tree.root().data().1 != B::TICKER {
            return Err(ContractError::InvalidBaseCurrency(
                tree.root().data().1.clone(),
                B::TICKER.to_owned(),
            ));
        }

        // check for duplicated nodes
        let mut supported_currencies: Vec<&SymbolOwned> =
            tree.bfs().iter.map(|v| &v.data.1).collect();
        supported_currencies.sort();
        supported_currencies.dedup();

        if supported_currencies.len() != tree.node_count() {
            return Err(ContractError::DuplicatedNodes {});
        }

        Ok(SupportedPairs {
            tree,
            _type: PhantomData,
        })
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::DB_ITEM
            .may_load(storage)?
            .ok_or_else(|| StdError::generic_err("supported pairs tree not found"))
    }

    pub fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::DB_ITEM.save(storage, self)
    }

    fn find_node<'b>(node: &'b Node, query: &SymbolOwned) -> Option<&'b Node> {
        if &node.data().1 == query {
            Some(node)
        } else {
            node.iter().find_map(|child| Self::find_node(child, query))
        }
    }

    pub fn load_path(&self, query: &SymbolOwned) -> Result<ResolutionPath, ContractError> {
        if let Some(mut node) = Self::find_node(self.tree.root(), query) {
            let mut path = vec![node.data().1.to_owned()];
            while let Some(parent) = node.parent() {
                path.push(parent.data().1.to_owned());
                node = parent;
            }
            Ok(path)
        } else {
            Err(error::unsupported_currency::<B>(query))
        }
    }

    pub fn load_swap_path(
        &self,
        from: &SymbolOwned,
        to: &SymbolOwned,
    ) -> Result<Vec<SwapTarget>, ContractError> {
        let mut path_from = if let Some(mut node) = Self::find_node(self.tree.root(), from) {
            let mut path = vec![];
            while let Some(parent) = node.parent() {
                path.push(SwapLeg {
                    from: node.data().1.to_owned(),
                    to: SwapTarget {
                        pool_id: node.data().0,
                        target: parent.data().1.to_owned(),
                    },
                });
                node = parent;
            }
            Ok(path)
        } else {
            Err(error::unsupported_currency::<B>(from))
        }?;

        let mut path_to = if let Some(mut node) = Self::find_node(self.tree.root(), to) {
            let mut path = vec![];
            while let Some(parent) = node.parent() {
                path.push(SwapLeg {
                    from: parent.data().1.to_owned(),
                    to: SwapTarget {
                        pool_id: node.data().0,
                        target: node.data().1.to_owned(),
                    },
                });
                node = parent;
            }
            Ok(path)
        } else {
            Err(error::unsupported_currency::<B>(to))
        }?;

        while let (Some(to_leg), Some(from_leg)) = (path_to.last(), path_from.last()) {
            if to_leg.to.target == from_leg.from {
                path_from.pop();
                path_to.pop();
            } else {
                break;
            }
        }

        path_to.reverse();
        path_from.append(&mut path_to);
        let result = path_from.iter_mut().map(|leg| &leg.to).cloned().collect();

        Ok(result)
    }

    pub fn load_affected(&self, pair: &CurrencyPair) -> Result<Vec<SymbolOwned>, ContractError> {
        if let Some(node) = Self::find_node(self.tree.root(), &pair.0) {
            let affected = node.bfs().iter.map(|v| v.data.1.clone()).collect();
            Ok(affected)
        } else {
            Err(ContractError::InvalidDenomPair(
                pair.0.to_owned(),
                pair.1.to_owned(),
            ))
        }
    }

    pub fn query_supported_pairs(self) -> Vec<SwapLeg> {
        let mut legs = vec![];
        let mut walk = TreeWalk::from(self.tree.0);

        while let Some(visit) = walk.next() {
            match visit {
                Visit::Leaf(node) | Visit::Begin(node) => {
                    if let Some(parent) = node.parent() {
                        let node = node.data();

                        let leg = SwapLeg {
                            from: node.1.clone(),
                            to: SwapTarget {
                                pool_id: node.0,
                                target: parent.data().1.clone(),
                            },
                        };

                        legs.push(leg)
                    }
                }
                _ => (),
            }
        }

        legs
    }
}

#[cfg(test)]
mod tests {
    use finance::{currency::Currency, test::currency::Usdc};
    use sdk::cosmwasm_std::testing;
    use trees::tr;

    use super::*;

    type TheCurrency = Usdc;

    fn test_case() -> TreeStore {
        let base = TheCurrency::TICKER;

        TreeStore(
            tr((0, base.into()))
                / (tr((4, "token4".into())) / tr((3, "token3".into())))
                / (tr((2, "token2".into()))
                    / (tr((1, "token1".into()))
                        / tr((5, "token5".into()))
                        / tr((6, "token6".into())))),
        )
    }

    #[test]
    fn test_storage() {
        let tree = test_case();
        let sp = SupportedPairs::<Usdc>::new(tree).unwrap();
        let mut deps = testing::mock_dependencies();

        sp.save(deps.as_mut().storage).unwrap();
        let restored = SupportedPairs::load(deps.as_ref().storage).unwrap();

        assert_eq!(restored, sp);
    }

    #[test]
    #[should_panic]
    fn test_invalid_base() {
        let tree = TreeStore(tr((0, "invalid".into())) / tr((1, "token1".into())));

        SupportedPairs::<TheCurrency>::new(tree).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_duplicated_nodes() {
        let tree = TreeStore(
            tr((0, TheCurrency::TICKER.into()))
                / tr((1, "token1".into()))
                / (tr((2, "token2".into())) / tr((1, "token1".into()))),
        );

        SupportedPairs::<TheCurrency>::new(tree).unwrap();
    }

    #[test]
    fn test_load_path() {
        let tree = SupportedPairs::<Usdc>::new(test_case()).unwrap();

        let resp = tree.load_path(&"token5".into()).unwrap();
        assert_eq!(
            resp,
            vec![
                "token5".to_string(),
                "token1".to_string(),
                "token2".to_string(),
                TheCurrency::TICKER.to_string()
            ]
        );
    }

    #[test]
    fn test_load_swap_path() {
        let tree = SupportedPairs::<Usdc>::new(test_case()).unwrap();

        let resp = tree
            .load_swap_path(&"token5".into(), &TheCurrency::TICKER.into())
            .unwrap();
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

        let resp = tree
            .load_swap_path(&"token6".into(), &"token5".into())
            .unwrap();
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

        let resp = tree
            .load_swap_path(&"token2".into(), &"token4".into())
            .unwrap();
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
    fn test_load_affected() {
        let tree = SupportedPairs::<Usdc>::new(test_case()).unwrap();

        let mut resp = tree
            .load_affected(&("token2".into(), TheCurrency::TICKER.into()))
            .unwrap();
        resp.sort();

        let mut expect = vec![
            "token1".to_string(),
            "token2".to_string(),
            "token5".to_string(),
            "token6".to_string(),
        ];
        expect.sort();

        assert_eq!(resp, expect);
    }

    #[test]
    fn test_query_supported_pairs() {
        let paths = test_case();
        let tree = SupportedPairs::<Usdc>::new(paths).unwrap();

        let mut response = tree.query_supported_pairs();
        response.sort_by(|a, b| a.from.cmp(&b.from));

        let mut expected = vec![
            SwapLeg {
                from: "token2".into(),
                to: SwapTarget {
                    pool_id: 2,
                    target: TheCurrency::TICKER.to_owned(),
                },
            },
            SwapLeg {
                from: "token4".into(),
                to: SwapTarget {
                    pool_id: 4,
                    target: TheCurrency::TICKER.to_owned(),
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
        expected.sort_by(|a, b| a.from.cmp(&b.from));

        assert_eq!(response, expected);
    }
}
