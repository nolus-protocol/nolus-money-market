use std::iter;

use anyhow::Result;
use either::Either;

use topology::HostCurrency;

use crate::{
    protocol::Protocol,
    sources::resolved_currency::{CurrentModule, ResolvedCurrency},
};

use super::DexCurrencies;

pub(super) fn pairs_group<'r, 'dex_currencies, 'parent, Parents>(
    current_module: CurrentModule,
    protocol: &Protocol,
    host_currency: &HostCurrency,
    dex_currencies: &'dex_currencies DexCurrencies<'_, '_>,
    name: &'r str,
    mut parents: Parents,
) -> Result<impl Iterator<Item = &'r str> + use<'r, 'parent, Parents>>
where
    'dex_currencies: 'r,
    'parent: 'r,
    Parents: Iterator<Item = &'parent str> + Clone,
{
    if let Some(ticker) = parents.next() {
        iter::once(ticker)
            .chain(parents)
            .map(|ticker| {
                ResolvedCurrency::resolve(
                    current_module,
                    protocol,
                    host_currency,
                    dex_currencies,
                    ticker,
                )
                .map(|resolved| [" (", resolved.module(), "::", resolved.name(), ","])
            })
            .collect::<Result<Vec<_>>>()
            .map(|currencies| {
                let closing_parenthesis = iter::repeat_n(")", currencies.len());

                Either::Left(currencies.into_iter().flatten().chain(closing_parenthesis))
            })
    } else {
        Ok(Either::Right(iter::once(" ()")))
    }
    .map(|currencies| {
        [
            "
    impl currency::PairsGroup for ",
            name,
            " {
        type CommonGroup = crate::payment::Group;

        type PairedWith =",
        ]
        .into_iter()
        .chain(currencies)
        .chain(iter::once(
            ";

        fn find_map<FindMap>(
            find_map: FindMap,
        ) -> Result<<FindMap as currency::PairsFindMap>::Outcome, FindMap>
        where
            FindMap: currency::PairsFindMap<Pivot = Self>,
        {
            currency::pairs_find(find_map)
        }
    }
",
        ))
    })
}
