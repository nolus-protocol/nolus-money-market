use std::ops::{Deref, DerefMut};

use currency::Currency;
use finance::price::{
    base::BasePrice,
    dto::{with_quote, WithQuote},
    Price,
};
use marketprice::{alarms::PriceAlarms, SpotPrice};
use sdk::cosmwasm_std::{Addr, Storage};
use swap::SwapGroup;

use crate::{alarms::Alarm as AlarmDTO, error::ContractError, result::ContractResult};

use self::iter::Iter as AlarmsIter;

mod iter;

pub type PriceResult<BaseC> = ContractResult<BasePrice<SwapGroup, BaseC>>;

const NAMESPACE_ALARMS_BELOW: &str = "alarms_below";
const NAMESPACE_INDEX_BELOW: &str = "index_below";
const NAMESPACE_ALARMS_ABOVE: &str = "alarms_above";
const NAMESPACE_INDEX_ABOVE: &str = "index_above";
const NAMESPACE_IN_DELIVERY: &str = "in_delivery";

pub(super) struct MarketAlarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    alarms: PriceAlarms<'storage, S>,
}

impl<'storage, S> MarketAlarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    pub fn new(storage: S) -> Self {
        Self {
            alarms: PriceAlarms::new(
                storage,
                NAMESPACE_ALARMS_BELOW,
                NAMESPACE_INDEX_BELOW,
                NAMESPACE_ALARMS_ABOVE,
                NAMESPACE_INDEX_ABOVE,
                NAMESPACE_IN_DELIVERY,
            ),
        }
    }

    pub fn notify_alarms_iter<I, BaseC>(
        &self,
        prices: I,
    ) -> ContractResult<AlarmsIter<'storage, '_, S, I, BaseC>>
    where
        I: Iterator<Item = PriceResult<BaseC>>,
        BaseC: Currency,
    {
        AlarmsIter::new(&self.alarms, prices)
    }

    pub fn try_query_alarms<I, BaseC>(&self, prices: I) -> Result<bool, ContractError>
    where
        I: Iterator<Item = PriceResult<BaseC>>,
        BaseC: Currency,
    {
        Ok(AlarmsIter::new(&self.alarms, prices)?
            .next()
            .transpose()?
            .is_some())
    }

    pub fn ensure_no_in_delivery(&self) -> ContractResult<&Self> {
        self.alarms
            .ensure_no_in_delivery()
            .map(|()| self)
            .map_err(Into::into)
    }
}

impl<'storage, S> MarketAlarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    pub fn try_add_price_alarm<BaseC>(
        &mut self,
        receiver: Addr,
        alarm: AlarmDTO,
    ) -> Result<(), ContractError>
    where
        BaseC: Currency,
    {
        let (below, above_or_equal) = alarm.into();

        with_quote::execute::<_, _, _, BaseC>(
            &below,
            AddAlarmsCmd {
                receiver,
                above_or_equal,
                price_alarms: &mut self.alarms,
            },
        )
    }

    pub fn out_for_delivery(&mut self, subscriber: Addr) -> ContractResult<()> {
        self.alarms.out_for_delivery(subscriber).map_err(Into::into)
    }

    pub fn last_delivered(&mut self) -> ContractResult<()> {
        self.alarms.last_delivered().map_err(Into::into)
    }

    pub fn last_failed(&mut self) -> ContractResult<()> {
        self.alarms.last_failed().map_err(Into::into)
    }

    #[cfg(test)]
    fn remove(&mut self, receiver: Addr) -> Result<(), ContractError> {
        self.alarms.remove_all(receiver).map_err(Into::into)
    }
}

struct AddAlarmsCmd<'storage, 'alarms, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    receiver: Addr,
    above_or_equal: Option<SpotPrice>,
    price_alarms: &'alarms mut PriceAlarms<'storage, S>,
}

impl<'storage, 'alarms, S, BaseC> WithQuote<BaseC> for AddAlarmsCmd<'storage, 'alarms, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
    BaseC: Currency,
{
    type Output = ();
    type Error = ContractError;

    fn exec<C>(self, below: Price<C, BaseC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
    {
        if let Some(above) = self.above_or_equal {
            self.price_alarms
                .add_alarm_above_or_equal::<C, BaseC>(self.receiver.clone(), above.try_into()?)?;
        } else {
            self.price_alarms
                .remove_above_or_equal(self.receiver.clone())?;
        }

        self.price_alarms
            .add_alarm_below(self.receiver.clone(), below)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use ::currency::lease::{Atom, Weth};
    use currency::lease::Juno;
    use sdk::cosmwasm_std::testing::MockStorage;

    use crate::tests::{self, TheCurrency as Base};

    use super::*;

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
        mut storage: &mut dyn Storage,
        mut alarms: impl Iterator<Item = (&'a str, AlarmDTO)>,
    ) -> Result<(), ContractError> {
        alarms.try_for_each(|(receiver, alarm)| -> Result<(), ContractError> {
            MarketAlarms::new(storage.deref_mut())
                .try_add_price_alarm::<Base>(Addr::unchecked(receiver), alarm)
        })
    }

    pub fn test_case(storage: &mut dyn Storage) {
        add_alarms(
            storage,
            [
                ("recv2", alarm_dto::<Weth>((1, 20), Some((1, 50)))),
                ("recv1", alarm_dto::<Weth>((1, 10), Some((1, 60)))),
                ("recv3", alarm_dto::<Atom>((1, 20), Some((1, 60)))),
                ("recv4", alarm_dto::<Atom>((1, 30), Some((1, 70)))),
                ("recv5", alarm_dto::<Juno>((1, 30), Some((1, 70)))),
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

        let _ = MarketAlarms::new(&mut storage as &mut dyn Storage).try_add_price_alarm::<Base>(
            receiver,
            AlarmDTO::new(tests::dto_price::<Base, Atom>(1, 20), None),
        );
    }

    #[test]
    fn add_remove() {
        let mut storage = MockStorage::new();
        let mut alarms = MarketAlarms::new(&mut storage as &mut dyn Storage);

        let receiver1 = Addr::unchecked("receiver1");
        let receiver2 = Addr::unchecked("receiver2");

        alarms
            .try_add_price_alarm::<Base>(receiver1, alarm_dto::<Atom>((1, 20), None))
            .unwrap();

        alarms
            .try_add_price_alarm::<Base>(
                receiver2.clone(),
                alarm_dto::<Weth>((1, 20), Some((1, 30))),
            )
            .unwrap();

        assert!(!alarms
            .try_query_alarms::<_, Base>(
                [
                    tests::base_price::<Atom>(1, 20),
                    tests::base_price::<Weth>(1, 20)
                ]
                .into_iter()
                .map(Ok),
            )
            .unwrap());

        assert!(alarms
            .try_query_alarms::<_, Base>([tests::base_price::<Weth>(1, 35)].into_iter().map(Ok),)
            .unwrap());

        alarms.remove(receiver2).unwrap();

        assert!(!alarms
            .try_query_alarms::<_, Base>([tests::base_price::<Weth>(1, 10)].into_iter().map(Ok))
            .unwrap());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn notify_with_wrong_currency_group() {
        use currency::test::Dai;

        let storage = MockStorage::new();

        let alarms = MarketAlarms::new(&storage as &dyn Storage);
        let res = alarms
            .notify_alarms_iter::<_, Base>([tests::base_price::<Dai>(1, 25)].into_iter().map(Ok));
        assert!(res.is_err())
    }

    #[test]
    fn alarms_no_pices() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let alarms = MarketAlarms::new(&storage as &dyn Storage);

        let mut sent = alarms
            .notify_alarms_iter::<_, Base>([].into_iter().map(Ok))
            .unwrap();

        assert!(sent.next().is_none());
    }

    #[test]
    fn alarms_below_none() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let alarms = MarketAlarms::new(&storage as &dyn Storage);

        let mut sent = alarms
            .notify_alarms_iter::<_, Base>([tests::base_price::<Weth>(1, 25)].into_iter().map(Ok))
            .unwrap();

        assert!(sent.next().is_none());
    }

    #[test]
    fn alarms_below_mid() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let sent: Vec<_> = MarketAlarms::new(&storage as &dyn Storage)
            .notify_alarms_iter::<_, Base>([tests::base_price::<Weth>(1, 15)].into_iter().map(Ok))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(sent, vec!["recv2"]);
    }

    #[test]
    fn alarms_below_all() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let sent: Vec<_> = MarketAlarms::new(&storage as &dyn Storage)
            .notify_alarms_iter::<_, Base>([tests::base_price::<Weth>(1, 5)].into_iter().map(Ok))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(sent, vec!["recv2", "recv1"]);
    }

    #[test]
    fn alarms_above_none() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let alarms = MarketAlarms::new(&storage as &dyn Storage);

        let mut sent = alarms
            .notify_alarms_iter::<_, Base>([tests::base_price::<Weth>(1, 25)].into_iter().map(Ok))
            .unwrap();

        assert!(sent.next().is_none());
    }

    #[test]
    fn alarms_above_mid() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let alarms = MarketAlarms::new(&storage as &dyn Storage);

        let sent: Vec<_> = alarms
            .notify_alarms_iter::<_, Base>([tests::base_price::<Weth>(1, 55)].into_iter().map(Ok))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(sent, vec!["recv2"]);
    }

    #[test]
    fn alarms_above_all() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let alarms = MarketAlarms::new(&storage as &dyn Storage);

        let sent: Vec<_> = alarms
            .notify_alarms_iter::<_, Base>([tests::base_price::<Weth>(1, 65)].into_iter().map(Ok))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(sent, vec!["recv1", "recv2"]);
    }

    #[test]
    fn alarms_mixed() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let alarms = MarketAlarms::new(&storage as &dyn Storage);

        let sent: Vec<_> = alarms
            .notify_alarms_iter::<_, Base>(
                [
                    tests::base_price::<Weth>(1, 65),
                    tests::base_price::<Atom>(1, 25),
                ]
                .into_iter()
                .map(Ok),
            )
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(sent, vec!["recv1", "recv2", "recv4"]);
    }

    #[test]
    fn alarms_middle_none() {
        let mut storage = MockStorage::new();

        test_case(&mut storage);

        let alarms = MarketAlarms::new(&storage as &dyn Storage);

        let sent: Vec<_> = alarms
            .notify_alarms_iter::<_, Base>(
                [
                    tests::base_price::<Weth>(1, 55),
                    tests::base_price::<Weth>(1, 35),
                    tests::base_price::<Atom>(1, 32),
                    tests::base_price::<Juno>(1, 29),
                ]
                .into_iter()
                .map(Ok),
            )
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(sent, vec!["recv2", "recv5"]);
    }
}
