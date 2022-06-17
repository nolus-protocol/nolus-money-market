use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Order, Response, StdResult, Storage, SubMsg};
use cw_storage_plus::Map;

use super::{errors::HooksError, msg::ExecuteHookMsg, SimpleRule};

pub type Rules = Vec<SimpleRule>;

pub struct PriceHooks<'m>(Map<'m, Addr, Rules>);

impl<'m> PriceHooks<'m> {
    pub const fn new(namespace: &'m str) -> PriceHooks {
        PriceHooks(Map::new(namespace))
    }

    pub fn try_add(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        mut rules: Vec<SimpleRule>,
    ) -> Result<Response, HooksError> {
        let update_rules = |old: Option<Vec<SimpleRule>>| -> StdResult<Vec<SimpleRule>> {
            match old {
                Some(mut h) => {
                    h.append(&mut rules);
                    Ok(h)
                }
                None => Ok(rules),
            }
        };

        self.0.update(storage, addr.to_owned(), update_rules)?;
        Ok(Response::new().add_attribute("method", "try_add"))
    }

    pub fn try_remove(
        _deps: DepsMut,
        _addr: Addr,
        _rule: SimpleRule,
    ) -> Result<Response, HooksError> {
        Ok(Response::new().add_attribute("method", "try_remove"))
    }

    pub fn try_remove_all(_deps: DepsMut, _addr: Addr) -> Result<Response, HooksError> {
        Ok(Response::new().add_attribute("method", "try_remove_all"))
    }

    pub fn check_rules(&self, storage: &mut dyn Storage) -> StdResult<Vec<SubMsg>> {
        let notifications: Vec<_> = self
            .0
            .prefix(())
            .range(storage, None, None, Order::Ascending)
            .map(|item| Self::evaluate(item.unwrap()).unwrap())
            .collect();

        let size = notifications.iter().fold(0, |a, b| a + b.len());
        Ok(notifications
            .into_iter()
            .fold(Vec::with_capacity(size), |mut acc, v| {
                acc.extend(v);
                acc
            }))
    }

    fn evaluate((addr, rules): (Addr, Vec<SimpleRule>)) -> StdResult<Vec<SubMsg>> {
        let mut submsgs = vec![];
        for rule in rules {
            let msg = ExecuteHookMsg::Notify(SimpleRule {});
            let wasm_msg = cosmwasm_std::wasm_execute(addr.to_string(), &msg, vec![])?;
            submsgs.push(SubMsg::reply_always(CosmosMsg::Wasm(wasm_msg), 1));
        }
        Ok(submsgs)
    }
}
