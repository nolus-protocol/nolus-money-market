use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::{bank::FixedAddressSender, message::Response as MessageResponse};
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::LpnCoin,
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
};

use super::repayable::Emitter;

pub(crate) struct Close<'a, ProfitSender, ChangeSender, EmitterT> {
    payment: LpnCoin,
    now: &'a Timestamp,
    profit: ProfitSender,
    change: ChangeSender,
    emitter_fn: EmitterT,
}

impl<'a, ProfitSender, ChangeSender, EmitterT> Close<'a, ProfitSender, ChangeSender, EmitterT> {
    pub fn new(
        payment: LpnCoin,
        now: &'a Timestamp,
        profit: ProfitSender,
        change: ChangeSender,
        emitter_fn: EmitterT,
    ) -> Self {
        Self {
            payment,
            now,
            profit,
            change,
            emitter_fn,
        }
    }
}

impl<'a, ProfitSender, ChangeSender, EmitterT> WithLease
    for Close<'a, ProfitSender, ChangeSender, EmitterT>
where
    ProfitSender: FixedAddressSender,
    ChangeSender: FixedAddressSender,
    EmitterT: Emitter,
{
    type Output = MessageResponse;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Lpp: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency,
    {
        let lease_addr = lease.addr().clone();

        // TODO [issue #92] request the needed amount from the Liquidation Fund
        // this holds true for both use cases - full liquidation and full close
        // make sure the message goes out before the liquidation messages.
        self.payment
            .try_into()
            .map_err(Into::into)
            .and_then(|payment| lease.close_full(payment, self.now, self.profit, self.change))
            .map(|result| {
                let (receipt, messages) = result.decompose();
                MessageResponse::messages_with_events(
                    messages,
                    self.emitter_fn.emit(&lease_addr, &receipt),
                )
            })
    }
}
