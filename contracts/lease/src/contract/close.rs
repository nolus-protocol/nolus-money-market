use cosmwasm_std::{Addr, Timestamp};
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use platform::bank::BankAccount;
use platform::batch::{Batch, Emitter};
use serde::Serialize;

use crate::error::ContractError;
use crate::lease::{Lease, WithLease};

pub struct Close<'a, Bank> {
    sender: &'a Addr,
    lease: Addr,
    account: Bank,
    now: Timestamp,
}

impl<'a, Bank> Close<'a, Bank> {
    pub fn new(sender: &'a Addr, lease: Addr, account: Bank, now: Timestamp) -> Self {
        Self {
            sender,
            lease,
            account,
            now,
        }
    }
}

impl<'a, Bank> WithLease for Close<'a, Bank>
where
    Bank: BankAccount,
{
    type Output = Emitter;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        if !lease.owned_by(self.sender) {
            return Err(Self::Error::Unauthorized {});
        }

        lease.close(self.lease, self.account, self.now)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
