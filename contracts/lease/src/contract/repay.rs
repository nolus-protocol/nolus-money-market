use cosmwasm_std::{Addr, Coin as CwCoin, SubMsg, Timestamp};
use finance::bank;
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use serde::Serialize;

use crate::error::ContractError;
use crate::lease::{Lease, WithLease};

pub struct Repay<'a> {
    payment: &'a [CwCoin],
    now: Timestamp,
    lease: Addr,
}

impl<'a> Repay<'a> {
    pub fn new(payment: &'a [CwCoin], now: Timestamp, lease: Addr) -> Self {
        Self {
            payment,
            now,
            lease,
        }
    }
}

impl<'a> WithLease for Repay<'a> {
    type Output = Option<SubMsg>;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, mut lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        // TODO 'receive' the payment from the bank using any currency it might be in
        let payment = bank::received::<Lpn>(self.payment)?;
        lease.repay(payment, self.now, self.lease)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
