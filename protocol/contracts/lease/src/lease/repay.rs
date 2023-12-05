use currencies::PaymentGroup;
use sdk::cosmwasm_std::Timestamp;

use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::bank::FixedAddressSender;

use crate::{error::ContractResult, lease::Lease, loan::RepayReceipt};

impl<Lpn, Asset, Lpp, Oracle> Lease<Lpn, Asset, Lpp, Oracle>
where
    Lpn: Currency,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency,
{
    pub(crate) fn validate_repay<PaymentC>(&self, payment: Coin<PaymentC>) -> ContractResult<()>
    where
        PaymentC: Currency,
    {
        self.oracle
            .price_of::<PaymentC, PaymentGroup>()
            .map_err(Into::into)
            .and_then(|price| self.position.validate_payment(payment, price))
    }

    pub(crate) fn repay<Profit>(
        &mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Profit: FixedAddressSender,
    {
        self.loan.repay(payment, now, profit)
    }
}
