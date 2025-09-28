use topology::HostCurrency;

use crate::protocol::Protocol;

use super::{DexCurrencies, resolved_currency::CurrentModule};

pub(super) use self::{
    builder::Builder,
    group_member::{Entry as GroupMemberEntry, GroupMember},
    in_pool_with::InPoolWith,
    pairs_group::PairsGroup,
    resolver::Resolver,
};

mod builder;
mod group_member;
mod in_pool_with;
mod pairs_group;
mod resolver;

// TODO [precise capturing in trait definition]
//  Replace with precise capturing when it becomes available in trait
//  definitions.
pub(super) trait Captures<T>
where
    T: ?Sized,
{
}

impl<T, U> Captures<U> for T
where
    T: ?Sized,
    U: ?Sized,
{
}

pub(super) struct StaticContext<
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
> {
    protocol: &'protocol Protocol,
    host_currency: &'host_currency HostCurrency,
    dex_currencies: &'dex_currencies DexCurrencies<'dex_currency_ticker, 'dex_currency_definition>,
}

impl<'protocol, 'host_currency, 'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    StaticContext<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >
{
    #[inline]
    pub const fn new(
        protocol: &'protocol Protocol,
        host_currency: &'host_currency HostCurrency,
        dex_currencies: &'dex_currencies DexCurrencies<
            'dex_currency_ticker,
            'dex_currency_definition,
        >,
    ) -> Self {
        Self {
            protocol,
            host_currency,
            dex_currencies,
        }
    }
}

pub(super) struct Generator<
    'static_context,
    'protocol,
    'host_currency,
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
    const PAIRS_GROUP: bool,
> {
    static_context: &'static_context StaticContext<
        'protocol,
        'host_currency,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
    >,
    current_module: CurrentModule,
}
