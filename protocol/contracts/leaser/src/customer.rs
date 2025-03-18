use sdk::cosmwasm_std::Addr;

use crate::result::ContractResult;

pub(crate) struct Customer<Leases> {
    pub customer: Addr,
    pub leases: Leases,
}

impl<Leases> Customer<Leases> {
    pub fn from(customer: Addr, leases: Leases) -> Self {
        Self { customer, leases }
    }
}

pub(crate) type MaybeCustomer<Leases> = ContractResult<Customer<Leases>>;
