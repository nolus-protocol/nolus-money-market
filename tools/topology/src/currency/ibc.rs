use serde::Deserialize;

use crate::{network, skippable::Skippable};

#[cfg(test)]
mod tests {
    use super::Ibc;

    #[test]
    fn overriden_symbol_present() {
        const SOURCE: &str =
            r#"{"network": "NetA", "currency": "CUR", "override_symbol": "myovr"}"#;

        let ibc = serde_json::from_str::<'_, Ibc>(SOURCE)
            .expect("Ibc with an override symbol should deserialize!");

        assert_eq!(
            ibc.overriden_symbol().map(|id| id.as_ref()),
            Some("myovr"),
            "{ibc:?}"
        );
    }

    #[test]
    fn overriden_symbol_absent() {
        const SOURCE: &str = r#"{"network": "NetA", "currency": "CUR"}"#;

        let ibc = serde_json::from_str::<'_, Ibc>(SOURCE)
            .expect("Ibc without an override symbol should deserialize!");

        assert!(ibc.overriden_symbol().is_none(), "{ibc:?}");
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Ibc {
    network: network::Id,
    currency: super::Id,
    #[serde(default)]
    override_symbol: Skippable<super::Id>,
}

impl Ibc {
    #[inline]
    pub const fn network(&self) -> &network::Id {
        &self.network
    }

    #[inline]
    pub const fn currency(&self) -> &super::Id {
        &self.currency
    }

    #[inline]
    pub const fn overriden_symbol(&self) -> Option<&super::Id> {
        match self.override_symbol {
            Skippable::Skipped => None,
            Skippable::Some(ref symbol) => Some(symbol),
        }
    }
}
