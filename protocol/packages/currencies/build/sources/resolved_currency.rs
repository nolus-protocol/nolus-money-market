use std::borrow::Borrow;

use anyhow::{anyhow, Result};

use topology::{CurrencyDefinition, HostCurrency};

use crate::protocol::Protocol;

use super::{DexCurrencies, LPN_NAME, NLS_NAME};

#[derive(Debug, Clone, Copy)]
pub(super) enum CurrentModule {
    Lease,
    Lpn,
    Native,
    PaymentOnly,
    Stable,
}

impl CurrentModule {
    #[inline]
    fn lease(&self) -> &'static str {
        if matches!(self, Self::Lease) {
            "self"
        } else {
            "crate::lease"
        }
    }

    #[inline]
    fn lpn(&self) -> &'static str {
        if matches!(self, Self::Lpn) {
            "self"
        } else {
            "crate::lpn"
        }
    }

    #[inline]
    fn native(&self) -> &'static str {
        if matches!(self, Self::Native) {
            "self"
        } else {
            "crate::native"
        }
    }

    #[inline]
    fn payment_only(&self) -> &'static str {
        if matches!(self, Self::PaymentOnly) {
            "self"
        } else {
            "crate::payment::only"
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
        host_currency: &'host_currency HostCurrency,
        dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
        ticker: &str,
    ) -> Result<Self> {
        if let Some(&(ref name, definition)) = dex_currencies.get(ticker) {
            if ticker == protocol.lpn_ticker {
                Ok((current_module.lpn(), LPN_NAME))
            } else if protocol.lease_currencies_tickers.contains(ticker) {
                Ok((current_module.lease(), name.as_str()))
            } else if protocol.payment_only_currencies_tickers.contains(ticker) {
                Ok((current_module.payment_only(), name.as_str()))
            } else {
                Err(anyhow!(
                    "Failed to resolve module because queried ticker belongs \
                    to a currency that is not assigned to any group."
                ))
            }
            .map(|(module, name)| Self {
                module,
                name,
                definition,
            })
        } else if ticker == CurrencyDefinition::ticker(host_currency.borrow()) {
            Ok(Self {
                module: current_module.native(),
                name: NLS_NAME,
                definition: host_currency.borrow(),
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
