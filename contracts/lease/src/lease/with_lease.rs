use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;

use super::Lease;

pub trait WithLease {
    type Output;
    type Error;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Lpn, Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Asset: Currency,
        LppLoan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>;
}
