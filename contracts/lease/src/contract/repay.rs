use cosmwasm_std::{Coin as CwCoin, Env};
use serde::Serialize;

use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::{
    bank::{self, BankAccountView},
    batch::{Emit, Emitter},
};

use crate::{
    error::ContractError,
    event::TYPE,
    lease::{Lease, LeaseDTO, RepayResult as LeaseRepayResult, WithLease},
};

pub struct Repay<'a, Bank>
where
    Bank: BankAccountView,
{
    payment: &'a [CwCoin],
    env: &'a Env,
    account: Bank,
}

impl<'a, Bank> Repay<'a, Bank>
where
    Bank: BankAccountView,
{
    pub fn new(payment: &'a [CwCoin], account: Bank, env: &'a Env) -> Self {
        Self {
            payment,
            env,
            account,
        }
    }
}

pub struct RepayResult {
    pub lease_dto: LeaseDTO,
    pub emitter: Emitter,
}

impl<'a, Bank> WithLease for Repay<'a, Bank>
where
    Bank: BankAccountView,
{
    type Output = RepayResult;

    type Error = ContractError;

    fn exec<Lpn, Lpp, Oracle>(
        self,
        lease: Lease<Lpn, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
    {
        // TODO 'receive' the payment from the bank using any currency it might be in
        let payment = bank::received::<Lpn>(self.payment)?;

        let lease_amount = self.account.balance::<Lpn>()? - payment;

        let LeaseRepayResult {
            batch,
            lease_dto,
            receipt,
        } = lease.repay(
            lease_amount,
            payment,
            self.env.block.time,
            self.env.contract.address.clone(),
        )?;

        let emitter = batch
            .into_emitter(TYPE::Repay)
            .emit_tx_info(self.env)
            .emit("to", self.env.contract.address.clone())
            .emit("payment-symbol", Lpn::SYMBOL)
            .emit_coin_amount("payment-amount", payment)
            .emit_timestamp("at", &self.env.block.time)
            .emit_to_string_value("loan-close", receipt.close())
            .emit_coin_amount("prev-margin-interest", receipt.previous_margin_paid())
            .emit_coin_amount("prev-loan-interest", receipt.previous_interest_paid())
            .emit_coin_amount("curr-margin-interest", receipt.current_margin_paid())
            .emit_coin_amount("curr-loan-interest", receipt.current_interest_paid())
            .emit_coin_amount("principal", receipt.principal_paid());

        Ok(RepayResult { lease_dto, emitter })
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
