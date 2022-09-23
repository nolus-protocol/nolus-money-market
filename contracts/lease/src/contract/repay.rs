use cosmwasm_std::{Coin as CwCoin, Env};
use serde::Serialize;

use finance::{
    currency::Currency,
    price::{total, Price},
};
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::{
    bank::{self, BankAccountView},
    batch::{Emit, Emitter},
};
use profit::stub::Profit as ProfitTrait;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::lease::stub::WithLease;
use crate::{
    error::ContractError,
    event::TYPE,
    lease::{Lease, LeaseDTO, RepayResult as LeaseRepayResult},
};

pub struct Repay<'a, Bank>
where
    Bank: BankAccountView,
{
    payment: Vec<CwCoin>,
    env: &'a Env,
    account: Bank,
}

impl<'a, Bank> Repay<'a, Bank>
where
    Bank: BankAccountView,
{
    pub fn new(payment: Vec<CwCoin>, account: Bank, env: &'a Env) -> Self {
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

        // TODO adjust/remove this logic when support for multiple currencies is added
        //  because this only works for `Asset = Lpn`
        let lease_amount = self.account.balance::<Asset>()? - total(payment, Price::identity());

        let LeaseRepayResult {
            batch,
            lease_dto,
            receipt,
        } = lease.repay(lease_amount, payment, self.env.block.time)?;

        let emitter = batch
            .into_emitter(TYPE::Repay)
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

        Ok(RepayResult { lease_dto, emitter })
    }
}
