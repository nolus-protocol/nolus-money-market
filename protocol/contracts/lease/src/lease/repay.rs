use currency::{Currency, CurrencyDef, MemberOf};
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::bank::FixedAddressSender;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    error::ContractResult,
    finance::{LpnCoin, LpnCurrencies, LpnCurrency},
    lease::Lease,
    loan::RepayReceipt,
};

impl<Asset, Lpp, Oracle> Lease<Asset, Lpp, Oracle>
where
    Lpp: LppLoanTrait<LpnCurrency, LpnCurrencies>,
    Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
    Asset: Currency + MemberOf<LeaseAssetCurrencies>,
{
    pub(crate) fn validate_repay<PaymentC>(&self, payment: Coin<PaymentC>) -> ContractResult<()>
    where
        PaymentC: CurrencyDef,
        PaymentC::Group: MemberOf<LeasePaymentCurrencies>,
    {
        self.oracle
            .price_of::<PaymentC>()
            .map_err(Into::into)
            .and_then(|price| {
                self.position
                    .validate_payment(payment, price)
                    .map_err(Into::into)
            })
    }

    pub(crate) fn repay<Profit>(
        &mut self,
        payment: LpnCoin,
        now: &Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt>
    where
        Profit: FixedAddressSender,
    {
        self.loan.repay(payment, now, profit)
    }
}
