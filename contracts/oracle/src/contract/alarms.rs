use ::currency::native::Nls;
use finance::{
    currency::{self, AnyVisitor, Currency},
    price::{
        dto::{with_quote, BasePrice, WithQuote},
        Price,
    },
};
use marketprice::{
    alarms::{errors::AlarmError, AlarmsIterator, PriceAlarms},
    SpotPrice,
};
use platform::batch::Batch;
use sdk::cosmwasm_std::{Addr, Storage};
use swap::SwapGroup;

use crate::{
    alarms::Alarm as AlarmDTO,
    msg::{AlarmsStatusResponse, ExecuteAlarmMsg},
    ContractError,
};

pub struct MarketAlarms;

impl MarketAlarms {
    const PRICE_ALARMS: PriceAlarms<'static> =
        PriceAlarms::new("alarms_below", "index_below", "alarms_above", "index_above");

    pub fn remove(storage: &mut dyn Storage, receiver: Addr) -> Result<(), ContractError> {
        Self::PRICE_ALARMS.remove(storage, receiver)?;
        Ok(())
    }

    pub fn try_add_price_alarm<BaseC>(
        storage: &mut dyn Storage,
        receiver: Addr,
        alarm: AlarmDTO,
    ) -> Result<(), ContractError>
    where
        BaseC: Currency,
    {
        struct AddAlarms<'m> {
            storage: &'m mut dyn Storage,
            receiver: Addr,
            above: Option<SpotPrice>,
            price_alarms: PriceAlarms<'m>,
        }

        impl<'m, BaseC> WithQuote<BaseC> for AddAlarms<'m>
        where
            BaseC: Currency,
        {
            type Output = ();
            type Error = ContractError;

            fn exec<C>(self, below: Price<C, BaseC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
            {
                if let Some(above) = self.above {
                    self.price_alarms.add_alarm_above::<C, BaseC>(
                        self.storage,
                        &self.receiver,
                        above.try_into()?,
                    )?;
                }
                self.price_alarms
                    .add_alarm_below(self.storage, &self.receiver, below)?;
                Ok(())
            }
        }

        let (below, above) = alarm.into();
        with_quote::execute::<_, _, _, BaseC>(
            &below,
            AddAlarms {
                storage,
                receiver,
                above,
                price_alarms: Self::PRICE_ALARMS,
            },
        )
    }

    pub fn try_notify_alarms<'a, BaseC>(
        &mut self,
        storage: &dyn Storage,
        prices: impl Iterator<Item = BasePrice<BaseC, SwapGroup>> + 'a,
        max_count: u32, // TODO: type alias
    ) -> Result<Batch, ContractError>
    where
        BaseC: Currency,
    {
        let batch = Self::alarms_iter::<BaseC>(storage, prices)
            .take(max_count.try_into()?)
            .try_fold(
                Batch::default(),
                |mut batch, receiver| -> Result<Batch, ContractError> {
                    // TODO: get rid of the Nls dummy type argument
                    batch.schedule_execute_wasm_reply_always::<_, Nls>(
                        &receiver?,
                        ExecuteAlarmMsg::PriceAlarm(),
                        None,
                        batch.len().try_into()?,
                    )?;
                    Ok(batch)
                },
            )?;

        // let processed = msgs.len().try_into()?;
        // let batch = msgs.merge(batch);

        // Ok(Response::from(batch).set_data(to_binary(&DispatchAlarmsResponse(processed))?))
        Ok(batch)
    }

    pub fn try_query_alarms<'a, BaseC>(
        storage: &dyn Storage,
        prices: impl Iterator<Item = BasePrice<BaseC, SwapGroup>> + 'a,
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
        prices: impl Iterator<Item = BasePrice<BaseC, SwapGroup>> + 'a,
    ) -> impl Iterator<Item = Result<Addr, AlarmError>> + 'a
    where
        BaseC: Currency,
    {
        struct AlarmsCmd<'a, 'b, OracleBase>
        where
            OracleBase: Currency,
        {
            storage: &'a dyn Storage,
            price_alarms: &'static PriceAlarms<'static>,
            price: &'b BasePrice<OracleBase, SwapGroup>,
        }

        impl<'a, 'b, OracleBase> AnyVisitor for AlarmsCmd<'a, 'b, OracleBase>
        where
            OracleBase: Currency,
        {
            type Error = ContractError;
            type Output = AlarmsIterator<'a>;

            fn on<C>(self) -> finance::currency::AnyVisitorResult<Self>
            where
                C: Currency + serde::Serialize + serde::de::DeserializeOwned,
            {
                Ok(self
                    .price_alarms
                    .alarms::<C, OracleBase>(self.storage, self.price.try_into()?))
            }
        }

        prices.flat_map(|price| {
            currency::visit_any_on_ticker::<SwapGroup, _>(
                price.base_ticker(),
                AlarmsCmd {
                    storage,
                    price_alarms: &Self::PRICE_ALARMS,
                    price: &price,
                },
            )
            .expect("Invalid price")
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use ::currency::{
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

        let receiver = Addr::unchecked("receiver");

        let _ = MarketAlarms::try_add_price_alarm::<Base>(
            &mut storage,
            receiver,
            AlarmDTO::new(
                price::total_of(Coin::<Base>::new(1)).is(Coin::<Atom>::new(20)),
                None,
            ),
        );
    }

    #[test]
    fn add_remove() {
        let mut storage = MockStorage::new();

        let receiver1 = Addr::unchecked("receiver1");
        let receiver2 = Addr::unchecked("receiver2");

        MarketAlarms::try_add_price_alarm::<Base>(
            &mut storage,
            receiver1,
            AlarmDTO::new(
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20)),
                None,
            ),
        )
        .unwrap();

        MarketAlarms::try_add_price_alarm::<Base>(
            &mut storage,
            receiver2.clone(),
            AlarmDTO::new(
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(20)),
                Some(price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(30))),
            ),
        )
        .unwrap();

        assert!(
            MarketAlarms::try_query_alarms::<Base>(
                &storage,
                [price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(35))
                    .into(),]
                .into_iter()
            )
            .unwrap()
            .remaining_alarms
        );

        MarketAlarms::remove(&mut storage, receiver2).unwrap();

        assert!(
            !MarketAlarms::try_query_alarms::<Base>(
                &storage,
                [price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(10))
                    .into(),]
                .into_iter()
            )
            .unwrap()
            .remaining_alarms
        );
    }

    #[test]
    #[should_panic]
    #[cfg(not(debug_assertions))]
    fn notify_with_wrong_currency_group() {
        let mut storage = MockStorage::new();

        let batch = Batch::default();

        let _ = MarketAlarms.try_notify_alarms::<Base>(
            &mut storage,
            batch,
            [price::total_of(Coin::<Nls>::new(1))
                .is(Coin::<Base>::new(25))
                .into()]
            .into_iter(),
            1,
        );
    }

    #[test]
    fn notify() {
        let mut storage = MockStorage::new();

        for x in 0..=5 {
            let delta = x * 10;

            let receiver = Addr::unchecked(format!("receiver1_{}", delta));

            MarketAlarms::try_add_price_alarm::<Base>(
                &mut storage,
                receiver,
                AlarmDTO::new(
                    price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(10 + delta)),
                    Some(price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(30 + delta))),
                ),
            )
            .unwrap();

            let receiver = Addr::unchecked(format!("receiver2_{}", delta));

            MarketAlarms::try_add_price_alarm::<Base>(
                &mut storage,
                receiver,
                AlarmDTO::new(
                    price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(50 + delta)),
                    None,
                ),
            )
            .unwrap();
        }

        let sent = MarketAlarms
            .try_notify_alarms::<Base>(
                &storage,
                [price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(25))
                    .into()]
                .into_iter(),
                3,
            )
            .unwrap()
            .len();
        assert_eq!(sent, 3);

        let sent = MarketAlarms
            .try_notify_alarms::<Base>(
                &storage,
                [
                    price::total_of(Coin::<Atom>::new(1))
                        .is(Coin::<Base>::new(35))
                        .into(),
                    price::total_of(Coin::<Weth>::new(1))
                        .is(Coin::<Base>::new(20))
                        .into(),
                ]
                .into_iter(),
                100,
            )
            .unwrap()
            .len();
        assert_eq!(sent, 10);
    }
}
