use sdk::cosmwasm_std::{Addr, ContractInfo};

use crate::{AccessPermission, Sender};

pub type DexResponseSafeDeliveryPermission<'a> = SameContractOnly<'a>;

pub struct SingleUserPermission<'a> {
    addr: &'a Addr,
}

impl<'a> SingleUserPermission<'a> {
    pub fn new(addr: &'a Addr) -> Self {
        Self { addr }
    }
}

impl AccessPermission for SingleUserPermission<'_> {
    fn granted_to(&self, sender: &Sender) -> bool {
        self.addr == sender.as_ref()
    }
}

pub struct SameContractOnly<'a> {
    contract_info: &'a ContractInfo,
}

impl<'a> SameContractOnly<'a> {
    pub fn new(contract_info: &'a ContractInfo) -> Self {
        Self { contract_info }
    }
}

impl AccessPermission for SameContractOnly<'_> {
    fn granted_to(&self, sender: &Sender) -> bool {
        self.contract_info.address == sender.as_ref()
    }
}
