use crate::AccessPermission;
use sdk::{cosmwasm_std::Addr, cosmwasm_std::ContractInfo};

pub struct SingleUserPermission<'a> {
    addr: &'a Addr,
}

impl<'a> SingleUserPermission<'a> {
    pub fn new(addr: &'a Addr) -> Self {
        Self { addr }
    }
}

impl AccessPermission for SingleUserPermission<'_> {
    fn is_granted_to(&self, caller: &Addr) -> bool {
        self.addr == caller
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
    fn is_granted_to(&self, caller: &Addr) -> bool {
        self.contract_info.address == caller
    }
}
