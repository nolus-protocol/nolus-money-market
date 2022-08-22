use cosmwasm_std::{Addr, Timestamp};
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use platform::{bank::BankAccount, batch::{Emitter, Emit}};
use serde::Serialize;

use crate::event::TYPE;
use crate::{
    error::ContractError,
    lease::{Lease, WithLease},
};

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

        let result = lease.close(self.lease.clone(), self.account)?;

        let emitter = result
            .into_emitter(TYPE::Close)
            .emit("id", self.lease)
            .emit_timestamp("at", &self.now);

        Ok(emitter)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
