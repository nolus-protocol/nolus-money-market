use std::iter::Chain;

use serde::{Deserialize, Serialize};

use finance::{
    coin::{Amount, Coin, CoinDTO},
    currency::{Currency, SymbolOwned},
    price::{self, Price},
};
use sdk::{
    cosmwasm_std::{Addr, Order, StdError, Storage},
    cw_storage_plus::{
        Bound, Index, IndexList, IndexedMap, IntKey, Key, MultiIndex, Prefixer, PrimaryKey,
    },
};
use swap::SwapGroup;

pub mod errors;
use errors::AlarmError;

pub type AlarmsCount = u32;

pub struct PriceAlarms<'m> {
    alarms_below_namespace: &'m str,
    alarms_above_namespace: &'m str,
    index_below_namespace: &'m str,
    index_above_namespace: &'m str,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct AlarmStore(CoinDTO<SwapGroup>);

const NORM_SCALE: u128 = 10u128.pow(18);

type BoxedIter<'a> = Box<dyn Iterator<Item = Result<(Addr, AlarmStore), StdError>> + 'a>;

pub struct AlarmsIterator<'a>(Chain<BoxedIter<'a>, BoxedIter<'a>>);

impl<'a> Iterator for AlarmsIterator<'a> {
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

struct AlarmsIndexes<'a>(MultiIndex<'a, AlarmStore, AlarmStore, Addr>);

impl<'a> IndexList<AlarmStore> for AlarmsIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AlarmStore>> + '_> {
        let v: Vec<&dyn Index<AlarmStore>> = vec![&self.0];
        Box::new(v.into_iter())
    }
}

impl<'m> PriceAlarms<'m> {
    pub const fn new(
        alarms_below_namespace: &'m str,
        index_below_namespace: &'m str,
        alarms_above_namespace: &'m str,
        index_above_namespace: &'m str,
    ) -> PriceAlarms<'m> {
        PriceAlarms {
            alarms_below_namespace,
            index_below_namespace,
            alarms_above_namespace,
            index_above_namespace,
        }
    }

    pub fn add_alarm_below<C, BaseC>(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        alarm: Price<C, BaseC>,
    ) -> Result<(), AlarmError>
    where
        C: Currency,
        BaseC: Currency,
    {
        Ok(self
            .alarms_below()
            .save(storage, addr.to_owned(), &AlarmStore::new(&alarm))?)
    }

    pub fn add_alarm_above_or_equal<C, BaseC>(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        alarm: Price<C, BaseC>,
    ) -> Result<(), AlarmError>
    where
        C: Currency,
        BaseC: Currency,
    {
        Ok(self
            .alarms_above_or_equal()
            .save(storage, addr.to_owned(), &AlarmStore::new(&alarm))?)
    }

    pub fn remove(&self, storage: &mut dyn Storage, addr: Addr) -> Result<(), AlarmError> {
        self.alarms_below().remove(storage, addr.clone())?;
        self.alarms_above_or_equal().remove(storage, addr)?;
        Ok(())
    }

    pub fn alarms<'a, C, BaseC>(
        &self,
        storage: &'a dyn Storage,
        price: Price<C, BaseC>,
    ) -> AlarmsIterator<'a>
    where
        C: Currency,
        BaseC: Currency,
    {
        let norm_price = AlarmStore::new(&price);

        AlarmsIterator(
            self.iter_below::<C>(storage, &norm_price)
                .chain(self.iter_above_or_equal::<C>(storage, &norm_price)),
        )
    }

    fn alarms_below(&self) -> IndexedMap<'_, Addr, AlarmStore, AlarmsIndexes<'_>> {
        let indexes = AlarmsIndexes(MultiIndex::new(
            |_, price| price.to_owned(),
            self.alarms_below_namespace,
            self.index_below_namespace,
        ));
        IndexedMap::new(self.alarms_below_namespace, indexes)
    }

    fn alarms_above_or_equal(&self) -> IndexedMap<'_, Addr, AlarmStore, AlarmsIndexes<'_>> {
        let indexes = AlarmsIndexes(MultiIndex::new(
            |_, price| price.to_owned(),
            self.alarms_above_namespace,
            self.index_above_namespace,
        ));
        IndexedMap::new(self.alarms_above_namespace, indexes)
    }

    fn iter_below<'a, C>(&self, storage: &'a dyn Storage, price: &AlarmStore) -> BoxedIter<'a>
    where
        C: Currency,
    {
        self.alarms_below()
            .idx
            .0
            .sub_prefix(C::TICKER.into())
            .range(
                storage,
                None,
                Some(Bound::exclusive((price.0.amount(), Addr::unchecked("")))),
                Order::Ascending,
            )
    }

    fn iter_above_or_equal<'a, C>(
        &self,
        storage: &'a dyn Storage,
        price: &AlarmStore,
    ) -> BoxedIter<'a>
    where
        C: Currency,
    {
        self.alarms_above_or_equal()
            .idx
            .0
            .sub_prefix(C::TICKER.into())
            .range(
                storage,
                Some(Bound::exclusive((price.0.amount(), Addr::unchecked("")))),
                None,
                Order::Ascending,
            )
    }
}

#[cfg(test)]
pub mod tests {
    use currency::{
        lease::{Atom, Weth},
        lpn::Usdc,
    };
    use finance::{coin::Coin, price};
    use sdk::cosmwasm_std::{testing::mock_dependencies, Addr};

    use super::*;

    type Base = Usdc;

    #[test]
    fn test_below_exclusive() {
        let alarms = alarms();
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");

        let price = price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20));
        alarms.add_alarm_below(storage, &addr1, price).unwrap();

        assert_eq!(None, alarms.alarms(storage, price).next());
    }

    #[test]
    fn test_above_inclusive() {
        let alarms = alarms();
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");

        let price = price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20));
        alarms
            .add_alarm_above_or_equal(storage, &addr1, price)
            .unwrap();

        let mut triggered_alarms = alarms.alarms(storage, price);
        assert_eq!(Some(Ok(addr1)), triggered_alarms.next());
        assert_eq!(None, triggered_alarms.next());
    }

    #[test]
    fn test_equal_alarms() {
        let alarms = alarms();
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");

        let price = price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20));
        alarms.add_alarm_below(storage, &addr1, price).unwrap();
        alarms
            .add_alarm_above_or_equal(storage, &addr1, price)
            .unwrap();

        let mut triggered_alarms = alarms.alarms(storage, price);
        assert_eq!(Some(Ok(addr1)), triggered_alarms.next());
        assert_eq!(None, triggered_alarms.next());
    }

    #[test]
    fn test_add_remove() {
        let alarms = alarms();
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        alarms
            .add_alarm_below(
                storage,
                &addr1,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();

        alarms
            .add_alarm_below(
                storage,
                &addr2,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(5)),
            )
            .unwrap();
        alarms
            .add_alarm_above_or_equal(
                storage,
                &addr2,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(10)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                storage,
                &addr3,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();

        alarms.remove(storage, addr1).unwrap();
        alarms.remove(storage, addr2).unwrap();

        let resp: Vec<_> = alarms
            .alarms(
                storage,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(15)),
            )
            .collect();

        assert_eq!(resp, vec![Ok(addr3)]);
    }

    #[test]
    fn test_alarms_selection() {
        let alarms = alarms();
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");
        let addr5 = Addr::unchecked("addr5");

        alarms
            .add_alarm_below(
                storage,
                &addr1,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(10)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                storage,
                &addr2,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                storage,
                &addr3,
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(30)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                storage,
                &addr4,
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();
        alarms
            .add_alarm_above_or_equal(
                storage,
                &addr4,
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(25)),
            )
            .unwrap();
        alarms
            .add_alarm_below(
                storage,
                &addr5,
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(20)),
            )
            .unwrap();
        alarms
            .add_alarm_above_or_equal(
                storage,
                &addr5,
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(35)),
            )
            .unwrap();

        let resp: Vec<_> = alarms
            .alarms(
                storage,
                price::total_of(Coin::<Atom>::new(1)).is(Coin::<Base>::new(15)),
            )
            .collect();

        assert_eq!(resp, vec![Ok(addr2)]);

        let resp: Vec<_> = alarms
            .alarms(
                storage,
                price::total_of(Coin::<Weth>::new(1)).is(Coin::<Base>::new(26)),
            )
            .collect();

        assert_eq!(resp, vec![Ok(addr3), Ok(addr4)]);
    }

    fn alarms() -> PriceAlarms<'static> {
        PriceAlarms::new("alarms_below", "index_below", "alarms_above", "index_above")
    }
}
