use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use platform::{
    bank::FixedAddressSender, batch::Emitter as PlatformEmitter,
    message::Response as MessageResponse,
};
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LpnCoin,
    contract::SplitDTOOut,
    error::{ContractError, ContractResult},
    lease::{with_lease::WithLease, IntoDTOResult, Lease, LeaseDTO},
    loan::RepayReceipt,
};

use super::{liquidation_status, LiquidationStatus};

pub(crate) trait Repayable {
    fn do_repay<Lpn, Asset, Lpp, Oracle, Profit>(
        self,
        lease: &mut Lease<Lpn, Asset, Lpp, Oracle>,
        amount: Coin<Lpn>,
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

pub(crate) trait Emitter {
    // TODO
    // fn emit<Lpn, Asset, Lpp, Oracle>(
    fn emit(
        self,
        // TODO
        // lease: &Lease<Lpn, Asset, Lpp, Oracle>,
        // receipt: RepayReceipt<Lpn>,
        lease: &Addr,
        receipt: &ReceiptDTO,
    ) -> PlatformEmitter;
    // where
    //     Lpn: Currency;
}

pub(crate) struct Repay<RepayableT, EmitterT>
where
    RepayableT: Repayable,
    EmitterT: Emitter,
{
    repay_fn: RepayableT,
    amount: LpnCoin,
    now: Timestamp,
    emitter_fn: EmitterT,
    profit: ProfitRef,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
}

impl<RepayableT, EmitterT> Repay<RepayableT, EmitterT>
where
    RepayableT: Repayable,
    EmitterT: Emitter,
{
    pub fn new(
        repay_fn: RepayableT,
        amount: LpnCoin,
        now: Timestamp,
        emitter_fn: EmitterT,
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        price_alarms: OracleRef,
    ) -> Self {
        Self {
            repay_fn,
            amount,
            now,
            emitter_fn,
            profit,
            time_alarms,
            price_alarms,
        }
    }
}

pub(crate) struct RepayLeaseResult {
    lease: LeaseDTO,
    result: RepayResult,
}

impl SplitDTOOut for RepayLeaseResult {
    type Other = RepayResult;

    fn split_into(self) -> (LeaseDTO, Self::Other) {
        (self.lease, self.result)
    }
}

pub(crate) struct RepayResult {
    pub response: MessageResponse,
    pub loan_paid: bool,
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

impl<RepayableT, EmitterT> WithLease for Repay<RepayableT, EmitterT>
where
    RepayableT: Repayable,
    EmitterT: Emitter,
{
    type Output = RepayLeaseResult;

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
        let amount = self.amount.try_into()?;
        let mut profit = self.profit.as_stub();

        let receipt = self
            .repay_fn
            .do_repay(&mut lease, amount, self.now, &mut profit)?;

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
                let receipt_dto = receipt.into();
                let events = self.emitter_fn.emit(&lease.addr, &receipt_dto);
                RepayLeaseResult {
                    lease,
                    result: RepayResult {
                        response: MessageResponse::messages_with_events(
                            messages.merge(profit.into()),
                            events,
                        ),
                        loan_paid: receipt_dto.close,
                        liquidation,
                    },
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
