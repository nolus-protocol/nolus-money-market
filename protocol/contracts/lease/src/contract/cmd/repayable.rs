use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::{
    bank::FixedAddressSender, batch::Emitter as PlatformEmitter,
    message::Response as MessageResponse,
};
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    contract::SplitDTOOut,
    error::{ContractError, ContractResult},
    finance::{LpnCoin, LpnCoinDTO, LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    lease::{with_lease::WithLease, IntoDTOResult, Lease as LeaseDO, LeaseDTO},
    loan::RepayReceipt,
};

use super::{check_close, CloseStatusDTO};

pub(crate) trait RepayFn {
    fn do_repay<Asset, Lpp, Oracle, Profit>(
        self,
        lease: &mut LeaseDO<Asset, Lpp, Oracle>,
        amount: LpnCoin,
        now: &Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        Lpp: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
        Profit: FixedAddressSender;
}

pub(crate) trait Emitter {
    fn emit(self, lease: &Addr, receipt: &RepayReceipt) -> PlatformEmitter;
}

pub(crate) struct Repay<'a, RepayableT, EmitterT>
where
    RepayableT: RepayFn,
    EmitterT: Emitter,
{
    repay_fn: RepayableT,
    amount: LpnCoinDTO,
    now: &'a Timestamp,
    emitter_fn: EmitterT,
    profit: ProfitRef,
    alarms: (TimeAlarmsRef, OracleRef),
    reserve: ReserveRef,
}

impl<'a, RepayableT, EmitterT> Repay<'a, RepayableT, EmitterT>
where
    RepayableT: RepayFn,
    EmitterT: Emitter,
{
    pub fn new(
        repay_fn: RepayableT,
        amount: LpnCoinDTO,
        now: &'a Timestamp,
        emitter_fn: EmitterT,
        profit: ProfitRef,
        alarms: (TimeAlarmsRef, OracleRef),
        reserve: ReserveRef,
    ) -> Self {
        Self {
            repay_fn,
            amount,
            now,
            emitter_fn,
            profit,
            alarms,
            reserve,
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
    pub close_status: CloseStatusDTO,
}

impl<'a, RepayableT, EmitterT> WithLease for Repay<'a, RepayableT, EmitterT>
where
    RepayableT: RepayFn,
    EmitterT: Emitter,
{
    type Output = RepayLeaseResult;

    type Error = ContractError;

    fn exec<Asset, Lpp, Oracle>(
        self,
        mut lease: LeaseDO<Asset, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        Lpp: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        let amount = self.amount.try_into()?;
        let mut profit_sender = self.profit.clone().into_stub();

        let receipt = self
            .repay_fn
            .do_repay(&mut lease, amount, self.now, &mut profit_sender)?;
        let events = self.emitter_fn.emit(lease.addr(), &receipt);

        let close_status =
            check_close::check_close(&lease, self.now, &self.alarms.0, &self.alarms.1)?;
        debug_assert!(!(receipt.close() ^ matches!(close_status, CloseStatusDTO::Paid))); // receipt.close() <=> status is CloseStatusDTO::Paid

        lease
            .try_into_dto(self.profit, self.alarms.0, self.reserve)
            .map(
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
                            close_status,
                        },
                    }
                },
            )
    }
}
