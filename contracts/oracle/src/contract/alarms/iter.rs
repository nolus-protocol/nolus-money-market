use std::{iter, ops::Deref};

use serde::{de::DeserializeOwned, Serialize};

use finance::{
    currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency},
    price::{base::BasePrice, Price},
};
use marketprice::alarms::{errors::AlarmError, AlarmsIterator, PriceAlarms};
use sdk::cosmwasm_std::{Addr, Storage};
use swap::SwapGroup;

use crate::{contract::alarms::PriceResult, error::ContractError, result::ContractResult};

type AlarmIterMapFn = fn(Result<Addr, AlarmError>) -> ContractResult<Addr>;
type AlarmIter<'alarms> = iter::Map<AlarmsIterator<'alarms>, AlarmIterMapFn>;

pub struct Iter<'storage, 'alarms, S, I, BaseC>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    I: Iterator<Item = PriceResult<BaseC>>,
    BaseC: Currency,
{
    alarms: &'alarms PriceAlarms<'storage, S>,
    price_iter: I,
    alarm_iter: Option<AlarmIter<'alarms>>,
}

impl<'storage, 'alarms, S, I, BaseC> Iter<'storage, 'alarms, S, I, BaseC>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    I: Iterator<Item = PriceResult<BaseC>>,
    BaseC: Currency,
{
    pub fn new(alarms: &'alarms PriceAlarms<'storage, S>, price_iter: I) -> Self {
        Self {
            alarms,
            price_iter,
            alarm_iter: None,
        }
    }
}

impl<'storage, 'alarms, S, I, BaseC> Iterator for Iter<'storage, 'alarms, S, I, BaseC>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    I: Iterator<Item = PriceResult<BaseC>>,
    BaseC: Currency,
{
    type Item = ContractResult<Addr>;

    fn next(&mut self) -> Option<Self::Item> {
        self.alarm_iter
            .as_mut()
            .map(Iterator::next)
            .unwrap_or_else(|| {
                self.alarm_iter = None;

                None
            })
            .or_else(|| {
                self.price_iter.next()?.map_or_else(
                    |error: ContractError| Some(Err(error)),
                    |ref price| {
                        let iter: AlarmIter<'alarms> = match visit_any_on_ticker::<
                            SwapGroup,
                            Cmd<'storage, 'alarms, '_, S, BaseC>,
                        >(
                            price.base_ticker(),
                            Cmd {
                                alarms: self.alarms,
                                price,
                            },
                        ) {
                            Ok(iter) => iter,
                            Err(error) => return Some(Err(error)),
                        };

                        self.alarm_iter.insert(iter).next()
                    },
                )
            })
    }
}

struct Cmd<'storage, 'alarms, 'price, S, BaseC>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    BaseC: Currency,
{
    alarms: &'alarms PriceAlarms<'storage, S>,
    price: &'price BasePrice<SwapGroup, BaseC>,
}

impl<'storage, 'alarms, 'price, S, BaseC> AnyVisitor for Cmd<'storage, 'alarms, 'price, S, BaseC>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    BaseC: Currency,
{
    type Output = AlarmIter<'alarms>;
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
