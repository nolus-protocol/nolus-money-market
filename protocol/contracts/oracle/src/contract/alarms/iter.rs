use std::iter;

use serde::{de::DeserializeOwned, Serialize};

use currency::{AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit, Tickers};
use finance::price::{base::BasePrice, Price};
use marketprice::alarms::{errors::AlarmError, AlarmsIterator, PriceAlarms};
use sdk::{cosmwasm_ext::as_dyn::storage, cosmwasm_std::Addr};

use crate::{contract::alarms::PriceResult, error::ContractError, result::ContractResult};

pub fn new<'alarms, 'iterator, 'r, S, PriceG, I, BaseC>(
    alarms: &'alarms PriceAlarms<S, PriceG>,
    price_iter: I,
) -> impl Iterator<Item = ContractResult<Addr>> + 'r
where
    'alarms: 'r,
    'iterator: 'r,
    S: storage::Dyn,
    I: Iterator<Item = PriceResult<PriceG, BaseC>> + 'iterator,
    PriceG: Group + Clone,
    BaseC: Currency,
{
    price_iter
        .map(move |price_result: PriceResult<PriceG, BaseC>| {
            price_result.and_then(move |ref price| {
                Tickers.visit_any::<PriceG, Cmd<'_, '_, S, PriceG, BaseC>>(
                    price.base_ticker(),
                    Cmd { alarms, price },
                )
            })
        })
        .flat_map(move |result| match result {
            Ok(iter) => IterOrErr::Iter(iter),
            Err(error) => IterOrErr::Err(iter::once(Err(error))),
        })
}

enum IterOrErr<T, Err, Iter: Iterator<Item = Result<T, Err>>> {
    Iter(Iter),
    Err(iter::Once<Result<T, Err>>),
}

impl<T, Err, Iter: Iterator<Item = Result<T, Err>>> Iterator for IterOrErr<T, Err, Iter> {
    type Item = Result<T, Err>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IterOrErr::Iter(iter) => iter.next(),
            IterOrErr::Err(iter) => iter.next(),
        }
    }
}

type AlarmIterMapFn = fn(Result<Addr, AlarmError>) -> ContractResult<Addr>;
type AlarmIter<'alarms, G> = iter::Map<AlarmsIterator<'alarms, G>, AlarmIterMapFn>;

struct Cmd<'alarms, 'price, S, PriceG, BaseC>
where
    S: storage::Dyn,
    PriceG: Group + Clone,
    BaseC: Currency,
{
    alarms: &'alarms PriceAlarms<S, PriceG>,
    price: &'price BasePrice<PriceG, BaseC>,
}

impl<'alarms, 'price, S, PriceG, BaseC> AnyVisitor for Cmd<'alarms, 'price, S, PriceG, BaseC>
where
    S: storage::Dyn,
    PriceG: Group + Clone,
    BaseC: Currency,
{
    type Output = AlarmIter<'alarms, PriceG>;
    type Error = ContractError;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency + Serialize + DeserializeOwned,
    {
        Price::<C, BaseC>::try_from(self.price)
            .map(|price: Price<C, BaseC>| {
                self.alarms
                    .alarms(price)
                    .map::<ContractResult<Addr>, AlarmIterMapFn>(
                        |result: Result<Addr, AlarmError>| result.map_err(Into::into),
                    )
            })
            .map_err(ContractError::from)
    }
}
