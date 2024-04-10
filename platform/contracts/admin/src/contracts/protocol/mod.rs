use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

#[cfg(feature = "contract")]
mod impl_mod;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ProtocolTemplate<T> {
    pub leaser: T,
    pub lpp: T,
    pub oracle: T,
    pub profit: T,
    pub reserve: T,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Protocol {
    pub network: Network,
    pub dex: Dex,
    pub contracts: ProtocolTemplate<Addr>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    rename_all = "PascalCase",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
// TODO remove aliases after next migration
pub enum Network {
    #[serde(alias = "NEUTRON")]
    Neutron,
    #[serde(alias = "OSMOSIS")]
    Osmosis,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    rename_all = "PascalCase",
    rename_all_fields = "snake_case",
    deny_unknown_fields
)]
pub enum Dex {
    Astroport { router_address: String },
    Osmosis,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_dex_serde() {
        const ASTROPORT_ROUTER_ADDRESS: &str = "neutron0123456789ABCDEF";

        assert_eq!(
            sdk::cosmwasm_std::from_json(format!(
                r#"{{
                    "Astroport": {{
                        "router_address": {ASTROPORT_ROUTER_ADDRESS:?}
                    }}
                }}"#
            )),
            Ok(super::Dex::Astroport {
                router_address: ASTROPORT_ROUTER_ADDRESS.to_string()
            })
        );

        assert_eq!(
            sdk::cosmwasm_std::from_json(r#""Osmosis""#),
            Ok(super::Dex::Osmosis {})
        );
    }
}
