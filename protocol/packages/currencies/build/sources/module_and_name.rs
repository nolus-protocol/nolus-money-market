use std::collections::BTreeMap;

use anyhow::{anyhow, Result};

use topology::CurrencyDefinition;

use crate::{protocol::Protocol, LPN_NAME, NLS_NAME};

#[derive(Debug, Clone, Copy)]
pub(super) enum CurrentModule {
    Lease,
    Lpn,
    Native,
    PaymentOnly,
}

impl CurrentModule {
    fn lease(&self) -> &'static str {
        if matches!(self, Self::Lease) {
            "self"
        } else {
            "crate::lease::impl_mod::definitions"
        }
    }

    fn lpn(&self) -> &'static str {
        if matches!(self, Self::Lpn) {
            "self"
        } else {
            "crate::lpn::impl_mod"
        }
    }

    fn native(&self) -> &'static str {
        if matches!(self, Self::Native) {
            "self"
        } else {
            "crate::native"
        }
    }
    fn payment_only(&self) -> &'static str {
        if matches!(self, Self::PaymentOnly) {
            "self"
        } else {
            "crate::payment::only::impl_mod::definitions"
        }
    }
}

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
        current_module: CurrentModule,
    ) -> Result<Self> {
        if let Some(name) = dex_currencies.get(ticker).map(|(name, _)| name) {
            Ok(if ticker == protocol.lpn_ticker {
                Self {
                    module: current_module.lpn(),
                    name: LPN_NAME,
                }
            } else {
                Self {
                    module: if protocol.lease_currencies_tickers.contains(ticker) {
                        current_module.lease()
                    } else {
                        current_module.payment_only()
                    },
                    name: name.as_str(),
                }
            })
        } else if ticker == host_currency.ticker() {
            Ok(Self {
                module: current_module.native(),
                name: NLS_NAME,
            })
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
