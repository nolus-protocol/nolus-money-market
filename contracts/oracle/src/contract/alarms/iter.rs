use std::{iter, ops::Deref};

use serde::{de::DeserializeOwned, Serialize};

use currency::{self, AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers};
use finance::price::{base::BasePrice, Price};
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
    pub fn new(alarms: &'alarms PriceAlarms<'storage, S>, price_iter: I) -> ContractResult<Self> {
        let mut iter = Self {
            alarms,
            price_iter,
            alarm_iter: None,
        };
        iter.alarm_iter = iter.next_alarms()?;
        Ok(iter)
    }

    fn move_to_next_alarms(&mut self) -> ContractResult<()> {
        debug_assert!(self.next_alarm().is_none());

        self.alarm_iter = self.next_alarms()?;
        Ok(())
    }

    fn next_alarms(&mut self) -> ContractResult<Option<AlarmIter<'alarms>>> {
        self.price_iter
            .next()
            .map(|price_result: PriceResult<BaseC>| {
                price_result.and_then(|ref price| {
                    Tickers.visit_any::<SwapGroup, Cmd<'storage, 'alarms, '_, S, BaseC>>(
                        price.base_ticker(),
                        Cmd {
                            alarms: self.alarms,
                            price,
                        },
                    )
                })
            })
            .transpose()
    }

    fn next_alarm(&mut self) -> Option<ContractResult<Addr>> {
        debug_assert!(self.alarm_iter.is_some());
        self.alarm_iter
            .as_mut()
            .expect("calling 'next_alarm' on Some price alarms")
            .next()
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
        self.alarm_iter.as_ref()?;

        let mut result = self.next_alarm();
        while result.is_none() && self.alarm_iter.is_some() {
            result = if let Err(error) = self.move_to_next_alarms() {
                Some(Err(error))
            } else if self.alarm_iter.is_none() {
                None
            } else {
                self.next_alarm()
            }
        }
        result
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
