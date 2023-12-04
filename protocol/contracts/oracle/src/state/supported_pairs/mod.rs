use std::fmt::Debug;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use currency::{SymbolOwned, SymbolSlice};
use swap::SwapTarget;

#[cfg(feature = "contract")]
pub use self::contract::SupportedPairs;

#[cfg(feature = "contract")]
mod contract;

pub type ResolutionPath = Vec<SymbolOwned>;
pub type CurrencyPair<'a> = (&'a SymbolSlice, &'a SymbolSlice);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SwapLeg {
    pub from: SymbolOwned,
    pub to: SwapTarget,
}

impl Serialize for SwapLeg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.from, &self.to).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SwapLeg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|(from, to)| Self { from, to })
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use currencies::test::StableC1;
    use currency::Currency;
    use sdk::cosmwasm_std::testing;
    use tree::HumanReadableTree;

    use super::*;

    type TheCurrency = StableC1;

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
        let sp = SupportedPairs::<StableC1>::new(tree.into_tree()).unwrap();
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
        let tree = SupportedPairs::<StableC1>::new(test_case().into_tree()).unwrap();

        let resp: Vec<_> = tree.load_path("token5").unwrap().collect();
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
        let tree = SupportedPairs::<StableC1>::new(test_case().into_tree()).unwrap();

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
    fn test_load_affected() {
        let tree = SupportedPairs::<StableC1>::new(test_case().into_tree()).unwrap();

        let mut resp = tree.load_affected(("token2", TheCurrency::TICKER)).unwrap();
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
        let tree = SupportedPairs::<StableC1>::new(paths.into_tree()).unwrap();

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
}
