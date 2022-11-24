use currency::native::Nls;
use finance::{
    coin::{Amount, Coin, CoinDTO},
    currency::SymbolOwned,
    price::{
        self,
        dto::{with_price, WithPrice},
    },
};
use platform::batch::Batch;
use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::{
        Bound, Index, IndexList, IndexedMap, IntKey, Item, Key, MultiIndex, Prefixer, PrimaryKey,
    },
};
use serde::{Deserialize, Serialize};
use swap::SwapGroup;

use crate::SpotPrice;

use super::{errors::AlarmError, Alarm, ExecuteAlarmMsg};

pub type AlarmReplyId = u64;

pub struct PriceAlarms<'m> {
    alarms_below_namespace: &'m str,
    alarms_above_namespace: &'m str,
    index_below_namespace: &'m str,
    index_above_namespace: &'m str,
    id_seq: Item<'m, AlarmReplyId>,
}

#[derive(Clone, Serialize, Deserialize)]
struct AlarmStore(CoinDTO<SwapGroup>);

const NORM_SCALE: u128 = 1_000_000_000;
struct InvNormalizeCmd;

impl WithPrice for InvNormalizeCmd {
    type Output = AlarmStore;
    type Error = finance::error::Error;

    fn exec<C, QuoteC>(
        self,
        price: finance::price::Price<C, QuoteC>,
    ) -> Result<Self::Output, Self::Error>
    where
        C: finance::currency::Currency,
        QuoteC: finance::currency::Currency,
    {
        Ok(AlarmStore(
            price::total(Coin::new(NORM_SCALE), price.inv()).into(),
        ))
    }
}

impl AlarmStore {
    fn inv_normalize(price: &SpotPrice) -> Result<Self, AlarmError> {
        let alarm = with_price::execute(price, InvNormalizeCmd)?;
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
    pub fn add_or_update(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        alarm: Alarm,
    ) -> Result<(), AlarmError> {
        self.alarms_below().save(
            storage,
            addr.to_owned(),
            &AlarmStore::inv_normalize(&alarm.below)?,
        )?;

        if let Some(alarm) = alarm.above {
            self.alarms_above().save(
                storage,
                addr.to_owned(),
                &AlarmStore::inv_normalize(&alarm)?,
            )?;
        }

        Ok(())
    }

    pub fn remove(&self, storage: &mut dyn Storage, addr: Addr) -> Result<(), AlarmError> {
        self.alarms_below().remove(storage, addr.clone())?;
        self.alarms_above().remove(storage, addr)?;
        Ok(())
    }

    pub fn notify(
        &self,
        storage: &mut dyn Storage,
        updated_prices: Vec<SpotPrice>,
        batch: &mut Batch,
    ) -> Result<(), AlarmError> {
        let mut next_id = self.id_seq.may_load(storage)?.unwrap_or(0);

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

            *next_id += 1;
            Ok(())
        }

        let alarms_below = self.alarms_below();
        let alarms_above = self.alarms_above();

        for price in updated_prices {
            alarms_below
                .idx
                .0
                .sub_prefix(price.base().ticker().into())
                .range(
                    storage,
                    None,
                    Some(Bound::exclusive((
                        AlarmStore::inv_normalize(&price)?.0.amount(),
                        Addr::unchecked(""),
                    ))),
                    Order::Ascending,
                )
                .try_for_each(|alarm| proc(batch, alarm?.0, &mut next_id))?;

            alarms_above
                .idx
                .0
                .sub_prefix(price.base().ticker().into())
                .range(
                    storage,
                    Some(Bound::exclusive((
                        AlarmStore::inv_normalize(&price)?.0.amount(),
                        Addr::unchecked(""),
                    ))),
                    None,
                    Order::Ascending,
                )
                .try_for_each(|addr| proc(batch, addr?.0, &mut next_id))?;
        }

        self.id_seq.save(storage, &next_id)?;

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use currency::{
        lease::{Atom, Weth},
        lpn::Usdc,
    };
    use finance::{coin::Coin, price};
    use sdk::cosmwasm_std::{testing::mock_dependencies, Addr, CosmosMsg, Response, WasmMsg};

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
            .add_or_update(
                storage,
                &addr1,
                Alarm::new(
                    price::total_of(Coin::<Atom>::new(1)).is(Coin::<Usdc>::new(20)),
                    None,
                ),
            )
            .unwrap();
        alarms
            .add_or_update(
                storage,
                &addr2,
                Alarm::new(
                    price::total_of(Coin::<Atom>::new(1)).is(Coin::<Usdc>::new(5)),
                    Some(price::total_of(Coin::<Atom>::new(1)).is(Coin::<Usdc>::new(10))),
                ),
            )
            .unwrap();
        alarms
            .add_or_update(
                storage,
                &addr3,
                Alarm::new(
                    price::total_of(Coin::<Atom>::new(1)).is(Coin::<Usdc>::new(25)),
                    None,
                ),
            )
            .unwrap();

        alarms.remove(storage, addr1).unwrap();
        alarms.remove(storage, addr2).unwrap();

        let mut batch = Batch::default();

        alarms
            .notify(
                storage,
                vec![price::total_of(Coin::<Atom>::new(1))
                    .is(Coin::<Usdc>::new(15))
                    .into()],
                &mut batch,
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

        alarms
            .add_or_update(
                storage,
                &addr1,
                Alarm::new(
                    price::total_of(Coin::<Atom>::new(1)).is(Coin::<Usdc>::new(10)),
                    None,
                ),
            )
            .unwrap();
        alarms
            .add_or_update(
                storage,
                &addr2,
                Alarm::new(
                    price::total_of(Coin::<Atom>::new(1)).is(Coin::<Usdc>::new(20)),
                    None,
                ),
            )
            .unwrap();
        alarms
            .add_or_update(
                storage,
                &addr3,
                Alarm::new(
                    price::total_of(Coin::<Weth>::new(1)).is(Coin::<Usdc>::new(10)),
                    None,
                ),
            )
            .unwrap();
        alarms
            .add_or_update(
                storage,
                &addr4,
                Alarm::new(
                    price::total_of(Coin::<Weth>::new(1)).is(Coin::<Usdc>::new(20)),
                    Some(price::total_of(Coin::<Weth>::new(1)).is(Coin::<Usdc>::new(25))),
                ),
            )
            .unwrap();
        alarms
            .add_or_update(
                storage,
                &addr5,
                Alarm::new(
                    price::total_of(Coin::<Weth>::new(1)).is(Coin::<Usdc>::new(30)),
                    Some(price::total_of(Coin::<Weth>::new(1)).is(Coin::<Usdc>::new(35))),
                ),
            )
            .unwrap();

        let mut batch = Batch::default();

        alarms
            .notify(
                storage,
                vec![
                    price::total_of(Coin::<Atom>::new(1))
                        .is(Coin::<Usdc>::new(15))
                        .into(),
                    price::total_of(Coin::<Weth>::new(1))
                        .is(Coin::<Usdc>::new(26))
                        .into(),
                ],
                &mut batch,
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

        assert_eq!(resp, vec![addr2, addr5, addr4]);
    }
}
