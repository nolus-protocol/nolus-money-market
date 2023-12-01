use sdk::cosmwasm_std::Timestamp;

use currencies::PaymentGroup;
use finance::coin::{Coin, WithCoin};
use platform::bank;
use sdk::cosmwasm_std::Coin as CwCoin;

use crate::{
    api::PaymentCoin,
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
};

pub(crate) struct ValidatePayment {
    cw_amount: Vec<CwCoin>,
    now: Timestamp,
}

impl ValidatePayment {
    pub(crate) fn new(cw_amount: Vec<CwCoin>, now: Timestamp) -> Self {
        Self { cw_amount, now }
    }
}

impl WithLease for ValidatePayment {
    type Output = PaymentCoin;

    type Error = ContractError;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Lpn, Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: currency::Currency,
        Asset: currency::Currency,
        LppLoan: lpp::stub::loan::LppLoan<Lpn>,
        Oracle: oracle_platform::Oracle<Lpn>,
    {
        bank::may_received::<PaymentGroup, _>(
            self.cw_amount,
            RepaymentHandler {
                lease,
                now: self.now,
            },
        )
        .ok_or_else(ContractError::NoPaymentError)?
    }
}

struct RepaymentHandler<Lpn, Asset, LppLoan, Oracle> {
    lease: Lease<Lpn, Asset, LppLoan, Oracle>,
    now: Timestamp,
}

impl<Lpn, Asset, LppLoan, Oracle> WithCoin for RepaymentHandler<Lpn, Asset, LppLoan, Oracle>
where
    Lpn: currency::Currency,
    Asset: currency::Currency,
    LppLoan: lpp::stub::loan::LppLoan<Lpn>,
    Oracle: oracle_platform::Oracle<Lpn>,
{
    type Output = PaymentCoin;

    type Error = ContractError;

    fn on<C>(&self, coin: Coin<C>) -> finance::coin::WithCoinResult<Self>
    where
        C: currency::Currency,
    {
        self.lease
            .validate_repay(coin, self.now)
            .map(|validated| validated.into())
    }
}
