use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::{bank::FixedAddressSender, message::Response as MessageResponse};
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    error::ContractError,
    finance::{LpnCoinDTO, LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    lease::{Lease, with_lease::WithLease},
};

use super::repayable::Emitter;

pub(crate) struct Close<ProfitSender, ChangeSender, EmitterT> {
    payment: LpnCoinDTO,
    now: Timestamp,
    profit: ProfitSender,
    reserve: ReserveRef,
    change: ChangeSender,
    emitter_fn: EmitterT,
}

impl<ProfitSender, ChangeSender, EmitterT> Close<ProfitSender, ChangeSender, EmitterT> {
    pub fn new(
        payment: LpnCoinDTO,
        now: Timestamp,
        profit: ProfitSender,
        reserve: ReserveRef,
        change: ChangeSender,
        emitter_fn: EmitterT,
    ) -> Self {
        Self {
            payment,
            now,
            profit,
            reserve,
            change,
            emitter_fn,
        }
    }
}

impl<ProfitSender, ChangeSender, EmitterT> WithLease for Close<ProfitSender, ChangeSender, EmitterT>
where
    ProfitSender: FixedAddressSender,
    ChangeSender: FixedAddressSender,
    EmitterT: Emitter,
{
    type Output = MessageResponse;

    type Error = ContractError;

    fn exec<Asset, Lpp, Oracle>(
        self,
        lease: Lease<Asset, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        Lpp: LppLoanTrait<LpnCurrency>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        let lease_addr = lease.addr().clone();

        self.payment
            .try_into()
            .map_err(Into::into)
            .and_then(|payment| {
                lease.close_full(
                    payment,
                    self.now,
                    self.profit,
                    self.reserve.into_reserve(),
                    self.change,
                )
            })
            .map(|result| {
                let (receipt, messages) = result.decompose();
                MessageResponse::messages_with_event(
                    messages,
                    self.emitter_fn.emit(&lease_addr, &receipt),
                )
            })
    }
}
