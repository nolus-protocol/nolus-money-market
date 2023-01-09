use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{to_binary, Binary, StdError, StdResult, Storage},
    cw_storage_plus::Item,
    schemars::{self, JsonSchema},
};

pub type Version = u64;

pub const VERSION_ITEM: Item<'static, Version> = Item::new("contract_version");

pub fn initialize<const VERSION: Version>(storage: &mut dyn Storage) -> StdResult<()> {
    VERSION_ITEM.save(storage, &VERSION)
}

pub fn upgrade_contract<const VERSION: Version>(storage: &mut dyn Storage) -> StdResult<()> {
    VERSION_ITEM.update(storage, |version| if version.wrapping_add(1) == VERSION {
        Ok(VERSION)
    } else {
        Err(StdError::generic_err("Couldn't upgrade contract because versions aren't adjacent and/or monotonically increasing."))
    }).map(|_| ())
}

#[derive(
    Debug,
    Copy,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Default,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct QueryVersion {}

#[derive(
    Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(untagged, rename_all = "snake_case")]
pub enum Query<Q> {
    Version {
        version: QueryVersion,
    },
    Query {
        #[serde(flatten)]
        query: Q,
    },
}

impl<Q> Query<Q> {
    pub const fn new_query(query: Q) -> Self {
        Self::Query { query }
    }

    pub fn handle_query<const VERSION: Version, F>(self, f: F) -> StdResult<Binary>
    where
        F: FnOnce(Q) -> StdResult<Binary>,
    {
        match self {
            Query::Version {
                version: QueryVersion {},
            } => to_binary(&VERSION),
            Query::Query { query } => f(query),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{Query, QueryVersion};

    #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum VariantsQuery {
        Abc {},
        Def {},
    }

    const QUERY_VERSION: Query<VariantsQuery> = Query::Version {
        version: QueryVersion {},
    };

    const QUERY_ABC: Query<VariantsQuery> = Query::Query {
        query: VariantsQuery::Abc {},
    };

    const QUERY_DEF: Query<VariantsQuery> = Query::Query {
        query: VariantsQuery::Def {},
    };

    fn assert_query_serde(value: Query<VariantsQuery>) {
        assert_eq!(
            serde_json::from_str::<Query<VariantsQuery>>(&serde_json::to_string(&value).unwrap())
                .unwrap(),
            value
        );
    }

    #[test]
    fn test_query_serde() {
        assert_query_serde(QUERY_VERSION);
        assert_query_serde(QUERY_ABC);
        assert_query_serde(QUERY_DEF);

        assert_eq!(
            serde_json::from_str::<Query<VariantsQuery>>(r#"{"version":{}}"#).unwrap(),
            QUERY_VERSION
        );

        assert_eq!(
            serde_json::from_str::<Query<VariantsQuery>>(r#"{"abc":{}}"#).unwrap(),
            QUERY_ABC
        );

        assert_eq!(
            serde_json::from_str::<Query<VariantsQuery>>(r#"{"def":{}}"#).unwrap(),
            QUERY_DEF
        );
    }
}
