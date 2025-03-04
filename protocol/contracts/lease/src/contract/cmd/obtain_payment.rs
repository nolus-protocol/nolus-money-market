use currency::{Currency, CurrencyDef, MemberOf};
use finance::coin::{Coin, WithCoin, WithCoinResult};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::bank;
use sdk::cosmwasm_std::Coin as CwCoin;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies, PaymentCoin},
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency},
    lease::{Lease, with_lease::WithLease},
};

pub(crate) struct ObtainPayment {
    cw_amount: Vec<CwCoin>,
}

impl ObtainPayment {
    pub(crate) fn new(cw_amount: Vec<CwCoin>) -> Self {
        Self { cw_amount }
    }
}

impl WithLease for ObtainPayment {
    type Output = PaymentCoin;

    type Error = ContractError;

    fn exec<Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: Currency + MemberOf<LeaseAssetCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
    {
        bank::may_received(&self.cw_amount, RepaymentHandler::<_, _, _> { lease })
            .ok_or_else(ContractError::NoPaymentError)?
    }
}

struct RepaymentHandler<Asset, LppLoan, Oracle> {
    lease: Lease<Asset, LppLoan, Oracle>,
}

impl<Asset, LppLoan, Oracle> WithCoin<LeasePaymentCurrencies>
    for RepaymentHandler<Asset, LppLoan, Oracle>
where
    Asset: Currency + MemberOf<LeaseAssetCurrencies>,
    LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
    Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
{
    type Output = PaymentCoin;

    type Error = ContractError;

    fn on<C>(self, coin: Coin<C>) -> WithCoinResult<LeasePaymentCurrencies, Self>
    where
        C: CurrencyDef,
        C::Group: MemberOf<LeasePaymentCurrencies>,
    {
        self.lease.validate_repay(coin).map(|()| coin.into())
    }
}
