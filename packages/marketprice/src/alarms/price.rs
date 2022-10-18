use std::collections::HashSet;

use currency::native::Nls;
use finance::{currency::SymbolOwned, price::dto::PriceDTO};
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Order, StdResult, Storage},
    cw_storage_plus::{Item, Map},
};

use super::{errors::AlarmError, Alarm, ExecuteAlarmMsg};

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
        use sdk::cosmwasm_std::StdError;

        self.hooks
            .may_load(storage, addr)?
            .ok_or_else(|| StdError::generic_err("no hook found for address"))
    }

    pub fn notify(
        &self,
        storage: &mut dyn Storage,
        updated_prices: Vec<PriceDTO>,
        batch: &mut Batch,
    ) -> Result<(), AlarmError> {
        let affected_contracts: Vec<_> = self.get_affected(storage, updated_prices)?;

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

        Ok(())
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
        updated_prices: Vec<PriceDTO>,
    ) -> StdResult<Vec<(Addr, Alarm, PriceDTO)>> {
        let mut affected: Vec<(Addr, Alarm, PriceDTO)> = vec![];
        for price in updated_prices {
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
    use currency::native::Nls;
    use finance::{
        coin::Coin,
        currency::{Currency, SymbolStatic},
        price,
    };
    use sdk::cosmwasm_std::{testing::mock_dependencies, Addr};

    use crate::alarms::{price::PriceHooks, Alarm};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct BTH;
    impl Currency for BTH {
        const TICKER: SymbolStatic = "BTH";
        const BANK_SYMBOL: SymbolStatic = "ibc/bth";
    }
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct ETH;
    impl Currency for ETH {
        const TICKER: SymbolStatic = "ETH";
        const BANK_SYMBOL: SymbolStatic = "ibc/eth";
    }

    #[test]
    fn test_add() {
        let hooks = PriceHooks::new("hooks", "hooks_sequence");
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        let price1 = price::total_of(Coin::<BTH>::new(1000000)).is(Coin::<Nls>::new(456789));
        let price2 = price::total_of(Coin::<ETH>::new(1000000)).is(Coin::<Nls>::new(123456));

        let expected_alarm1 = Alarm::new(price1, None);
        let expected_alarm2 = Alarm::new(price2, None);

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
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
        pub struct OtherCoin;
        impl Currency for OtherCoin {
            const TICKER: SymbolStatic = "OtherCoin";
            const BANK_SYMBOL: SymbolStatic = "ibc/other_coin";
        }

        let hooks = PriceHooks::new("hooks", "hooks_sequence");
        let storage = &mut mock_dependencies().storage;

        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        let price1 = price::total_of(Coin::<BTH>::new(1000000)).is(Coin::<OtherCoin>::new(456789));
        let price2 = price::total_of(Coin::<ETH>::new(1000000)).is(Coin::<OtherCoin>::new(123456));

        let expected_alarm1 = Alarm::new(price1, None);
        let expected_alarm2 = Alarm::new(price2, None);

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
