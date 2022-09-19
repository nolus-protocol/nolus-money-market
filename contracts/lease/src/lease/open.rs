use cosmwasm_std::{Reply, Timestamp};
use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::{bank::BankAccountView, batch::Batch};
use profit::stub::Profit as ProfitTrait;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{error::ContractResult, lease::Lease, loan::OpenReceipt};

use super::LeaseDTO;

impl<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
    Lease<'r, Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    pub(crate) fn open_loan_req(mut self, downpayment: Coin<Lpn>) -> ContractResult<Batch> {
        // TODO add a type parameter to this function to designate the downpayment currency
        // TODO query the market price oracle to get the price of the downpayment currency to LPN
        //  and calculate `downpayment` in LPN
        let borrow = self.liability.init_borrow_amount(downpayment);

        self.loan.open_loan_req(borrow)?;

        let (_lease_dto, batch) = self.into_dto();
        Ok(batch)
    }

    // TODO lease currency can be different than Lpn, therefore result's type parameter
    pub(crate) fn open_loan_resp<B>(
        mut self,
        resp: Reply,
        account: B,
        now: &Timestamp,
    ) -> ContractResult<Result<Lpn>>
    where
        B: BankAccountView,
    {
        self.initial_alarm_schedule(account.balance()?, now)?;

        self.loan.open_loan_resp(resp).map({
            let (lease_dto, batch) = self.into_dto();

            |receipt| Result {
                batch,
                lease_dto,
                receipt,
            }
        })
    }
}

pub(crate) struct Result<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub lease_dto: LeaseDTO,
    pub receipt: OpenReceipt<Lpn>,
}
