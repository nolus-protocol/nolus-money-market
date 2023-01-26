use crate::{alarms::Alarm as AlarmDTO, msg::ExecuteAlarmMsg};
use currency::native::Nls;
use finance::{
    currency::Currency,
    price::{
        dto::{with_quote, WithQuote},
        Price,
    },
};
use marketprice::{alarms::PriceAlarms, SpotPrice};
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Addr, Storage},
    cw_storage_plus::Item,
};

use crate::{
    msg::{AlarmsStatusResponse, DispatchAlarmsResponse},
    ContractError,
};

pub type AlarmReplyId = u64;

pub struct MarketAlarms;

impl MarketAlarms {
    const PRICE_ALARMS: PriceAlarms<'static> =
        PriceAlarms::new("alarms_below", "index_below", "alarms_above", "index_above");

    const MSG_ID: Item<'static, AlarmReplyId> = Item::new("msg_id");

    pub fn remove(storage: &mut dyn Storage, addr: Addr) -> Result<Response, ContractError> {
        Self::PRICE_ALARMS.remove(storage, addr)?;
        Ok(Response::default())
    }

    pub fn try_add_price_alarm<BaseC>(
        storage: &mut dyn Storage,
        addr: Addr,
        alarm: AlarmDTO,
    ) -> Result<Response, ContractError>
    where
        BaseC: Currency,
    {
        struct AddAlarms<'m> {
            storage: &'m mut dyn Storage,
            addr: Addr,
            above: Option<SpotPrice>,
            price_alarms: PriceAlarms<'m>,
        }

        impl<'m, BaseC> WithQuote<BaseC> for AddAlarms<'m>
        where
            BaseC: Currency,
        {
            type Output = Response;
            type Error = ContractError;

            fn exec<C>(self, below: Price<C, BaseC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
            {
                if let Some(above) = self.above {
                    self.price_alarms.add_alarm_above::<C, BaseC>(
                        self.storage,
                        &self.addr,
                        above.try_into()?,
                    )?;
                }
                self.price_alarms
                    .add_alarm_below(self.storage, &self.addr, below)?;
                Ok(Response::new())
            }
        }

        let (below, above) = alarm.into();
        with_quote::execute::<_, _, _, BaseC>(
            &below,
            AddAlarms {
                storage,
                addr,
                above,
                price_alarms: Self::PRICE_ALARMS,
            },
        )
    }

    // fn alarms_iter<'a, BaseC>(storage: &'a dyn Storage, price: &SpotPrice) -> Result<impl Iterator<Item = StdResult<Addr>> + 'a, ContractError>
    //     where BaseC: Currency,
    // {

    //     struct AlarmsCmd<'a> {
    //         storage: &'a dyn Storage,
    //         price_alarms: &'static PriceAlarms<'static>,
    //     }

    //     impl<'a, BaseC> WithQuote<BaseC> for AlarmsCmd<'a>
    //         where
    //         BaseC: Currency,
    //     {
    //         type Error = ContractError;
    //         type Output = impl Iterator<Item = StdResult<Addr>> + 'a;
    //         fn exec<C>(self, price: Price<C, BaseC>) -> Result<Self::Output, Self::Error>
    //             where
    //                 C: Currency {
    //             Ok(self.price_alarms.alarms(self.storage, price))
    //         }
    //     }

    //     with_quote::execute::<_,_,_, BaseC>(price, AlarmsCmd {storage, price_alarms: &Self::PRICE_ALARMS })
    // }

    fn schedule_alarm(
        batch: &mut Batch,
        addr: Addr,
        next_id: &mut AlarmReplyId,
    ) -> Result<(), ContractError> {
        // TODO: get rid of the Nls dummy type argument
        batch.schedule_execute_wasm_reply_always::<_, Nls>(
            &addr,
            ExecuteAlarmMsg::PriceAlarm(),
            None,
            *next_id,
        )?;

        *next_id = next_id.wrapping_add(1);

        Ok(())
    }

    pub fn try_notify_alarms<BaseC>(
        storage: &mut dyn Storage,
        mut batch: Batch,
        prices: &[SpotPrice],
        max_count: u32, // TODO: type alias
    ) -> Result<Response, ContractError>
    where
        BaseC: Currency,
    {
        struct NotifyCmd<'a> {
            storage: &'a dyn Storage,
            price_alarms: &'a PriceAlarms<'static>,
            max_count: u32,
            next_id: &'a mut AlarmReplyId,
            batch: &'a mut Batch,
        }

        impl<'a, BaseC> WithQuote<BaseC> for NotifyCmd<'a>
        where
            BaseC: Currency,
        {
            type Error = ContractError;
            type Output = u32;

            fn exec<C>(self, price: Price<C, BaseC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
            {
                let initial = *self.next_id;

                self.price_alarms
                    .alarms(self.storage, price)
                    .take(self.max_count as usize)
                    .try_for_each(|addr_result| {
                        addr_result.map(|addr| {
                            MarketAlarms::schedule_alarm(self.batch, addr, self.next_id)
                        })?
                    })?;
                let processed = *self.next_id - initial;
                Ok(processed.try_into()?)
            }
        }

        let mut next_id = Self::MSG_ID.may_load(storage)?.unwrap_or_default();
        let mut processed = 0u32;

        prices
            .iter()
            .try_for_each(|price| -> Result<(), ContractError> {
                processed += with_quote::execute::<_, _, _, BaseC>(
                    price,
                    NotifyCmd {
                        storage,
                        price_alarms: &Self::PRICE_ALARMS,
                        max_count: max_count - processed,
                        next_id: &mut next_id,
                        batch: &mut batch,
                    },
                )?;
                Ok(())
            })?;

        Self::MSG_ID.save(storage, &next_id)?;

        Ok(Response::from(batch).set_data(to_binary(&DispatchAlarmsResponse(processed))?))
    }

    pub fn try_query_alarms<BaseC>(
        storage: &dyn Storage,
        prices: &[SpotPrice],
    ) -> Result<AlarmsStatusResponse, ContractError>
    where
        BaseC: Currency,
    {
        struct QueryCmd<'a> {
            storage: &'a dyn Storage,
            price_alarms: &'a PriceAlarms<'static>,
        }

        impl<'a, BaseC> WithQuote<BaseC> for QueryCmd<'a>
        where
            BaseC: Currency,
        {
            type Error = ContractError;
            type Output = bool;

            fn exec<C>(self, price: Price<C, BaseC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
            {
                Ok(self.price_alarms.alarms(self.storage, price).any(|_| true))
            }
        }

        let remaining_alarms = prices
            .iter()
            .flat_map(|price| {
                with_quote::execute::<_, _, _, BaseC>(
                    price,
                    QueryCmd {
                        storage,
                        price_alarms: &Self::PRICE_ALARMS,
                    },
                )
            })
            .any(|remaining_alarms| remaining_alarms);

        Ok(AlarmsStatusResponse { remaining_alarms })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::from_binary;
    use currency::{
        lease::{Atom, Weth},
        lpn::Usdc,
    };
    use finance::{coin::Coin, price};
    use sdk::cosmwasm_std::testing::MockStorage;

    type Base = Usdc;

    #[test]
    #[should_panic]
    fn wrong_base_currency() {
        let mut storage = MockStorage::new();

        let addr = Addr::unchecked("addr");

        let _ = MarketAlarms::try_add_price_alarm::<Base>(
            &mut storage,
            addr,
            AlarmDTO::new(
                price::total_of(Coin::<Base>::new(1)).is(Coin::<Atom>::new(20)),
                None,
            ),
        );
    }

    #[test]
    fn add_remove() {
        let mut storage = MockStorage::new();

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");

        MarketAlarms::try_add_price_alarm::<Base>(
            &mut storage,
            addr1,
            AlarmDTO::new(
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20)),
                None,
            ),
        )
        .unwrap();

        MarketAlarms::try_add_price_alarm::<Base>(
            &mut storage,
            addr2.clone(),
            AlarmDTO::new(
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(20)),
                Some(price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(30))),
                // None,
            ),
        )
        .unwrap();

        assert!(
            MarketAlarms::try_query_alarms::<Base>(
                &storage,
                &[price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(35))
                    .into(),]
            )
            .unwrap()
            .remaining_alarms
        );

        MarketAlarms::remove(&mut storage, addr2).unwrap();

        assert!(
            !MarketAlarms::try_query_alarms::<Base>(
                &storage,
                &[price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(10))
                    .into(),]
            )
            .unwrap()
            .remaining_alarms
        );
    }

    #[test]
    fn notify() {
        let mut storage = MockStorage::new();

        for x in 0..=5 {
            let delta = x * 10;

            let addr = Addr::unchecked(format!("addr1_{}", delta));

            MarketAlarms::try_add_price_alarm::<Base>(
                &mut storage,
                addr,
                AlarmDTO::new(
                    price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(10 + delta)),
                    Some(price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(30 + delta))),
                ),
            )
            .unwrap();

            let addr = Addr::unchecked(format!("addr2_{}", delta));

            MarketAlarms::try_add_price_alarm::<Base>(
                &mut storage,
                addr,
                AlarmDTO::new(
                    price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(50 + delta)),
                    None,
                ),
            )
            .unwrap();
        }

        let batch = Batch::default();

        let sent = from_binary::<DispatchAlarmsResponse>(
            &MarketAlarms::try_notify_alarms::<Base>(
                &mut storage,
                batch,
                &[price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(25))
                    .into()],
                3,
            )
            .unwrap()
            .data
            .unwrap(),
        )
        .unwrap()
        .0;
        assert_eq!(sent, 3);

        let batch = Batch::default();

        let sent = from_binary::<DispatchAlarmsResponse>(
            &MarketAlarms::try_notify_alarms::<Base>(
                &mut storage,
                batch,
                &[
                    price::total_of(Coin::<Atom>::new(1))
                        .is(Coin::<Base>::new(35))
                        .into(),
                    price::total_of(Coin::<Weth>::new(1))
                        .is(Coin::<Base>::new(20))
                        .into(),
                ],
                100,
            )
            .unwrap()
            .data
            .unwrap(),
        )
        .unwrap()
        .0;
        assert_eq!(sent, 10);
    }
}
