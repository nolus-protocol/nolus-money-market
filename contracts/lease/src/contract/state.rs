use cosmwasm_std::{to_binary, Addr, Binary, Timestamp};
use finance::bank::BankAccount;
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use serde::Serialize;

use crate::error::ContractError;
use crate::lease::{Lease, WithLease};

pub struct LeaseState<Bank> {
    now: Timestamp,
    account: Bank,
    lease: Addr,
}

impl<Bank> LeaseState<Bank> {
    pub fn new(now: Timestamp, account: Bank, lease: Addr) -> Self {
        Self {
            now,
            account,
            lease,
        }
    }
}

impl<Bank> WithLease for LeaseState<Bank>
where
    Bank: BankAccount,
{
    type Output = Binary;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        let resp = lease.state(self.now, &self.account, self.lease)?;
        to_binary(&resp).map_err(ContractError::from)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
