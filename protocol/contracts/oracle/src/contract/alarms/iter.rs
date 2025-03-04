use std::{iter, marker::PhantomData, ops::Deref};

use currency::{Currency, CurrencyDef, Group, MemberOf};
use finance::price::{self, base::with_price::WithPrice, Price};
use marketprice::alarms::{errors::AlarmError, AlarmsIterator, PriceAlarms};
use sdk::cosmwasm_std::{Addr, Storage};

use crate::{contract::alarms::PriceResult, error::Error, result::Result};

type AlarmIterMapFn<ErrorG> = fn(std::result::Result<Addr, AlarmError>) -> Result<Addr, ErrorG>;
type AlarmIter<'alarms, AlarmG, ErrorG> =
    iter::Map<AlarmsIterator<'alarms, AlarmG>, AlarmIterMapFn<ErrorG>>;

pub struct Iter<'storage, 'alarms, S, I, AlarmsG, BaseC, BaseG, ErrorG>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    I: Iterator<Item = PriceResult<AlarmsG, BaseC, BaseG, ErrorG>>,
    AlarmsG: Group,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<AlarmsG::TopG>,
    BaseG: Group,
    ErrorG: Group,
{
    alarms: &'alarms PriceAlarms<'storage, AlarmsG, S>,
    price_iter: I,
    alarm_iter: Option<AlarmIter<'alarms, AlarmsG, ErrorG>>,
}

impl<'storage, 'alarms, S, I, AlarmsG, BaseC, BaseG, ErrorG>
    Iter<'storage, 'alarms, S, I, AlarmsG, BaseC, BaseG, ErrorG>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    I: Iterator<Item = PriceResult<AlarmsG, BaseC, BaseG, ErrorG>>,
    AlarmsG: Group,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<AlarmsG::TopG>,
    BaseG: Group,
    ErrorG: Group,
{
    pub fn new(
        alarms: &'alarms PriceAlarms<'storage, AlarmsG, S>,
        price_iter: I,
    ) -> Result<Self, ErrorG> {
        let mut iter = Self {
            alarms,
            price_iter,
            alarm_iter: None,
        };
        iter.alarm_iter = iter.next_alarms()?;
        Ok(iter)
    }

    fn move_to_next_alarms(&mut self) -> Result<(), ErrorG> {
        debug_assert!(self.next_alarm().is_none());

        self.alarm_iter = self.next_alarms()?;
        Ok(())
    }

    fn next_alarms(&mut self) -> Result<Option<AlarmIter<'alarms, AlarmsG, ErrorG>>, ErrorG> {
        self.price_iter
            .next()
            .map(|price_result| {
                price_result.and_then(|ref price| {
                    price::base::with_price::execute(
                        price,
                        Cmd {
                            alarms: self.alarms,
                            _base_c: PhantomData::<BaseC>,
                            _error_g: PhantomData::<ErrorG>,
                        },
                    )
                })
            })
            .transpose()
    }

    fn next_alarm(&mut self) -> Option<Result<Addr, ErrorG>> {
        match self.alarm_iter.as_mut() {
            None => unimplemented!("calling 'next_alarm' on Some price alarms"),
            Some(iter) => iter.next(),
        }
    }
}

impl<'storage, S, I, AlarmsG, BaseC, BaseG, ErrorG> Iterator
    for Iter<'storage, '_, S, I, AlarmsG, BaseC, BaseG, ErrorG>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    I: Iterator<Item = PriceResult<AlarmsG, BaseC, BaseG, ErrorG>>,
    AlarmsG: Group,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<AlarmsG::TopG>,
    BaseG: Group,
    ErrorG: Group,
{
    type Item = Result<Addr, ErrorG>;

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
            };
        }
        result
    }
}

struct Cmd<'storage, 'alarms, S, AlarmsG, BaseC, ErrorG>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    AlarmsG: Group + Clone,
    BaseC: Currency,
{
    alarms: &'alarms PriceAlarms<'storage, AlarmsG, S>,
    _base_c: PhantomData<BaseC>,
    _error_g: PhantomData<ErrorG>,
}

impl<'storage, 'alarms, S, AlarmsG, BaseC, ErrorG> WithPrice<BaseC>
    for Cmd<'storage, 'alarms, S, AlarmsG, BaseC, ErrorG>
where
    S: Deref<Target = (dyn Storage + 'storage)>,
    AlarmsG: Group,
    BaseC: CurrencyDef,
    ErrorG: Group,
{
    type PriceG = AlarmsG;

    type Output = AlarmIter<'alarms, AlarmsG, ErrorG>;
    type Error = Error<ErrorG>;

    fn exec<C>(self, price: Price<C, BaseC>) -> std::result::Result<Self::Output, Self::Error>
    where
        C: CurrencyDef,
        C::Group: MemberOf<Self::PriceG>,
    {
        Ok(self
            .alarms
            .alarms(price)
            .map(|may_alarm| may_alarm.map_err(Into::into)))
    }
}
