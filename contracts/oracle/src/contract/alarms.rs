use finance::{
    currency::{self, AnyVisitor, AnyVisitorResult, Currency},
    price::{
        base::BasePrice,
        dto::{with_quote, WithQuote},
        Price,
    },
};
use marketprice::{
    alarms::{errors::AlarmError, AlarmsIterator, PriceAlarms},
    SpotPrice,
};
use sdk::cosmwasm_std::{Addr, Storage};
use serde::{de::DeserializeOwned, Serialize};
use swap::SwapGroup;

use crate::{alarms::Alarm as AlarmDTO, msg::AlarmsStatusResponse, ContractError};

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

    pub fn notify_alarms_iter<'a, BaseC>(
        storage: &'a dyn Storage,
        prices: impl Iterator<Item = BasePrice<SwapGroup, BaseC>> + 'a,
        max_count: usize,
    ) -> impl Iterator<Item = Result<Addr, ContractError>> + 'a
    where
        BaseC: Currency,
    {
        Self::alarms_iter::<BaseC>(storage, prices)
            .take(max_count)
            .map(|item| item.map_err(Into::into))
    }

    pub fn try_query_alarms<'a, BaseC>(
        storage: &dyn Storage,
        prices: impl Iterator<Item = BasePrice<SwapGroup, BaseC>> + 'a,
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
        prices: impl Iterator<Item = BasePrice<SwapGroup, BaseC>> + 'a,
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
            price: &'b BasePrice<SwapGroup, OracleBase>,
        }

        impl<'a, 'b, OracleBase> AnyVisitor for AlarmsCmd<'a, 'b, OracleBase>
        where
            OracleBase: Currency,
        {
            type Error = ContractError;
            type Output = AlarmsIterator<'a>;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency + Serialize + DeserializeOwned,
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

    use crate::tests::{self, TheCurrency as Base};
    use ::currency::lease::{Atom, Weth};
    use sdk::cosmwasm_std::testing::MockStorage;

    fn alarm_dto<C>(below: (u128, u128), above: Option<(u128, u128)>) -> AlarmDTO
    where
        C: Currency,
    {
        AlarmDTO::new(
            tests::dto_price::<C, Base>(below.0, below.1),
            above.map(|above| tests::dto_price::<C, Base>(above.0, above.1)),
        )
    }

    fn add_alarms<'a>(
        storage: &mut dyn Storage,
        mut alarms: impl Iterator<Item = (&'a str, AlarmDTO)>,
    ) -> Result<(), ContractError> {
        alarms.try_for_each(|(receiver, alarm)| -> Result<(), ContractError> {
            MarketAlarms::try_add_price_alarm::<Base>(storage, Addr::unchecked(receiver), alarm)
        })
    }

    fn test_case(storage: &mut dyn Storage) {
        add_alarms(
            storage,
            [
                ("recv2", alarm_dto::<Weth>((1, 20), Some((1, 50)))),
                ("recv1", alarm_dto::<Weth>((1, 10), Some((1, 60)))),
                ("recv3", alarm_dto::<Atom>((1, 20), Some((1, 60)))),
                ("recv4", alarm_dto::<Atom>((1, 30), Some((1, 70)))),
            ]
            .into_iter(),
        )
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn wrong_base_currency() {
        let mut storage = MockStorage::new();

        let receiver = Addr::unchecked("receiver");

        let _ = MarketAlarms::try_add_price_alarm::<Base>(
            &mut storage,
            receiver,
            AlarmDTO::new(tests::dto_price::<Base, Atom>(1, 20), None),
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
            alarm_dto::<Atom>((1, 20), None),
        )
        .unwrap();

        MarketAlarms::try_add_price_alarm::<Base>(
            &mut storage,
            receiver2.clone(),
            alarm_dto::<Weth>((1, 20), Some((1, 30))),
        )
        .unwrap();

        assert!(
            MarketAlarms::try_query_alarms::<Base>(
                &storage,
                [tests::base_price::<Weth>(1, 35)].into_iter()
            )
            .unwrap()
            .remaining_alarms
        );

        MarketAlarms::remove(&mut storage, receiver2).unwrap();

        assert!(
            !MarketAlarms::try_query_alarms::<Base>(
                &storage,
                [tests::base_price::<Weth>(1, 10)].into_iter()
            )
            .unwrap()
            .remaining_alarms
        );
    }

    #[test]
    #[should_panic]
    #[cfg(not(debug_assertions))]
    fn notify_with_wrong_currency_group() {
        use ::currency::native::Nls;
        use finance::{coin::Coin, price};

        let mut storage = MockStorage::new();

        let _: Vec<_> = MarketAlarms::notify_alarms_iter::<Base>(
            &mut storage,
            [price::total_of(Coin::<Nls>::new(1))
                .is(Coin::<Base>::new(25))
                .into()]
            .into_iter(),
            1,
        )
        .collect();
    }

    #[test]
    fn alarms_below_none() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let mut sent = MarketAlarms::notify_alarms_iter::<Base>(
            &storage,
            [tests::base_price::<Weth>(1, 25)].into_iter(),
            100,
        );

        assert!(sent.next().is_none());
    }

    #[test]
    fn alarms_below_mid() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let sent: Vec<_> = MarketAlarms::notify_alarms_iter::<Base>(
            &storage,
            [tests::base_price::<Weth>(1, 15)].into_iter(),
            100,
        )
        .flatten()
        .collect();

        assert_eq!(sent, vec!["recv2"]);
    }

    #[test]
    fn alarms_below_all() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let sent: Vec<_> = MarketAlarms::notify_alarms_iter::<Base>(
            &storage,
            [tests::base_price::<Weth>(1, 5)].into_iter(),
            100,
        )
        .flatten()
        .collect();

        assert_eq!(sent, vec!["recv2", "recv1"]);
    }

    #[test]
    fn alarms_above_none() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let mut sent = MarketAlarms::notify_alarms_iter::<Base>(
            &storage,
            [tests::base_price::<Weth>(1, 25)].into_iter(),
            100,
        );

        assert!(sent.next().is_none());
    }

    #[test]
    fn alarms_above_mid() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let sent: Vec<_> = MarketAlarms::notify_alarms_iter::<Base>(
            &storage,
            [tests::base_price::<Weth>(1, 55)].into_iter(),
            100,
        )
        .flatten()
        .collect();

        assert_eq!(sent, vec!["recv2"]);
    }

    #[test]
    fn alarms_above_all() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let sent: Vec<_> = MarketAlarms::notify_alarms_iter::<Base>(
            &storage,
            [tests::base_price::<Weth>(1, 65)].into_iter(),
            100,
        )
        .flatten()
        .collect();

        assert_eq!(sent, vec!["recv1", "recv2"]);
    }

    #[test]
    fn alarms_mixed() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let sent: Vec<_> = MarketAlarms::notify_alarms_iter::<Base>(
            &storage,
            [
                tests::base_price::<Weth>(1, 65),
                tests::base_price::<Atom>(1, 25),
            ]
            .into_iter(),
            100,
        )
        .flatten()
        .collect();

        assert_eq!(sent, vec!["recv1", "recv2", "recv4"]);
    }

    #[test]
    fn alarms_max_count() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let sent: Vec<_> = MarketAlarms::notify_alarms_iter::<Base>(
            &storage,
            [
                tests::base_price::<Weth>(1, 65),
                tests::base_price::<Atom>(1, 15),
            ]
            .into_iter(),
            3,
        )
        .flatten()
        .collect();

        assert_eq!(sent, vec!["recv1", "recv2", "recv4"]);
    }
}
