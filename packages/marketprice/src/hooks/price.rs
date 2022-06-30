use std::collections::HashSet;

use cosmwasm_std::{to_binary, Addr, Order, Response, StdResult, Storage, Timestamp};
use cw_storage_plus::{Item, Map};

use super::{errors::HooksError, HookDispatcher};
use crate::feed::{Denom, DenomToPrice};

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
    hooks: Map<'m, Addr, DenomToPrice>,
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
        target: DenomToPrice,
    ) -> Result<Response, HooksError> {
        let update_hook = |_: Option<DenomToPrice>| -> StdResult<DenomToPrice> { Ok(target) };
        self.hooks.update(storage, addr.to_owned(), update_hook)?;
        Ok(Response::new().add_attribute("method", "add_or_update"))
    }

    pub fn remove(&self, storage: &mut dyn Storage, addr: Addr) -> Result<Response, HooksError> {
        let hook = self.hooks.key(addr);
        hook.remove(storage);
        Ok(Response::new().add_attribute("method", "remove"))
    }
    pub fn notify(
        &self,
        storage: &mut dyn Storage,
        dispatcher: &mut impl HookDispatcher,
        ctime: Timestamp,
        updated_prices: Vec<DenomToPrice>,
    ) -> StdResult<()> {
        let affected_contracts: Vec<_> = self.get_affected(storage, updated_prices)?;

        for (addr, price) in affected_contracts {
            dispatcher.send_to(
                self.id_seq.next(storage)?,
                addr,
                ctime,
                &Some(to_binary(&price)?),
            )?;

            // let (id, alarm) = timestamp?;
            // dispatcher.send_to(id, alarm.addr, ctime, &None)?;
        }

        Ok(())
    }

    pub fn get_hook_denoms(&self, storage: &dyn Storage) -> StdResult<HashSet<Denom>> {
        let hook_denoms: HashSet<Denom> = self
            .hooks
            .prefix(())
            .range(storage, None, None, Order::Ascending)
            .map(|item| match item {
                Ok((_, hook)) => hook.denom,
                Err(_) => todo!(),
            })
            .collect();
        Ok(hook_denoms)
    }

    pub fn get_affected(
        &self,
        storage: &mut dyn Storage,
        updated_prices: Vec<DenomToPrice>,
    ) -> StdResult<Vec<(Addr, DenomToPrice)>> {
        let mut affected: Vec<(Addr, DenomToPrice)> = vec![];
        for updated in updated_prices {
            let mut msgs: Vec<_> = self
                .hooks
                .prefix(())
                .range(storage, None, None, Order::Ascending)
                .filter_map(|item| item.ok())
                .filter(|(_, hook)| {
                    updated.denom.eq(&hook.denom) && updated.price.is_below(&hook.price)
                })
                .map(|(addr, _)| (addr, updated.clone()))
                .collect();

            affected.append(&mut msgs);
        }
        Ok(affected)
    }
}

#[cfg(test)]
pub mod tests {
    use std::str::FromStr;

    use cosmwasm_std::{testing::mock_dependencies, Addr, Decimal};

    use crate::{
        feed::{DenomToPrice, Price},
        hooks::price::PriceHooks,
    };

    #[test]
    fn test_add() {
        let hooks = PriceHooks::new("hooks", "hooks_sequence");
        let storage = &mut mock_dependencies().storage;

        let t1 = DenomToPrice::new(
            "BTH".to_string(),
            Price::new(Decimal::from_str("0.456789").unwrap(), "NLS".to_string()),
        );
        let t2 = DenomToPrice::new(
            "ETH".to_string(),
            Price::new(Decimal::from_str("0.123456").unwrap(), "NLS".to_string()),
        );
        let addr1 = Addr::unchecked("addr1");
        let addr2 = Addr::unchecked("addr2");
        let addr3 = Addr::unchecked("addr3");

        assert!(hooks.add_or_update(storage, &addr1, t1.clone()).is_ok());
        // same timestamp
        assert!(hooks.add_or_update(storage, &addr2, t1).is_ok());
        // different timestamp
        assert!(hooks.add_or_update(storage, &addr3, t2).is_ok());

        let hook_denoms = hooks.get_hook_denoms(storage).unwrap();
        assert_eq!(hook_denoms.len(), 2);

        assert!(hook_denoms.contains("BTH"));
        assert!(hook_denoms.contains("ETH"));
    }
}
