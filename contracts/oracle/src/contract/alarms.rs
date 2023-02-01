use crate::{alarms::Alarm as AlarmDTO, msg::ExecuteAlarmMsg};
use currency::native::Nls;
use finance::{
    currency::Currency,
    price::{
        dto::{with_quote, WithQuote},
        Price,
    },
};
use marketprice::{
    alarms::{errors::AlarmError, AlarmsIterator, PriceAlarms},
    SpotPrice,
};
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

    pub fn try_notify_alarms<BaseC>(
        storage: &mut dyn Storage,
        mut batch: Batch,
        prices: &[SpotPrice],
        max_count: u32, // TODO: type alias
    ) -> Result<Response, ContractError>
    where
        BaseC: Currency,
    {
        let mut next_id = Self::MSG_ID.may_load(storage)?.unwrap_or_default();
        let initial_id = next_id;

        Self::alarms_iter::<BaseC>(storage, prices)
            .take(max_count.try_into()?)
            .try_for_each(|addr| Self::schedule_alarm(&mut batch, addr?, &mut next_id))?;

        Self::MSG_ID.save(storage, &next_id)?;

        let processed = next_id.wrapping_sub(initial_id);

        Ok(Response::from(batch)
            .set_data(to_binary(&DispatchAlarmsResponse(processed.try_into()?))?))
    }

    pub fn try_query_alarms<BaseC>(
        storage: &dyn Storage,
        prices: &[SpotPrice],
    ) -> Result<AlarmsStatusResponse, ContractError>
    where
        BaseC: Currency,
    {
        Ok(AlarmsStatusResponse {
            remaining_alarms: Self::alarms_iter::<BaseC>(storage, prices)
                .next()
                .transpose()?
                .is_some(),
        })
    }

    fn alarms_iter<'a, BaseC>(
        storage: &'a dyn Storage,
        prices: &'a [SpotPrice],
    ) -> impl Iterator<Item = Result<Addr, AlarmError>> + 'a
    where
        BaseC: Currency,
    {
        struct AlarmsCmd<'a> {
            storage: &'a dyn Storage,
            price_alarms: &'static PriceAlarms<'static>,
        }

        impl<'a, BaseC> WithQuote<BaseC> for AlarmsCmd<'a>
        where
            BaseC: Currency,
        {
            type Error = ContractError;
            type Output = AlarmsIterator<'a>;

            fn exec<C>(self, price: Price<C, BaseC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
            {
                Ok(self.price_alarms.alarms(self.storage, price))
            }
        }

        prices.iter().flat_map(|price| {
            with_quote::execute::<_, _, _, BaseC>(
                price,
                AlarmsCmd {
                    storage,
                    price_alarms: &Self::PRICE_ALARMS,
                },
            )
            .expect("Invalid price")
        })
    }

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
    #[should_panic]
    fn notify_with_wrong_base() {
        let mut storage = MockStorage::new();

        let batch = Batch::default();

        let _ = MarketAlarms::try_notify_alarms::<Base>(
            &mut storage,
            batch,
            &[price::total_of(Coin::<Base>::new(1))
                .is(Coin::<Atom>::new(25))
                .into()],
            1,
        );
    }

    #[test]
    #[should_panic]
    #[cfg(not(debug_assertions))]
    fn notify_with_wrong_currency_group() {
        let mut storage = MockStorage::new();

        let batch = Batch::default();

        let _ = MarketAlarms::try_notify_alarms::<Base>(
            &mut storage,
            batch,
            &[price::total_of(Coin::<Nls>::new(1))
                .is(Coin::<Base>::new(25))
                .into()],
            1,
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

        // check msg_id wrapping
        let id: Item<AlarmReplyId> = Item::new("msg_id");
        id.save(&mut storage, &(AlarmReplyId::MAX - 5)).unwrap();

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
