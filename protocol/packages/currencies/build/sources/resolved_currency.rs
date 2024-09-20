use anyhow::{anyhow, Result};

use topology::CurrencyDefinition;

use crate::protocol::Protocol;

use super::{DexCurrencies, LPN_NAME, NLS_NAME};

#[derive(Debug, Clone, Copy)]
pub(super) enum CurrentModule {
    Lease,
    Lpn,
    Native,
    PaymentOnly,
}

impl CurrentModule {
    #[inline]
    fn lease(&self) -> &'static str {
        if matches!(self, Self::Lease) {
            "self"
        } else {
            "crate::lease::impl_mod"
        }
    }

    #[inline]
    fn lpn(&self) -> &'static str {
        if matches!(self, Self::Lpn) {
            "self"
        } else {
            "crate::lpn::impl_mod"
        }
    }

    #[inline]
    fn native(&self) -> &'static str {
        if matches!(self, Self::Native) {
            "self"
        } else {
            "crate::native::impl_mod"
        }
    }

    #[inline]
    fn payment_only(&self) -> &'static str {
        if matches!(self, Self::PaymentOnly) {
            "self"
        } else {
            "crate::payment::only::impl_mod"
        }
    }
}

pub(super) struct ResolvedCurrency<'name, 'definition> {
    module: &'static str,
    name: &'name str,
    definition: &'definition CurrencyDefinition,
}

impl<'host_currency, 'dex_currencies, 'definition> ResolvedCurrency<'dex_currencies, 'definition>
where
    'host_currency: 'definition,
    'dex_currencies: 'definition,
{
    pub fn resolve(
        current_module: CurrentModule,
        protocol: &Protocol,
        host_currency: &'host_currency CurrencyDefinition,
        dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
        ticker: &str,
    ) -> Result<Self> {
        if let Some((name, definition)) = dex_currencies.get(ticker) {
            let (module, name) = if ticker == protocol.lpn_ticker {
                (current_module.lpn(), LPN_NAME)
            } else {
                (
                    if protocol.lease_currencies_tickers.contains(ticker) {
                        current_module.lease()
                    } else {
                        current_module.payment_only()
                    },
                    name.as_str(),
                )
            };

            Ok(Self {
                module,
                name,
                definition,
            })
        } else if ticker == host_currency.ticker() {
            Ok(Self {
                module: current_module.native(),
                name: NLS_NAME,
                definition: host_currency,
            })
        } else {
            Err(anyhow!(
                "Failed to resolve module and name because queried ticker \
                {ticker:?} is not defined neither as a DEX currency, nor as a \
                host currency!",
            ))
        }
    }
}

impl<'name, 'definition> ResolvedCurrency<'name, 'definition> {
    #[inline]
    pub const fn module(&self) -> &'static str {
        self.module
    }

    #[inline]
    pub const fn name(&self) -> &'name str {
        self.name
    }

    #[inline]
    pub const fn definition(&self) -> &'definition CurrencyDefinition {
        self.definition
    }
}
