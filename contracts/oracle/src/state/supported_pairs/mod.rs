use std::{fmt::Debug, marker::PhantomData};

use ::serde::{Deserialize, Serialize};
use cosmwasm_std::{StdError, StdResult, Storage};
use cw_storage_plus::Item;
use trees::{Node as TreeNode, Tree};

use finance::{
    coin::serde::{deserialize as deserialize_currency, serialize as serialize_currency},
    currency::{Currency, SymbolOwned},
};

use crate::error::ContractError;

use self::serde::TreeStore;

mod serde;

pub type ResolutionPath = Vec<SymbolOwned>;
pub type CurrencyPair = (SymbolOwned, SymbolOwned);
type Node = TreeNode<SymbolOwned>;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportedPairs<B>
where
    B: Currency,
{
    tree: TreeStore,
    supported_currencies: Vec<SymbolOwned>,
    #[serde(serialize_with = "serialize_currency")]
    #[serde(deserialize_with = "deserialize_currency")]
    _type: PhantomData<B>,
}

impl<B> Debug for SupportedPairs<B>
where
    B: Currency,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.tree.to_string())
    }
}

impl<'a, B> SupportedPairs<B>
where
    B: Currency,
{
    const DB_ITEM: Item<'a, SupportedPairs<B>> = Item::new("supported_pairs");

    // TODO: add checks for empty paths
    pub fn new(paths: Vec<ResolutionPath>) -> Result<Self, ContractError> {
        let mut supported_currencies = vec![];
        for path in paths.iter() {
            Self::validate_path(path)?;
            supported_currencies.push(path[0].clone());
        }

        supported_currencies.sort();
        supported_currencies.dedup();

        // edges with depth level
        let mut edges: Vec<((&SymbolOwned, usize), &SymbolOwned)> = paths
            .iter()
            .flat_map(|path| {
                path[..path.len() - 1]
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (n, path.len() - i))
                    .zip(path[1..].iter())
            })
            .collect();
        edges.sort();
        edges.dedup();

        // check for unambiguous edges
        let mut prev = &edges[0];
        for edge in edges[1..].iter() {
            if edge.0 == prev.0 {
                return Err(ContractError::InvalidDenomPair((
                    edge.0 .0.to_owned(),
                    edge.1.to_owned(),
                )));
            }
            prev = edge;
        }

        // fill the tree
        edges.sort_by(|a, b| a.0 .1.cmp(&b.0 .1).reverse());

        let mut branches: Vec<Tree<SymbolOwned>> = vec![];
        while !edges.is_empty() {
            let mut rest = edges.split_off(edges.partition_point(|e| e.0 .1 == edges[0].0 .1));
            std::mem::swap(&mut rest, &mut edges);

            let mut tips: Vec<_> = rest.iter().map(|e| Tree::new(e.1.clone())).collect();
            tips.sort();
            tips.dedup();

            // add branches
            while let Some(branch) = branches.pop() {
                if let Some(edge) = rest.iter().find(|e| e.0 .0 == branch.root().data()) {
                    if let Some(tip) = tips.iter_mut().find(|tip| tip.root().data() == edge.1) {
                        tip.push_back(branch);
                    }
                }
            }

            // add free edges
            for edge in rest {
                if let Some(tip) = tips.iter_mut().find(|tip| tip.root().data() == edge.1) {
                    if !tip.iter().any(|child| child.data() == edge.0 .0) {
                        tip.push_back(Tree::new(edge.0 .0.clone()));
                    }
                }
            }

            branches = tips;
        }

        let tree = branches
            .pop()
            .ok_or_else(|| StdError::generic_err("Wrong resolution paths"))?;
        let supported_pairs = Self {
            tree: TreeStore(tree),
            supported_currencies,
            _type: PhantomData::default(),
        };

        Ok(supported_pairs)
    }

    fn validate_path(path: &ResolutionPath) -> Result<(), ContractError> {
        if let Some(base) = path.last() {
            if base != B::SYMBOL || path.len() < 2 {
                return Err(ContractError::InvalidResolutionPath(path.clone()));
            }
        } else {
            return Err(ContractError::InvalidResolutionPath(path.clone()));
        }

        Ok(())
    }

    fn find_node<'b>(node: &'b Node, query: &SymbolOwned) -> Option<&'b Node> {
        if node.data() == query {
            Some(node)
        } else {
            node.iter().find_map(|child| Self::find_node(child, query))
        }
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::DB_ITEM
            .may_load(storage)?
            .ok_or_else(|| StdError::generic_err("supported pairs tree not found"))
    }

    pub fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Self::DB_ITEM.save(storage, self)
    }

    pub fn load_path(&self, query: &SymbolOwned) -> Result<ResolutionPath, ContractError> {
        if let Some(mut node) = Self::find_node(self.tree.root(), query) {
            let mut path = vec![node.data().to_owned()];
            while let Some(parent) = node.parent() {
                path.push(parent.data().to_owned());
                node = parent;
            }
            Ok(path)
        } else {
            Err(ContractError::InvalidDenomPair((
                query.to_owned(),
                B::SYMBOL.to_owned(),
            )))
        }
    }

    pub fn load_affected(&self, pair: &CurrencyPair) -> Result<Vec<SymbolOwned>, ContractError> {
        if let Some(node) = Self::find_node(self.tree.root(), &pair.0) {
            let affected = node.bfs().iter.map(|v| v.data.clone()).collect();
            Ok(affected)
        } else {
            Err(ContractError::InvalidDenomPair(pair.to_owned()))
        }
    }

    pub fn validate_supported(&self, query: &SymbolOwned) -> Result<(), ContractError> {
        self.supported_currencies
            .binary_search(query)
            .map_err(|_| {
                ContractError::InvalidDenomPair((query.to_owned(), B::SYMBOL.to_owned()))
            })?;
        Ok(())
    }

    pub fn query_supported_pairs(&self) -> Vec<CurrencyPair> {
        self.supported_currencies
            .iter()
            .cloned()
            .map(|c| (c, B::SYMBOL.to_owned()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing;

    use finance::{currency::Currency, test::currency::Usdc};

    use super::*;

    type TheCurrency = Usdc;

    fn test_case() -> Vec<ResolutionPath> {
        let base = TheCurrency::SYMBOL;
        vec![
            vec!["token0", "token1", "token2", base],
            vec!["token3", "token4", base],
            vec!["token5", "token1", "token2", base],
        ]
        .into_iter()
        .map(|path| path.into_iter().map(|x| x.to_owned()).collect())
        .collect()
    }

    #[test]
    fn test_is_supported() {
        let paths = test_case();
        let tree = SupportedPairs::<Usdc>::new(paths).unwrap();

        assert!(tree.validate_supported(&"token3".into()).is_ok());
        assert!(tree.validate_supported(&"token1".into()).is_err());
        assert!(tree.validate_supported(&"token6".into()).is_err());
    }

    #[test]
    fn test_load_path() {
        let paths = test_case();
        let tree = SupportedPairs::<Usdc>::new(paths.clone()).unwrap();

        let resp = tree.load_path(&"token5".into()).unwrap();
        assert_eq!(resp, paths[2]);
    }

    #[test]
    fn test_load_affected() {
        let paths = test_case();
        let tree = SupportedPairs::<Usdc>::new(paths).unwrap();

        let mut resp = tree
            .load_affected(&("token2".into(), TheCurrency::SYMBOL.into()))
            .unwrap();
        resp.sort();

        let mut expect = vec![
            "token0".to_string(),
            "token1".to_string(),
            "token2".to_string(),
            "token5".to_string(),
        ];
        expect.sort();

        assert_eq!(resp, expect);
    }

    #[test]
    fn test_storage() {
        let paths = test_case();
        let tree = SupportedPairs::<Usdc>::new(paths).unwrap();
        let mut deps = testing::mock_dependencies();

        tree.save(deps.as_mut().storage).unwrap();
        let restored = SupportedPairs::load(deps.as_ref().storage).unwrap();

        assert_eq!(restored, tree);
    }

    #[test]
    #[should_panic]
    fn test_no_base_path() {
        SupportedPairs::<TheCurrency>::new(vec![vec!["token1".into(), "token2".into()]]).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_wrong_paths() {
        SupportedPairs::<TheCurrency>::new(vec![
            vec!["token0".into(), "token1".into(), TheCurrency::SYMBOL.into()],
            vec!["token0".into(), "token2".into(), TheCurrency::SYMBOL.into()],
        ])
        .unwrap();
    }
}
