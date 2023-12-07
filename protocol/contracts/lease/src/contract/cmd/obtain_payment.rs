use currencies::PaymentGroup;
use currency::Currency;
use finance::coin::{Coin, WithCoin, WithCoinResult};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::bank;
use sdk::cosmwasm_std::Coin as CwCoin;

use crate::{
    api::PaymentCoin,
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
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

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Lpn, Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Asset: Currency,
        LppLoan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
    {
        bank::may_received::<PaymentGroup, _>(&self.cw_amount, RepaymentHandler { lease })
            .ok_or_else(ContractError::NoPaymentError)?
    }
}

struct RepaymentHandler<Lpn, Asset, LppLoan, Oracle> {
    lease: Lease<Lpn, Asset, LppLoan, Oracle>,
}

impl<Lpn, Asset, LppLoan, Oracle> WithCoin for RepaymentHandler<Lpn, Asset, LppLoan, Oracle>
where
    Lpn: Currency,
    Asset: Currency,
    LppLoan: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
{
    type Output = PaymentCoin;

    type Error = ContractError;

    fn on<C>(&self, coin: Coin<C>) -> WithCoinResult<Self>
    where
        C: Currency,
    {
        self.lease.validate_repay(coin).map(|()| coin.into())
    }
}
