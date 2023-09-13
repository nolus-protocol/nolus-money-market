use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use platform::{bank::FixedAddressSender, batch::Batch};
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LpnCoin,
    error::{ContractError, ContractResult},
    lease::{with_lease::WithLease, IntoDTOResult, Lease, LeaseDTO},
    loan::RepayReceipt,
};

use super::{liquidation_status, LiquidationStatus};

pub(crate) trait Repayable {
    fn do_repay<Lpn, Asset, Lpp, Oracle, Profit>(
        self,
        lease: &mut Lease<Lpn, Asset, Lpp, Oracle>,
        payment: Coin<Lpn>,
        now: Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Lpn: Currency,
        Lpp: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency,
        Profit: FixedAddressSender;
}

pub(crate) struct Repay<RepayableT>
where
    RepayableT: Repayable,
{
    lease_fn: RepayableT,
    payment: LpnCoin,
    now: Timestamp,
    profit: ProfitRef,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
}

impl<RepayableT> Repay<RepayableT>
where
    RepayableT: Repayable,
{
    pub fn new(
        lease_fn: RepayableT,
        payment: LpnCoin,
        now: Timestamp,
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        price_alarms: OracleRef,
    ) -> Self {
        Self {
            lease_fn,
            payment,
            now,
            profit,
            time_alarms,
            price_alarms,
        }
    }
}

pub(crate) struct RepayResult {
    pub lease: LeaseDTO,
    pub receipt: ReceiptDTO,
    pub messages: Batch,
    pub liquidation: LiquidationStatus,
}

pub(crate) struct ReceiptDTO {
    pub total: LpnCoin,
    pub previous_margin_paid: LpnCoin,
    pub current_margin_paid: LpnCoin,
    pub previous_interest_paid: LpnCoin,
    pub current_interest_paid: LpnCoin,
    pub principal_paid: LpnCoin,
    pub change: LpnCoin,
    pub close: bool,
}

impl<RepayableT> WithLease for Repay<RepayableT>
where
    RepayableT: Repayable,
{
    type Output = RepayResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Oracle>(
        self,
        mut lease: Lease<Lpn, Asset, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Lpp: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency,
    {
        let payment = self.payment.try_into()?;
        let mut profit = self.profit.as_stub();

        let receipt = self
            .lease_fn
            .do_repay(&mut lease, payment, self.now, &mut profit)?;

        let liquidation = liquidation_status::status_and_schedule(
            &lease,
            self.now,
            &self.time_alarms,
            &self.price_alarms,
        )?;

        lease.try_into_dto(self.profit, self.time_alarms).map(
            |IntoDTOResult {
                 lease,
                 batch: messages,
             }| {
                RepayResult {
                    lease,
                    receipt: receipt.into(),
                    messages: messages.merge(profit.into()),
                    liquidation,
                }
            },
        )
    }
}

impl<Lpn> From<RepayReceipt<Lpn>> for ReceiptDTO
where
    Lpn: Currency,
{
    fn from(value: RepayReceipt<Lpn>) -> Self {
        Self {
            total: value.total().into(),
            previous_margin_paid: value.previous_margin_paid().into(),
            current_margin_paid: value.current_margin_paid().into(),
            previous_interest_paid: value.previous_interest_paid().into(),
            current_interest_paid: value.current_interest_paid().into(),
            principal_paid: value.principal_paid().into(),
            change: value.change().into(),
            close: value.close(),
        }
    }
}
