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
    fn emit<Lpn>(self, lease: &Addr, receipt: &RepayReceipt<Lpn>) -> PlatformEmitter
    where
        Lpn: Currency;
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
        let mut profit_sender = self.profit.clone().into_stub();

        let receipt = self
            .repay_fn
            .do_repay(&mut lease, amount, self.now, &mut profit_sender)?;
        let events = self.emitter_fn.emit(lease.addr(), &receipt);

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
                RepayLeaseResult {
                    lease,
                    result: RepayResult {
                        response: MessageResponse::messages_with_events(
                            messages.merge(profit_sender.into()),
                            events,
                        ),
                        loan_paid: receipt.close(),
                        liquidation,
                    },
                }
            },
        )
    }
}
