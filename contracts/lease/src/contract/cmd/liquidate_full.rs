use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::LpnCoin,
    error::ContractError,
    lease::{with_lease::WithLease, FullRepayReceipt, Lease},
};

use super::ReceiptDTO;

pub(crate) struct LiquidateResult {
    pub receipt: ReceiptDTO,
    pub messages: Batch,
}

impl LiquidateResult {
    fn new(receipt: ReceiptDTO, messages: Batch) -> Self {
        debug_assert!(
            receipt.close,
            "The full-liquidation payment should have repaid the total outstanding liability!"
        );
        Self { receipt, messages }
    }
}

impl<Lpn> From<FullRepayReceipt<Lpn>> for LiquidateResult
where
    Lpn: Currency,
{
    fn from(value: FullRepayReceipt<Lpn>) -> Self {
        let (receipt, messages) = value.decompose();
        Self::new(receipt.into(), messages)
    }
}

pub(crate) struct Liquidate {
    payment: LpnCoin,
    now: Timestamp,
    profit: ProfitRef,
}

impl Liquidate {
    pub fn new(payment: LpnCoin, now: Timestamp, profit: ProfitRef) -> Self {
        Self {
            payment,
            now,
            profit,
        }
    }
}

impl WithLease for Liquidate {
    type Output = LiquidateResult;

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
        // TODO [issue #92] request the needed amount from the Liquidation Fund and
        // make sure the message goes out before the liquidation messages.
        lease
            .liquidate_full(self.payment.try_into()?, self.now, self.profit.as_stub())
            .map(Into::into)
    }
}
