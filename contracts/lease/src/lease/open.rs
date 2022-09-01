use cosmwasm_std::{Addr, Reply, Timestamp};
use serde::Serialize;

use finance::{
    coin::Coin,
    currency::{Currency, SymbolOwned},
    percent::Percent,
};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::{bank::BankAccountView, batch::Batch};

use crate::{
    error::ContractResult,
    lease::{Lease, Status},
};

impl<Lpn, Lpp, Oracle> Lease<Lpn, Lpp, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
{
    pub(crate) fn open_loan_req(self, downpayment: Coin<Lpn>) -> ContractResult<Batch> {
        // TODO add a type parameter to this function to designate the downpayment currency
        // TODO query the market price oracle to get the price of the downpayment currency to LPN
        //  and calculate `downpayment` in LPN
        let borrow = self.liability.init_borrow_amount(downpayment);

        self.loan.open_loan_req(borrow).map_err(Into::into)
    }

    // TODO lease currency can be different than Lpn, therefore result's type parameter
    pub(crate) fn open_loan_resp<B>(
        mut self,
        lease: Addr,
        resp: Reply,
        account: B,
        now: &Timestamp,
    ) -> ContractResult<Result<Lpn>>
    where
        B: BankAccountView,
    {
        self.initial_alarm_schedule(lease, account.balance()?, now, &Status::None)?;

        let result = self.loan.open_loan_resp(resp).map({
            // Force move before closure to avoid edition warning from clippy;
            let customer = self.customer;
            let currency = self.currency;
            let oracle = self.oracle;

            |result| Result {
                batch: result.batch.merge(oracle.into()),
                customer,
                annual_interest_rate: result.annual_interest_rate,
                currency,
                loan_pool_id: result.loan_pool_id,
                loan_amount: result.borrowed,
            }
        })?;

        Ok(result)
    }
}

pub(crate) struct Result<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub customer: Addr,
    pub annual_interest_rate: Percent,
    pub currency: SymbolOwned,
    pub loan_pool_id: Addr,
    pub loan_amount: Coin<Lpn>,
}
