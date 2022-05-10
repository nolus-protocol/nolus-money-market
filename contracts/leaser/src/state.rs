use crate::{
    config::Config,
    msg::{InstantiateMsg, UpdateConfigMsg},
    ContractError,
};
use cosmwasm_std::{Addr, DepsMut, StdResult, Storage};
use cw_storage_plus::{Item, Map};

pub const LS: LeaserState = LeaserState::new(
    "config",
    "loans",
    "instantiate_reply_ids",
    "pending_instance_creations",
);

pub type InstantiateReplyId = u64;
pub struct InstantiateReplyIdSeq<'a>(Item<'a, InstantiateReplyId>);

impl<'a> InstantiateReplyIdSeq<'a> {
    pub const fn new(namespace: &'a str) -> InstantiateReplyIdSeq {
        InstantiateReplyIdSeq(Item::new(namespace))
    }

    pub fn next(&self, store: &mut dyn Storage) -> Result<InstantiateReplyId, ContractError> {
        let mut next_seq = self.0.load(store).unwrap_or(0);
        next_seq += 1;
        self.0.save(store, &next_seq)?;
        Ok(next_seq)
    }
}

pub struct LeaserState<'a> {
    config: Item<'a, Config>,
    loans: Map<'a, Addr, Addr>,
    instantiate_reply_ids: InstantiateReplyIdSeq<'a>,
    pending_instance_creations: Map<'a, InstantiateReplyId, Addr>,
}

impl<'a> LeaserState<'a> {
    pub const fn new(
        config_ns: &'a str,
        loans_ns: &'a str,
        reply_ids_ns: &'a str,
        pending_ns: &'a str,
    ) -> Self {
        Self {
            config: Item::new(config_ns),
            loans: Map::new(loans_ns),
            instantiate_reply_ids: InstantiateReplyIdSeq::new(reply_ids_ns),
            pending_instance_creations: Map::new(pending_ns),
        }
    }

    pub fn init(
        &self,
        deps: DepsMut,
        msg: InstantiateMsg,
        sender: Addr,
    ) -> Result<(), ContractError> {
        let config = Config::new(sender, msg)?;
        self.config.save(deps.storage, &config)?;

        Ok(())
    }

    pub fn get_config(&self, storage: &dyn Storage) -> StdResult<Config> {
        self.config.load(storage)
    }

    pub fn update_config(
        &self,
        storage: &mut dyn Storage,
        msg: UpdateConfigMsg,
    ) -> Result<(), ContractError> {
        self.config.load(storage)?;

        self.config
            .update(storage, |mut c| -> Result<Config, ContractError> {
                c.update_from(msg)?;
                Ok(c)
            })?;
        Ok(())
    }

    pub fn next(
        &self,
        storage: &mut dyn Storage,
        sender: Addr,
    ) -> Result<InstantiateReplyId, ContractError> {
        let instance_reply_id = self.instantiate_reply_ids.next(storage)?;

        self.pending_instance_creations
            .save(storage, instance_reply_id, &sender)?;

        Ok(instance_reply_id)
    }

    pub fn load_pending_instance(&self, storage: &mut dyn Storage, msg_id: u64) -> StdResult<Addr> {
        self.pending_instance_creations.load(storage, msg_id)
    }

    pub fn remove_pending_instance(&self, storage: &mut dyn Storage, msg_id: u64) {
        self.pending_instance_creations.remove(storage, msg_id)
    }

    pub fn save(&self, storage: &mut dyn Storage, msg_id: u64, lease_addr: Addr) -> StdResult<()> {
        let owner_addr = self.pending_instance_creations.load(storage, msg_id)?;
        self.loans.save(storage, owner_addr, &lease_addr)?;
        self.pending_instance_creations.remove(storage, msg_id);

        Ok(())
    }
}
