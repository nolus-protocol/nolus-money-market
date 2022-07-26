use cosmwasm_std::Addr;
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use platform::bank::BankAccount;
use platform::batch::Batch;
use serde::Serialize;

use crate::error::ContractError;
use crate::lease::{Lease, WithLease};

pub struct Close<'a, Bank> {
    sender: &'a Addr,
    lease: Addr,
    account: Bank,
}

impl<'a, Bank> Close<'a, Bank> {
    pub fn new(sender: &'a Addr, lease: Addr, account: Bank) -> Self {
        Self {
            sender,
            lease,
            account,
        }
    }
}

impl<'a, Bank> WithLease for Close<'a, Bank>
where
    Bank: BankAccount,
{
    type Output = Batch;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        if !lease.owned_by(self.sender) {
            return Err(Self::Error::Unauthorized {});
        }

        lease.close(self.lease, self.account)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
