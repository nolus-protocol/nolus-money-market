use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::message::Response as MessageResponse;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::LpnCoin,
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
};

use super::repayable::Emitter;

pub(crate) struct Close<EmitterT> {
    payment: LpnCoin,
    now: Timestamp,
    emitter_fn: EmitterT,
    profit: ProfitRef,
}

impl<EmitterT> Close<EmitterT> {
    pub fn new(payment: LpnCoin, now: Timestamp, emitter_fn: EmitterT, profit: ProfitRef) -> Self {
        Self {
            payment,
            now,
            emitter_fn,
            profit,
        }
    }
}

impl<EmitterT> WithLease for Close<EmitterT>
where
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
        lease
            .close_full(self.payment.try_into()?, self.now, self.profit.as_stub())
            .map(|result| {
                let (receipt, messages) = result.decompose();
                MessageResponse::messages_with_events(
                    messages,
                    self.emitter_fn.emit(&lease_addr, &receipt),
                )
            })
    }
}
