use std::collections::{HashMap, HashSet};

use cosmwasm_std::{Addr, Order, Response, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use finance::currency::{Nls, SymbolOwned};
use platform::batch::Batch;

use super::{errors::AlarmError, Alarm, ExecuteAlarmMsg};
use crate::storage::Price;

pub type HookReplyId = u64;
pub struct HookReplyIdSeq<'a>(Item<'a, HookReplyId>);

impl<'a> HookReplyIdSeq<'a> {
    pub const fn new(namespace: &'a str) -> HookReplyIdSeq {
        HookReplyIdSeq(Item::new(namespace))
    }

    pub fn next(&self, store: &mut dyn Storage) -> StdResult<HookReplyId> {
        let mut next_seq = self.0.load(store).unwrap_or(0);
        next_seq += 1;
        self.0.save(store, &next_seq)?;
        Ok(next_seq)
    }
}

pub struct PriceHooks<'m> {
    hooks: Map<'m, Addr, Alarm>,
    id_seq: HookReplyIdSeq<'m>,
}

impl<'m> PriceHooks<'m> {
    pub const fn new(hooks_namespace: &'m str, seq_namespace: &'m str) -> PriceHooks<'m> {
        PriceHooks {
            hooks: Map::new(hooks_namespace),
            id_seq: HookReplyIdSeq::new(seq_namespace),
        }
    }

    pub fn add_or_update(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        alarm: Alarm,
    ) -> Result<Response, AlarmError> {
        let update_hook = |_: Option<Alarm>| -> StdResult<Alarm> { Ok(alarm) };
        self.hooks.update(storage, addr.to_owned(), update_hook)?;
        Ok(Response::new())
    }

    pub fn remove(&self, storage: &mut dyn Storage, addr: Addr) -> Result<Response, AlarmError> {
        let hook = self.hooks.key(addr);
        hook.remove(storage);
        Ok(Response::new())
    }

    #[cfg(test)]
    pub fn get(&self, storage: &dyn Storage, addr: Addr) -> StdResult<Alarm> {
        use cosmwasm_std::StdError;

        self.hooks
            .may_load(storage, addr)?
            .ok_or_else(|| StdError::generic_err("no hook found for address"))
    }

    pub fn notify(
        &self,
        storage: &mut dyn Storage,
        updated_prices: HashMap<SymbolOwned, Price>,
    ) -> Result<Batch, AlarmError> {
        let affected_contracts: Vec<_> = self.get_affected(storage, updated_prices)?;

        let mut batch = Batch::default();

        for (addr, alarm, _) in affected_contracts {
            let next_id = self.id_seq.next(storage)?;

            batch
                .schedule_execute_wasm_reply_always::<_, Nls>(
                    &addr,
                    ExecuteAlarmMsg::PriceAlarm(alarm),
                    None,
                    next_id,
                )
                .map_err(AlarmError::from)?;
        }

        Ok(batch)
    }

    pub fn get_hook_denoms(&self, storage: &dyn Storage) -> StdResult<HashSet<SymbolOwned>> {
        let hook_denoms: HashSet<SymbolOwned> = self
            .hooks
            .prefix(())
            .range(storage, None, None, Order::Ascending)
            .filter_map(|item| item.ok())
            .map(|(_, hook)| hook.currency)
            .collect();
        Ok(hook_denoms)
    }

    pub fn get_affected(
        &self,
        storage: &mut dyn Storage,
        updated_prices: HashMap<SymbolOwned, Price>,
    ) -> StdResult<Vec<(Addr, Alarm, Price)>> {
        let mut affected: Vec<(Addr, Alarm, Price)> = vec![];
        for price in updated_prices.values() {
            let mut events: Vec<_> = self
                .hooks
                .prefix(())
                .range(storage, None, None, Order::Ascending)
                .filter_map(|item| item.ok())
                .filter(|(_, alarm)| alarm.should_fire(price.clone()))
                .map(|(addr, alarm)| (addr, alarm, price.clone()))
                .collect();

            affected.append(&mut events);
        }
        Ok(affected)
    }
}

#[cfg(test)]
pub mod tests {

    use cosmwasm_std::{testing::mock_dependencies, Addr};

    use crate::{
        alarms::{price::PriceHooks, Alarm},
        storage::Price,
    };

    #[test]
    fn test_add() {
        let hooks = PriceHooks::new("hooks", "hooks_sequence");
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        let price1: Price = Price::new("BTH", 1000000, "NLS", 456789);
        let price2: Price = Price::new("ETH", 1000000, "NLS", 123456);

        let expected_alarm1 = Alarm::new("BTH".to_string(), price1, None);
        let expected_alarm2 = Alarm::new("ETH".to_string(), price2, None);

        assert!(hooks
            .add_or_update(storage, &addr1, expected_alarm1.clone())
            .is_ok());
        assert_eq!(hooks.get(storage, addr1.clone()).unwrap(), expected_alarm1);

        // same price hook
        assert!(hooks
            .add_or_update(storage, &addr2, expected_alarm1.clone())
            .is_ok());
        assert_eq!(hooks.get(storage, addr2.clone()).unwrap(), expected_alarm1);

        // different timestamp
        assert!(hooks
            .add_or_update(storage, &addr3, expected_alarm2.clone())
            .is_ok());

        assert!(hooks
            .add_or_update(storage, &addr1, expected_alarm2.clone())
            .is_ok());

        let hook_denoms = hooks.get_hook_denoms(storage).unwrap();
        assert_eq!(hook_denoms.len(), 2);

        assert_eq!(hooks.get(storage, addr1).unwrap(), expected_alarm2);

        assert!(hook_denoms.contains("BTH"));
        assert!(hook_denoms.contains("ETH"));
    }

    #[test]
    fn test_remove() {
        let hooks = PriceHooks::new("hooks", "hooks_sequence");
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        let price1 = Price::new("some_coin", 1000000, "another_coin", 456789);
        let price2 = Price::new("some_coin", 1000000, "another_coin", 123456);

        let expected_alarm1 = Alarm::new("some_coin".to_string(), price1, None);
        let expected_alarm2 = Alarm::new("some_coin".to_string(), price2, None);

        assert!(hooks
            .add_or_update(storage, &addr1, expected_alarm1.clone())
            .is_ok());
        assert!(hooks
            .add_or_update(storage, &addr2, expected_alarm1.clone())
            .is_ok());
        assert!(hooks
            .add_or_update(storage, &addr3, expected_alarm2)
            .is_ok());

        assert_eq!(hooks.get(storage, addr2.clone()).unwrap(), expected_alarm1);
        hooks.remove(storage, addr2.clone()).unwrap();
        assert_eq!(
            hooks.get(storage, addr2.clone()).unwrap_err().to_string(),
            "Generic error: no hook found for address"
        );
        hooks.remove(storage, addr2).unwrap();
    }
}
