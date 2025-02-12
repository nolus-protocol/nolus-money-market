use std::{borrow::Borrow, collections::BTreeMap};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
#[serde(transparent)]
pub(crate) struct Dexes(BTreeMap<Id, Dex>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
struct Id(String);

impl Borrow<str> for Id {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct Dex {
    r#type: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum Type {
    AstroportTest,
    AstroportMain,
    Osmosis,
}
