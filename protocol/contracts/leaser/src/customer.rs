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

pub(crate) trait CustomerLeases
where
    Self: Iterator<Item = MaybeCustomer<Self::Leases>>,
    Self::Leases: ExactSizeIterator<Item = Addr>,
{
    type Leases;
}

impl<Customers, Leases> CustomerLeases for Customers
where
    Customers: Iterator<Item = MaybeCustomer<Leases>>,
    Leases: ExactSizeIterator<Item = Addr>,
{
    type Leases = Leases;
}
