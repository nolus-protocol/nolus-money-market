use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::{
    bank::{self},
    batch::{Emit, Emitter},
};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{Coin as CwCoin, Env};
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractError,
    event::Type,
    lease::{with_lease::WithLease, Lease, LeaseDTO, RepayResult as LeaseRepayResult},
};

pub struct Repay<'a> {
    payment: Vec<CwCoin>,
    env: &'a Env,
}

impl<'a> Repay<'a> {
    pub fn new(payment: Vec<CwCoin>, env: &'a Env) -> Self {
        Self { payment, env }
    }
}

pub struct RepayResult {
    pub lease: LeaseDTO,
    pub emitter: Emitter,
}

impl<'a> WithLease for Repay<'a> {
    type Output = RepayResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        // TODO 'receive' the payment from the bank using any currency it might be in
        let payment = bank::received::<Lpn>(self.payment)?;

        let LeaseRepayResult {
            batch,
            lease: lease_dto,
            receipt,
        } = lease.repay(payment, self.env.block.time)?;

        let emitter = batch
            .into_emitter(Type::Repay)
            .emit_tx_info(self.env)
            .emit("to", self.env.contract.address.clone())
            .emit_currency::<_, Lpn>("payment-symbol")
            .emit_coin_amount("payment-amount", payment)
            .emit_to_string_value("loan-close", receipt.close())
            .emit_coin_amount("prev-margin-interest", receipt.previous_margin_paid())
            .emit_coin_amount("prev-loan-interest", receipt.previous_interest_paid())
            .emit_coin_amount("curr-margin-interest", receipt.current_margin_paid())
            .emit_coin_amount("curr-loan-interest", receipt.current_interest_paid())
            .emit_coin_amount("principal", receipt.principal_paid())
            .emit_coin_amount("change", receipt.change());

        Ok(RepayResult {
            lease: lease_dto,
            emitter,
        })
    }
}
