use std::collections::{BTreeMap, BTreeSet};

use heck::ToPascalCase;
use serde::{Deserialize, Serialize};

use nolus_config::ModuleName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Currencies {
    currencies: BTreeMap<String, Currency>,
}

impl Currencies {
    pub fn currency_names_iter(&self) -> impl Iterator<Item = &str> {
        self.currencies.keys().map(String::as_str)
    }

    pub fn currencies_iter(&self) -> impl Iterator<Item = nolus_config::Currency> {
        self.currencies.clone().into_iter().map(
            |(
                ticker,
                Currency {
                    friendly_name,
                    symbol,
                    ibc_route,
                    ..
                },
            )| {
                nolus_config::Currency::new(
                    ticker.to_pascal_case(),
                    ticker,
                    friendly_name,
                    symbol,
                    ibc_route,
                )
            },
        )
    }

    pub fn groups_iter(&self) -> impl Iterator<Item = nolus_config::Group> + '_ {
        let groups: BTreeSet<String> = self
            .currencies
            .values()
            .flat_map(|Currency { groups, .. }| groups.iter().cloned())
            .collect();

        groups.into_iter().map(|friendly_name| {
            nolus_config::Group::new(
                friendly_name.to_pascal_case(),
                friendly_name.clone(),
                self.currencies
                    .iter()
                    .filter_map(
                        |(
                            ticker,
                            Currency {
                                friendly_name: currency_friendly_name,
                                symbol,
                                ibc_route,
                                groups,
                            },
                        )| {
                            groups.contains(&friendly_name).then(|| {
                                let currency = nolus_config::Currency::new(
                                    ticker.to_pascal_case(),
                                    ticker.clone(),
                                    currency_friendly_name.clone(),
                                    symbol.clone(),
                                    ibc_route.clone(),
                                );
                                nolus_config::CurrencyWithModule::new(
                                    currency.module_name(),
                                    currency,
                                )
                            })
                        },
                    )
                    .collect(),
            )
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Currency {
    #[serde(rename = "name")]
    friendly_name: String,
    symbol: String,
    ibc_route: Vec<String>,
    groups: Vec<String>,
}
