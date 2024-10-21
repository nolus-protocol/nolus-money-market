use anyhow::Result;

use topology::CurrencyDefinition;

use crate::{
    protocol::Protocol,
    sources::module_and_name::{CurrentModule, ModuleAndName},
};

use super::DexCurrencies;

pub(super) fn in_pool_with<'r, 'dex_currencies, 'parent, 'name, Parents>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &CurrencyDefinition,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    parents: Parents,
    name: &'name str,
) -> Result<impl Iterator<Item = &'r str> + use<'r, Parents>>
where
    'dex_currencies: 'r,
    'name: 'r,
    Parents: Iterator<Item = &'parent str>,
{
    parents
        .map(|ticker| {
            ModuleAndName::resolve(
                current_module,
                protocol,
                host_currency,
                dex_currencies,
                ticker,
            )
            .map(|resolved| {
                [
                    "
    impl currency::InPoolWith<",
                    resolved.module(),
                    "::",
                    resolved.name(),
                    "> for ",
                    name,
                    " {}
",
                ]
            })
        })
        .collect::<Result<_, _>>()
        .map(Vec::into_iter)
        .map(Iterator::flatten)
}
