use cosmwasm_std::{Addr, Coin as CwCoin, Timestamp};
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use platform::{
    batch::{
        Emit,
        Emitter
    },
    bank,
};
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
    type Output = Emitter;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        // TODO 'receive' the payment from the bank using any currency it might be in
        let payment = bank::received::<Lpn>(self.payment)?;

        let result = lease.repay(payment, self.now, self.lease)?;

        let emitter = result.batch.into_emitter(TYPE::Repay)
            .emit("payment-symbol", Lpn::SYMBOL)
            .emit_coin_amount("payment-amount", payment)
            .emit_timestamp("at", &self.now)
            .emit_to_string_value("loan-close", result.receipt.close())
            .emit_coin_amount("prev-margin-interest", result.receipt.previous_margin_paid())
            .emit_coin_amount("prev-loan-interest", result.receipt.previous_interest_paid())
            .emit_coin_amount("curr-margin-interest", result.receipt.current_margin_paid())
            .emit_coin_amount("curr-loan-interest", result.receipt.current_interest_paid())
            .emit_coin_amount("principal", result.receipt.principal_paid());

        Ok(emitter)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
