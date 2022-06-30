use std::collections::HashSet;

use cosmwasm_std::{Addr, Order, Response, StdResult, Storage};
use cw_storage_plus::Map;

use super::errors::HooksError;
use crate::feed::{Denom, DenomToPrice};

pub struct PriceHooks<'m>(Map<'m, Addr, DenomToPrice>);

impl<'m> PriceHooks<'m> {
    pub const fn new(namespace: &'m str) -> PriceHooks {
        PriceHooks(Map::new(namespace))
    }

    pub fn add_or_update(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        target: DenomToPrice,
    ) -> Result<Response, HooksError> {
        let update_hook = |_: Option<DenomToPrice>| -> StdResult<DenomToPrice> { Ok(target) };
        self.0.update(storage, addr.to_owned(), update_hook)?;
        Ok(Response::new().add_attribute("method", "add_or_update"))
    }

    pub fn remove(&self, storage: &mut dyn Storage, addr: Addr) -> Result<Response, HooksError> {
        let hook = self.0.key(addr);
        hook.remove(storage);
        Ok(Response::new().add_attribute("method", "remove"))
    }

    pub fn get_hook_denoms(&self, storage: &dyn Storage) -> StdResult<HashSet<Denom>> {
        let hook_denoms: HashSet<Denom> = self
            .0
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
                .0
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
        let hooks = PriceHooks::new("price_hooks");
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
