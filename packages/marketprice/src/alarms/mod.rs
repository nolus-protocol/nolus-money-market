use std::{
    iter,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};

use finance::{
    coin::{Amount, Coin, CoinDTO},
    currency::{Currency, SymbolOwned},
    price::{self, Price},
};
use sdk::{
    cosmwasm_std::{Addr, Order, StdError, Storage},
    cw_storage_plus::{
        Bound, Deque, Index, IndexList, IndexedMap as CwIndexedMap, IntKey, Key, MultiIndex,
        Prefixer, PrimaryKey,
    },
};
use swap::SwapGroup;

use self::errors::AlarmError;

pub mod errors;

pub type AlarmsCount = u32;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct AlarmStore(CoinDTO<SwapGroup>);

const NORM_SCALE: u128 = 10u128.pow(18);

type BoxedIter<'storage> =
    Box<dyn Iterator<Item = Result<(Addr, AlarmStore), StdError>> + 'storage>;

pub struct AlarmsIterator<'alarms>(iter::Chain<BoxedIter<'alarms>, BoxedIter<'alarms>>);

impl<'alarms> Iterator for AlarmsIterator<'alarms> {
    type Item = Result<Addr, AlarmError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|res| res.map(|pair| pair.0).map_err(Into::into))
    }
}

impl AlarmStore {
    fn new<C, BaseC>(price: &Price<C, BaseC>) -> Self
    where
        C: Currency,
        BaseC: Currency,
    {
        AlarmStore(price::total(Coin::new(NORM_SCALE), price.inv()).into())
    }
}

impl<'a> PrimaryKey<'a> for AlarmStore {
    type Prefix = SymbolOwned;
    type Suffix = Amount;
    type SubPrefix = ();
    type SuperSuffix = (SymbolOwned, Amount);

    fn key(&self) -> Vec<Key<'_>> {
        vec![
            Key::Ref(self.0.ticker().as_bytes()),
            Key::Val128(self.0.amount().to_cw_bytes()),
        ]
    }
}

impl<'a> Prefixer<'a> for AlarmStore {
    fn prefix(&self) -> Vec<Key<'_>> {
        self.key()
    }
}

struct AlarmsIndexes(MultiIndex<'static, AlarmStore, AlarmStore, Addr>);

impl IndexList<AlarmStore> for AlarmsIndexes {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<AlarmStore>> + '_> {
        Box::new(vec![&self.0 as &_].into_iter())
    }
}

type IndexedMap = CwIndexedMap<'static, Addr, AlarmStore, AlarmsIndexes>;

fn alarms_index(alarms_namespace: &'static str, index_namespace: &'static str) -> IndexedMap {
    let indexes = AlarmsIndexes(MultiIndex::new(
        |_, price| price.to_owned(),
        alarms_namespace,
        index_namespace,
    ));

    IndexedMap::new(alarms_namespace, indexes)
}

pub struct PriceAlarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    storage: S,
    alarms_below: IndexedMap,
    alarms_above_or_equal: IndexedMap,
    in_delivery: Deque<'static, AlarmWithSubscriber>,
}

impl<'storage, S> PriceAlarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    pub fn new(
        storage: S,
        alarms_below_namespace: &'static str,
        index_below_namespace: &'static str,
        alarms_above_namespace: &'static str,
        index_above_namespace: &'static str,
        in_delivery_namespace: &'static str,
    ) -> Self {
        Self {
            storage,
            alarms_below: alarms_index(alarms_below_namespace, index_below_namespace),
            alarms_above_or_equal: alarms_index(alarms_above_namespace, index_above_namespace),
            in_delivery: Deque::new(in_delivery_namespace),
        }
    }

    pub fn alarms<C, BaseC>(&self, price: Price<C, BaseC>) -> AlarmsIterator<'_>
    where
        C: Currency,
        BaseC: Currency,
    {
        let norm_price = AlarmStore::new(&price);

        AlarmsIterator(
            self.iter_below::<C>(&norm_price)
                .chain(self.iter_above_or_equal::<C>(&norm_price)),
        )
    }

    pub fn ensure_no_in_delivery(&self) -> Result<(), AlarmError> {
        if self.in_delivery.is_empty(&*self.storage)? {
            Ok(())
        } else {
            Err(AlarmError::NonEmptyAlarmsInDeliveryQueue(String::from(
                "Assertion requested",
            )))
        }
    }

    fn iter_below<C>(&self, price: &AlarmStore) -> BoxedIter<'_>
    where
        C: Currency,
    {
        self.alarms_below.idx.0.sub_prefix(C::TICKER.into()).range(
            &*self.storage,
            None,
            Some(Bound::exclusive((price.0.amount(), Addr::unchecked("")))),
            Order::Ascending,
        )
    }

    fn iter_above_or_equal<C>(&self, price: &AlarmStore) -> BoxedIter<'_>
    where
        C: Currency,
    {
        self.alarms_above_or_equal
            .idx
            .0
            .sub_prefix(C::TICKER.into())
            .range(
                &*self.storage,
                Some(Bound::exclusive((price.0.amount(), Addr::unchecked("")))),
                None,
                Order::Ascending,
            )
    }
}

impl<'storage, S> PriceAlarms<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    pub fn add_alarm_below<C, BaseC>(
        &mut self,
        subscriber: Addr,
        alarm: Price<C, BaseC>,
    ) -> Result<(), AlarmError>
    where
        C: Currency,
        BaseC: Currency,
    {
        Self::add_alarm_internal(
            &self.alarms_below,
            &mut *self.storage,
            subscriber,
            &AlarmStore::new(&alarm),
        )
    }

    pub fn add_alarm_above_or_equal<C, BaseC>(
        &mut self,
        subscriber: Addr,
        alarm: Price<C, BaseC>,
    ) -> Result<(), AlarmError>
    where
        C: Currency,
        BaseC: Currency,
    {
        Self::add_alarm_internal(
            &self.alarms_above_or_equal,
            &mut *self.storage,
            subscriber,
            &AlarmStore::new(&alarm),
        )
    }

    pub fn remove(&mut self, addr: Addr) -> Result<(), AlarmError> {
        self.alarms_below
            .remove(&mut *self.storage, addr.clone())
            .and_then(|()| self.alarms_above_or_equal.remove(&mut *self.storage, addr))
            .map_err(Into::into)
    }

    pub fn out_for_delivery(&mut self, subscriber: Addr) -> Result<(), AlarmError> {
        let below: AlarmStore = self.alarms_below.load(&*self.storage, subscriber.clone())?;

        self.alarms_below
            .replace(&mut *self.storage, subscriber.clone(), None, Some(&below))?;

        let above: Option<AlarmStore> = self
            .alarms_above_or_equal
            .may_load(&*self.storage, subscriber.clone())?;

        if let Some(above) = &above {
            self.alarms_below
                .replace(&mut *self.storage, subscriber.clone(), None, Some(above))?;
        }

        self.in_delivery
            .push_back(
                &mut *self.storage,
                &AlarmWithSubscriber {
                    subscriber,
                    below,
                    above,
                },
            )
            .map_err(Into::into)
    }

    pub fn last_delivered(&mut self) -> Result<(), AlarmError> {
        self.in_delivery
            .pop_front(&mut *self.storage)
            .map_err(Into::into)
            .and_then(|maybe_alarm: Option<AlarmWithSubscriber>| {
                if maybe_alarm.is_some() {
                    Ok(())
                } else {
                    Err(AlarmError::EmptyAlarmsInDeliveryQueue(String::from(
                        "Received success reply status",
                    )))
                }
            })
    }

    pub fn last_failed(&mut self) -> Result<(), AlarmError> {
        self.in_delivery
            .pop_front(&mut *self.storage)
            .map_err(Into::into)
            .and_then(|maybe_alarm: Option<AlarmWithSubscriber>| {
                maybe_alarm.ok_or(AlarmError::EmptyAlarmsInDeliveryQueue(String::from(
                    "Received failure reply status",
                )))
            })
            .and_then(|alarm: AlarmWithSubscriber| {
                Self::add_alarm_internal(
                    &self.alarms_below,
                    &mut *self.storage,
                    alarm.subscriber.clone(),
                    &alarm.below,
                )
                .and_then(|()| {
                    if let Some(above) = alarm.above {
                        Self::add_alarm_internal(
                            &self.alarms_above_or_equal,
                            &mut *self.storage,
                            alarm.subscriber.clone(),
                            &above,
                        )
                    } else {
                        Ok(())
                    }
                })
            })
    }

    fn add_alarm_internal(
        alarms: &IndexedMap,
        storage: &mut dyn Storage,
        subscriber: Addr,
        alarm: &AlarmStore,
    ) -> Result<(), AlarmError> {
        alarms.save(storage, subscriber, alarm).map_err(Into::into)
    }
}

#[derive(Serialize, Deserialize)]
struct AlarmWithSubscriber {
    subscriber: Addr,
    below: AlarmStore,
    above: Option<AlarmStore>,
}

#[cfg(test)]
pub mod tests {
    use currency::{
        lease::{Atom, Weth},
        lpn::Usdc,
    };
    use finance::{coin::Coin, price};
    use sdk::cosmwasm_std::{testing::MockStorage, Addr};

    use super::*;

    type Base = Usdc;

    #[test]
    fn test_below_exclusive() {
        let mut storage = MockStorage::new();
        let mut alarms = alarms(&mut storage);

        let addr1 = Addr::unchecked("addr1");

        let price = price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20));
        alarms.add_alarm_below(addr1, price).unwrap();

        assert_eq!(None, alarms.alarms(price).next());
    }

    #[test]
    fn test_above_inclusive() {
        let mut storage = MockStorage::new();
        let mut alarms = alarms(&mut storage);

        let addr1 = Addr::unchecked("addr1");

        let price = price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20));
        alarms
            .add_alarm_above_or_equal(addr1.clone(), price)
            .unwrap();

        let mut triggered_alarms = alarms.alarms(price);
        assert_eq!(Some(Ok(addr1)), triggered_alarms.next());
        assert_eq!(None, triggered_alarms.next());
    }

    #[test]
    fn test_equal_alarms() {
        let mut storage = MockStorage::new();
        let mut alarms = alarms(&mut storage);

        let addr1 = Addr::unchecked("addr1");

        let price = price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20));
        alarms.add_alarm_below(addr1.clone(), price).unwrap();
        alarms
            .add_alarm_above_or_equal(addr1.clone(), price)
            .unwrap();

        let mut triggered_alarms = alarms.alarms(price);
        assert_eq!(Some(Ok(addr1)), triggered_alarms.next());
        assert_eq!(None, triggered_alarms.next());
    }

    #[test]
    fn test_add_remove() {
        let mut storage = MockStorage::new();
        let mut alarms = alarms(&mut storage);

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        alarms
            .add_alarm_below(
                addr1.clone(),
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();

        alarms
            .add_alarm_below(
                addr2.clone(),
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(5)),
            )
            .unwrap();
        alarms
            .add_alarm_above_or_equal(
                addr2.clone(),
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(10)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                addr3.clone(),
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();

        alarms.remove(addr1).unwrap();
        alarms.remove(addr2).unwrap();

        let resp: Vec<_> = alarms
            .alarms(price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(15)))
            .collect();

        assert_eq!(resp, vec![Ok(addr3)]);
    }

    #[test]
    fn test_alarms_selection() {
        let mut storage = MockStorage::new();
        let mut alarms = alarms(&mut storage);

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");
        let addr5 = Addr::unchecked("addr5");

        alarms
            .add_alarm_below(
                addr1,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(10)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                addr2.clone(),
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                addr3.clone(),
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(30)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                addr4.clone(),
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();
        alarms
            .add_alarm_above_or_equal(
                addr4.clone(),
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(25)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                addr5.clone(),
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();
        alarms
            .add_alarm_above_or_equal(
                addr5,
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(35)),
            )
            .unwrap();

        let resp: Vec<_> = alarms
            .alarms(price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(15)))
            .collect();

        assert_eq!(resp, vec![Ok(addr2)]);

        let resp: Vec<_> = alarms
            .alarms(price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(26)))
            .collect();

        assert_eq!(resp, vec![Ok(addr3), Ok(addr4)]);
    }

    fn alarms<'storage, 'storage_ref>(
        storage: &'storage_ref mut (dyn Storage + 'storage),
    ) -> PriceAlarms<'storage, &'storage_ref mut (dyn Storage + 'storage)> {
        PriceAlarms::new(
            storage,
            "alarms_below",
            "index_below",
            "alarms_above",
            "index_above",
            "in_delivery",
        )
    }
}
