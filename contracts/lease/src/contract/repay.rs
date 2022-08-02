use cosmwasm_std::{Addr, Coin as CwCoin, Timestamp};
use platform::bank;
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use platform::batch::{Batch, Emit};
use serde::Serialize;

use crate::error::ContractError;
use crate::event::TYPE;
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
    type Output = Batch;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        // TODO 'receive' the payment from the bank using any currency it might be in
        let payment = bank::received::<Lpn>(self.payment)?;

        let result = lease.repay(payment, self.now, self.lease)?;

        let batch = { result.batch }
            .emit(TYPE::Repay, "payment-symbol", Lpn::SYMBOL)
            .emit_coin_amount(TYPE::Repay, "payment-amount", payment)
            .emit_timestamp(TYPE::Repay, "at", &self.now)
            .emit_bool(TYPE::Repay, "loan-close", result.paid.close())
            .emit_coin_amount(TYPE::Repay, "prev-margin-interest", result.paid.previous_margin_paid())
            .emit_coin_amount(TYPE::Repay, "prev-loan-interest", result.paid.previous_interest_paid())
            .emit_coin_amount(TYPE::Repay, "curr-margin-interest", result.paid.current_margin_paid())
            .emit_coin_amount(TYPE::Repay, "curr-loan-interest", result.paid.current_interest_paid())
            .emit_coin_amount(TYPE::Repay, "principal", result.paid.principal_paid());

        Ok(batch)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
