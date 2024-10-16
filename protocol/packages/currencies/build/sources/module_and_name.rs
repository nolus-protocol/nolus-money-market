use std::collections::BTreeMap;

use anyhow::{anyhow, Result};

use topology::CurrencyDefinition;

use crate::{protocol::Protocol, LPN_NAME, NLS_NAME};

pub(super) struct ModuleAndName<'module, 'name> {
    module: &'module str,
    name: &'name str,
}

impl<'currencies_map> ModuleAndName<'static, 'currencies_map> {
    pub fn resolve(
        protocol: &Protocol,
        host_currency: &CurrencyDefinition,
        dex_currencies: &'currencies_map BTreeMap<&str, (String, &CurrencyDefinition)>,
        ticker: &str,
    ) -> Result<Self> {
        if let Some(name) = dex_currencies.get(ticker).map(|(name, _)| name) {
            Ok(if ticker == protocol.lpn_ticker {
                const {
                    Self {
                        module: "lpn::impl_mod",
                        name: LPN_NAME,
                    }
                }
            } else {
                Self {
                    module: if protocol.lease_currencies_tickers.contains(ticker) {
                        "lease::impl_mod::definitions"
                    } else {
                        "payment::only::impl_mod::definitions"
                    },
                    name: name.as_str(),
                }
            })
        } else if ticker == host_currency.ticker() {
            const {
                Ok(Self {
                    module: "native",
                    name: NLS_NAME,
                })
            }
        } else {
            Err(anyhow!(
                "Queried ticker is not defined neither as a DEX currency, nor \
                as a host currency!",
            ))
        }
    }
}

impl<'module, 'name> ModuleAndName<'module, 'name> {
    pub const fn module(&self) -> &'module str {
        self.module
    }

    pub const fn name(&self) -> &'name str {
        self.name
    }
}
