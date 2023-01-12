use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::native::Nls;
use finance::{
    coin::{Amount, Coin, CoinDTO},
    currency::{Currency, SymbolOwned},
    price::{
        self,
        dto::{with_quote, WithQuote},
        Price,
    },
};
use platform::batch::Batch;
use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{
        Bound, Index, IndexList, IndexedMap, IntKey, Item, Key, MultiIndex, Prefixer, PrimaryKey,
    },
};
use swap::SwapGroup;

use crate::SpotPrice;

use super::{errors::AlarmError, ExecuteAlarmMsg};

pub type AlarmReplyId = u64;

pub type AlarmsCount = u32;

pub struct PriceAlarms<'m> {
    alarms_below_namespace: &'m str,
    alarms_above_namespace: &'m str,
    index_below_namespace: &'m str,
    index_above_namespace: &'m str,
    id_seq: Item<'m, AlarmReplyId>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct AlarmStore(CoinDTO<SwapGroup>);

const NORM_SCALE: u128 = 1_000_000_000;
struct InvNormalizeCmd<QuoteC>(PhantomData<QuoteC>);

impl<QuoteC> WithQuote<QuoteC> for InvNormalizeCmd<QuoteC>
where
    QuoteC: Currency,
{
    type Output = AlarmStore;
    type Error = finance::error::Error;

    fn exec<BaseC>(self, price: Price<BaseC, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        BaseC: Currency,
    {
        Ok(AlarmStore(
            price::total(Coin::new(NORM_SCALE), price.inv()).into(),
        ))
    }
}

impl AlarmStore {
    fn inv_normalize<BaseC>(price: &SpotPrice) -> Result<Self, AlarmError>
    where
        BaseC: Currency,
    {
        let alarm = with_quote::execute(price, InvNormalizeCmd::<BaseC>(PhantomData))?;
        Ok(alarm)
    }
}

impl<'a> PrimaryKey<'a> for AlarmStore {
    type Prefix = SymbolOwned;
    type Suffix = Amount;
    type SubPrefix = ();
    type SuperSuffix = (SymbolOwned, Amount);

    fn key(&self) -> Vec<sdk::cw_storage_plus::Key> {
        vec![
            Key::Ref(self.0.ticker().as_bytes()),
            Key::Val128(self.0.amount().to_cw_bytes()),
        ]
    }
}

impl<'a> Prefixer<'a> for AlarmStore {
    fn prefix(&self) -> Vec<Key> {
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
        seq_namespace: &'m str,
    ) -> PriceAlarms<'m> {
        PriceAlarms {
            alarms_below_namespace,
            index_below_namespace,
            alarms_above_namespace,
            index_above_namespace,
            id_seq: Item::new(seq_namespace),
        }
    }

    fn alarms_below(&self) -> IndexedMap<Addr, AlarmStore, AlarmsIndexes> {
        let indexes = AlarmsIndexes(MultiIndex::new(
            |_, price| price.to_owned(),
            self.alarms_below_namespace,
            self.index_below_namespace,
        ));
        IndexedMap::new(self.alarms_below_namespace, indexes)
    }

    fn alarms_above(&self) -> IndexedMap<Addr, AlarmStore, AlarmsIndexes> {
        let indexes = AlarmsIndexes(MultiIndex::new(
            |_, price| price.to_owned(),
            self.alarms_above_namespace,
            self.index_above_namespace,
        ));
        IndexedMap::new(self.alarms_above_namespace, indexes)
    }

    // TODO: rename
    pub fn add_alarm_below<BaseC>(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        alarm: &SpotPrice,
    ) -> Result<(), AlarmError>
    where
        BaseC: Currency,
    {
        Ok(self.alarms_below().save(
            storage,
            addr.to_owned(),
            &AlarmStore::inv_normalize::<BaseC>(alarm)?,
        )?)
    }

    pub fn add_alarm_above<BaseC>(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        alarm: &SpotPrice,
    ) -> Result<(), AlarmError>
    where
        BaseC: Currency,
    {
        Ok(self.alarms_above().save(
            storage,
            addr.to_owned(),
            &AlarmStore::inv_normalize::<BaseC>(alarm)?,
        )?)
    }

    pub fn remove(&self, storage: &mut dyn Storage, addr: Addr) -> Result<(), AlarmError> {
        self.alarms_below().remove(storage, addr.clone())?;
        self.alarms_above().remove(storage, addr)?;
        Ok(())
    }

    pub fn query_alarms<BaseC>(
        &self,
        storage: &dyn Storage,
        prices: &[SpotPrice],
    ) -> Result<bool, AlarmError>
    where
        BaseC: Currency,
    {
        let alarms_below = self.alarms_below();
        let alarms_above = self.alarms_above();

        let results = prices.iter().map(|price| -> Result<bool, AlarmError> {
            let inv_normalized_price = AlarmStore::inv_normalize::<BaseC>(price)?.0.amount();
            Ok(alarms_below
                .idx
                .0
                .sub_prefix(price.base().ticker().into())
                .range(
                    storage,
                    None,
                    Some(Bound::exclusive((
                        inv_normalized_price,
                        Addr::unchecked(""),
                    ))),
                    Order::Ascending,
                )
                .next()
                .is_some()
                | alarms_above
                    .idx
                    .0
                    .sub_prefix(price.base().ticker().into())
                    .range(
                        storage,
                        Some(Bound::exclusive((
                            inv_normalized_price,
                            Addr::unchecked(""),
                        ))),
                        None,
                        Order::Ascending,
                    )
                    .next()
                    .is_some())
        });

        for res in results {
            if res? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn notify<BaseC>(
        &self,
        storage: &mut dyn Storage,
        batch: &mut Batch,
        prices: &[SpotPrice],
        max_count: u32,
    ) -> Result<AlarmsCount, AlarmError>
    where
        BaseC: Currency,
    {
        let mut count = max_count.try_into()?;
        let mut next_id = self.id_seq.may_load(storage)?.unwrap_or(0);
        let mut start_id = next_id;

        #[inline]
        fn proc(
            batch: &mut Batch,
            addr: Addr,
            next_id: &mut AlarmReplyId,
        ) -> Result<(), AlarmError> {
            batch
                .schedule_execute_wasm_reply_always::<_, Nls>(
                    &addr,
                    ExecuteAlarmMsg::PriceAlarm(),
                    None,
                    *next_id,
                )
                .map_err(AlarmError::from)?;

            *next_id = next_id.wrapping_add(1);
            Ok(())
        }

        let alarms_below = self.alarms_below();
        let alarms_above = self.alarms_above();

        for price in prices {
            let inv_normalized_price = AlarmStore::inv_normalize::<BaseC>(price)?.0.amount();

            alarms_below
                .idx
                .0
                .sub_prefix(price.base().ticker().into())
                .range(
                    storage,
                    None,
                    Some(Bound::exclusive((
                        inv_normalized_price,
                        Addr::unchecked(""),
                    ))),
                    Order::Ascending,
                )
                .take(count)
                .try_for_each(|alarm| proc(batch, alarm?.0, &mut next_id))?;

            count -= usize::try_from(next_id.wrapping_sub(start_id))?;
            start_id = next_id;

            alarms_above
                .idx
                .0
                .sub_prefix(price.base().ticker().into())
                .range(
                    storage,
                    Some(Bound::exclusive((
                        inv_normalized_price,
                        Addr::unchecked(""),
                    ))),
                    None,
                    Order::Ascending,
                )
                .take(count)
                .try_for_each(|addr| proc(batch, addr?.0, &mut next_id))?;

            count -= usize::try_from(next_id - start_id)?;
            start_id = next_id;
        }

        self.id_seq.save(storage, &next_id)?;

        Ok(max_count - u32::try_from(count)?)
    }
}

#[cfg(test)]
pub mod tests {
    use currency::{
        lease::{Atom, Weth},
        lpn::Usdc,
    };
    use finance::{coin::Coin, price};
    use sdk::cosmwasm_std::{testing::mock_dependencies, Addr, CosmosMsg, Response, WasmMsg};

    use super::*;

    type Base = Usdc;

    #[test]
    #[should_panic]
    fn add_below_wrong_base() {
        let alarms = PriceAlarms::new(
            "alarms_below",
            "index_below",
            "alarms_above",
            "index_above",
            "alarms_sequence",
        );
        let storage = &mut mock_dependencies().storage;

        alarms
            .add_alarm_below::<Base>(
                storage,
                &Addr::unchecked("addr1"),
                &price::total_of(Coin::<Base>::new(1))
                    .is(Coin::<Atom>::new(20))
                    .into(),
            )
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn add_above_wrong_base() {
        let alarms = PriceAlarms::new(
            "alarms_below",
            "index_below",
            "alarms_above",
            "index_above",
            "alarms_sequence",
        );
        let storage = &mut mock_dependencies().storage;

        alarms
            .add_alarm_above::<Base>(
                storage,
                &Addr::unchecked("addr1"),
                &price::total_of(Coin::<Base>::new(1))
                    .is(Coin::<Atom>::new(20))
                    .into(),
            )
            .unwrap();
    }

    #[test]
    fn test_add_remove() {
        let alarms = PriceAlarms::new(
            "alarms_below",
            "index_below",
            "alarms_above",
            "index_above",
            "alarms_sequence",
        );
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr1,
                &price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(20))
                    .into(),
            )
            .unwrap();

        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr2,
                &price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(5))
                    .into(),
            )
            .unwrap();
        alarms
            .add_alarm_above::<Base>(
                storage,
                &addr2,
                &price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(10))
                    .into(),
            )
            .unwrap();
        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr3,
                &price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(20))
                    .into(),
            )
            .unwrap();

        alarms.remove(storage, addr1).unwrap();
        alarms.remove(storage, addr2).unwrap();

        let mut batch = Batch::default();

        alarms
            .notify::<Base>(
                storage,
                &mut batch,
                &[price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(15))
                    .into()],
                10,
            )
            .unwrap();

        let resp = Response::from(batch);
        let resp: Vec<_> = resp
            .messages
            .into_iter()
            .map(|m| {
                if let CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) = m.msg {
                    Some(contract_addr)
                } else {
                    None
                }
                .unwrap()
            })
            .collect();

        assert_eq!(resp, vec![addr3]);
    }

    #[test]
    #[should_panic]
    fn test_notify_wrong_base() {
        let alarms = PriceAlarms::new(
            "alarms_below",
            "index_below",
            "alarms_above",
            "index_above",
            "alarms_sequence",
        );
        let storage = &mut mock_dependencies().storage;

        let mut batch = Batch::default();

        let _ = alarms.notify::<Base>(
            storage,
            &mut batch,
            &[price::total_of(Coin::<Atom>::new(1))
                .is(Coin::<Weth>::new(15))
                .into()],
            10,
        );
    }

    #[test]
    fn test_notify() {
        let alarms = PriceAlarms::new(
            "alarms_below",
            "index_below",
            "alarms_above",
            "index_above",
            "alarms_sequence",
        );
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");
        let addr4 = Addr::unchecked("addr4");
        let addr5 = Addr::unchecked("addr5");

        let remaining_alarms = alarms
            .query_alarms::<Base>(
                storage,
                &[
                    price::total_of(Coin::<Atom>::new(1))
                        .is(Coin::<Base>::new(15))
                        .into(),
                    price::total_of(Coin::<Weth>::new(1))
                        .is(Coin::<Base>::new(26))
                        .into(),
                ],
            )
            .unwrap();

        assert!(!remaining_alarms);

        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr1,
                &price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(10))
                    .into(),
            )
            .unwrap();
        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr2,
                &price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(20))
                    .into(),
            )
            .unwrap();
        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr3,
                &price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(30))
                    .into(),
            )
            .unwrap();
        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr4,
                &price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(20))
                    .into(),
            )
            .unwrap();
        alarms
            .add_alarm_above::<Base>(
                storage,
                &addr4,
                &price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(25))
                    .into(),
            )
            .unwrap();
        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr5,
                &price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(20))
                    .into(),
            )
            .unwrap();
        alarms
            .add_alarm_above::<Base>(
                storage,
                &addr5,
                &price::total_of(Coin::<Weth>::new(1))
                    .is(Coin::<Base>::new(35))
                    .into(),
            )
            .unwrap();

        let remaining_alarms = alarms
            .query_alarms::<Base>(
                storage,
                &[
                    price::total_of(Coin::<Atom>::new(1))
                        .is(Coin::<Base>::new(15))
                        .into(),
                    price::total_of(Coin::<Weth>::new(1))
                        .is(Coin::<Base>::new(26))
                        .into(),
                ],
            )
            .unwrap();

        assert!(remaining_alarms);

        let mut batch = Batch::default();

        let sent = alarms
            .notify::<Base>(
                storage,
                &mut batch,
                &[
                    price::total_of(Coin::<Atom>::new(1))
                        .is(Coin::<Base>::new(15))
                        .into(),
                    price::total_of(Coin::<Weth>::new(1))
                        .is(Coin::<Base>::new(26))
                        .into(),
                ],
                10,
            )
            .unwrap();

        let resp = Response::from(batch);
        let resp: Vec<_> = resp
            .messages
            .into_iter()
            .map(|m| {
                if let CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) = m.msg {
                    Some(contract_addr)
                } else {
                    None
                }
                .unwrap()
            })
            .collect();

        assert_eq!(resp, vec![addr2.clone(), addr3.clone(), addr4]);
        assert_eq!(sent, 3);

        let mut batch = Batch::default();

        // check limited max_count
        let sent = alarms
            .notify::<Base>(
                storage,
                &mut batch,
                &[
                    price::total_of(Coin::<Atom>::new(1))
                        .is(Coin::<Base>::new(15))
                        .into(),
                    price::total_of(Coin::<Weth>::new(1))
                        .is(Coin::<Base>::new(26))
                        .into(),
                ],
                2,
            )
            .unwrap();

        let resp = Response::from(batch);
        let resp: Vec<_> = resp
            .messages
            .into_iter()
            .map(|m| {
                if let CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) = m.msg {
                    Some(contract_addr)
                } else {
                    None
                }
                .unwrap()
            })
            .collect();

        assert_eq!(resp, vec![addr2, addr3]);
        assert_eq!(sent, 2);
    }

    #[test]
    #[should_panic]
    fn test_query_wrong_base() {
        let alarms = PriceAlarms::new(
            "alarms_below",
            "index_below",
            "alarms_above",
            "index_above",
            "alarms_sequence",
        );
        let storage = &mut mock_dependencies().storage;

        let _ = alarms.query_alarms::<Base>(
            storage,
            &[price::total_of(Coin::<Atom>::new(1))
                .is(Coin::<Weth>::new(15))
                .into()],
        );
    }

    #[test]
    fn test_id_overflow() {
        let storage = &mut mock_dependencies().storage;
        let alarms = PriceAlarms::new(
            "alarms_below",
            "index_below",
            "alarms_above",
            "index_above",
            "alarms_sequence",
        );

        let id_item: Item<AlarmReplyId> = Item::new("alarms_sequence");
        id_item.save(storage, &AlarmReplyId::MAX).unwrap();

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");

        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr1,
                &price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(20))
                    .into(),
            )
            .unwrap();

        alarms
            .add_alarm_below::<Base>(
                storage,
                &addr2,
                &price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(30))
                    .into(),
            )
            .unwrap();

        let mut batch = Batch::default();
        let sent = alarms
            .notify::<Base>(
                storage,
                &mut batch,
                &[price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Base>::new(10))
                    .into()],
                10,
            )
            .unwrap();

        assert_eq!(sent, 2);

        let resp = Response::from(batch);
        let resp: Vec<_> = resp.messages.into_iter().map(|m| m.id).collect();

        assert_eq!(resp, vec![AlarmReplyId::MAX, 0]);
    }
}
