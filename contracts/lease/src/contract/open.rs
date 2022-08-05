use cosmwasm_std::{Coin as CwCoin, Reply};
use platform::bank;
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use platform::batch::{Batch, Emitter};

use crate::error::ContractError;
use crate::lease::{Lease, WithLease};

pub struct OpenLoanReq<'a> {
    downpayment: &'a [CwCoin],
}

impl<'a> OpenLoanReq<'a> {
    pub fn new(downpayment: &'a [CwCoin]) -> Self {
        Self { downpayment }
    }
}

impl<'a> WithLease for OpenLoanReq<'a> {
    type Output = Emitter;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        // TODO 'receive' the downpayment from the bank using any currency it might be in
        let downpayment_lpn = bank::received::<Lpn>(self.downpayment)?;

        lease
            .open_loan_req(downpayment_lpn)
            .map_err(Self::Error::from)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

pub struct OpenLoanResp {
    resp: Reply,
}

impl OpenLoanResp {
    pub fn new(resp: Reply) -> Self {
        Self { resp }
    }
}

impl WithLease for OpenLoanResp {
    type Output = Batch;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        lease.open_loan_resp(self.resp)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
