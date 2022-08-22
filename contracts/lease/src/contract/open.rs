use cosmwasm_std::{Addr, Coin as CwCoin, Reply};
use platform::bank;
use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use platform::batch::{Batch, Emit, Emitter};

use crate::error::ContractError;
use crate::event::TYPE;
use crate::lease::{Lease, WithLease};

pub struct OpenLoanReq<'a> {
    contract: Addr,
    downpayment: &'a [CwCoin],
}

impl<'a> OpenLoanReq<'a> {
    pub fn new(contract: Addr, downpayment: &'a [CwCoin]) -> Self {
        Self { contract, downpayment }
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

        let result = lease
            .open_loan_req(downpayment_lpn)
            .map_err(Self::Error::from)?;

        let emitter = result.batch
            .into_emitter(TYPE::Open)
            .emit("id", self.contract)
            .emit("customer", result.customer)
            .emit_percent_amount("air", result.annual_interest_rate + result.annual_interest_rate_margin)
            .emit("currency", result.currency)
            .emit("loan-pool-id", result.loan_pool_id)
            .emit("loan-symbol", Lpn::SYMBOL)
            .emit_coin_amount("loan-amount", result.loan_amount)
            // TODO when downpayment currency is replaced with a type parameter change from `Lpn` to the type parameter
            .emit("downpayment-symbol", Lpn::SYMBOL)
            .emit_coin_amount("downpayment-amount", downpayment_lpn);

        Ok(emitter)
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
