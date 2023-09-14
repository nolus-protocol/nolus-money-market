use platform::{bank::FixedAddressSender, batch::Batch};

use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    error::{ContractError, ContractResult},
    lease::Lease,
    loan::RepayReceipt,
};

pub(crate) struct FullRepayReceipt<Lpn>
where
    Lpn: Currency,
{
    receipt: RepayReceipt<Lpn>,
    messages: Batch,
}

impl<Lpn> FullRepayReceipt<Lpn>
where
    Lpn: Currency,
{
    fn new(receipt: RepayReceipt<Lpn>, messages: Batch) -> Self {
        debug_assert!(receipt.close());
        Self { receipt, messages }
    }

    pub(crate) fn decompose(self) -> (RepayReceipt<Lpn>, Batch) {
        (self.receipt, self.messages)
    }
}

impl<Lpn, Asset, Lpp, Oracle> Lease<Lpn, Asset, Lpp, Oracle>
where
    Lpn: Currency,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency,
{
    pub(crate) fn close_partial<Profit>(
        &mut self,
        asset: Coin<Asset>,
        payment: Coin<Lpn>,
        now: Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Profit: FixedAddressSender,
    {
        let amount = self.amount();
        debug_assert!(
            asset < amount,
            "Liquidated asset {asset} should be less than the available {0}",
            amount
        );
        self.position.decrease_amount_with(asset);
        self.repay(payment, now, profit)
    }

    pub(crate) fn close_full<Profit>(
        mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
        mut profit: Profit,
    ) -> ContractResult<FullRepayReceipt<Lpn>>
    where
        Profit: FixedAddressSender,
    {
        // TODO [issue #92] debug_assert!(payment >= self.state().total_due())
        let receipt = self.repay(payment, now, &mut profit).and_then(|receipt| {
            if receipt.close() {
                Ok(receipt)
            } else {
                Err(ContractError::InsufficientLiquidation()) //issue #92
            }
        })?;

        profit.send(receipt.change());

        self.try_into_messages().map(|lease_messages| {
            FullRepayReceipt::new(receipt, lease_messages.merge(profit.into()))
        })
    }
}
