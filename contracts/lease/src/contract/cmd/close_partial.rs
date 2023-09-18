use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::bank::FixedAddressSender;
use sdk::cosmwasm_std::Timestamp;

use crate::{api::LeaseCoin, error::ContractResult, lease::Lease, loan::RepayReceipt};

use super::repayable::RepayFn;

pub(crate) struct CloseFn {
    asset: LeaseCoin,
}
impl CloseFn {
    pub fn new(asset: LeaseCoin) -> Self {
        Self { asset }
    }
}
impl RepayFn for CloseFn {
    fn do_repay<Lpn, Asset, Lpp, Oracle, Profit>(
        self,
        lease: &mut Lease<Lpn, Asset, Lpp, Oracle>,
        payment: Coin<Lpn>,
        now: Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Lpn: Currency,
        Lpp: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency,
        Profit: FixedAddressSender,
    {
        self.asset
            .try_into()
            .map_err(Into::into)
            .and_then(|asset| lease.close_partial(asset, payment, now, profit))
    }
}
