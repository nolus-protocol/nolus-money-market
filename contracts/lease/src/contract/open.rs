use cosmwasm_std::{Coin as CwCoin, Env, Reply};

use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use platform::{
    bank,
    batch::{Batch, Emit, Emitter}
};

use crate::{
    error::ContractError,
    event::TYPE,
    lease::{DownpaymentDTO, Lease, WithLease}
};

pub struct OpenLoanReq<'a> {
    downpayment: &'a [CwCoin],
}

impl<'a> OpenLoanReq<'a> {
    pub fn new(downpayment: &'a [CwCoin]) -> Self {
        Self { downpayment }
    }
}

impl<'a> WithLease for OpenLoanReq<'a> {
    type Output = OpenLoanReqResult;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        // TODO 'receive' the downpayment from the bank using any currency it might be in
        let downpayment = bank::received::<Lpn>(self.downpayment)?;
        // TODO do swapping and convert to Lpn
        let downpayment_lpn = downpayment;

        Ok(OpenLoanReqResult {
            batch: lease.open_loan_req(downpayment_lpn)?,
            downpayment: DownpaymentDTO::new(downpayment.into()),
        })
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

pub struct OpenLoanReqResult {
    pub(super) batch: Batch,
    pub(super) downpayment: DownpaymentDTO,
}

pub struct OpenLoanResp {
    resp: Reply,
    downpayment: DownpaymentDTO,
    env: Env,
}

impl OpenLoanResp {
    pub fn new(resp: Reply, downpayment: DownpaymentDTO, env: Env) -> Self {
        Self { resp, downpayment, env }
    }
}

impl WithLease for OpenLoanResp {
    type Output = Emitter;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        let result = lease.open_loan_resp(self.resp)?;

        Ok(result.batch
            .into_emitter(TYPE::Open)
            .emit_tx_info(&self.env)
            .emit("id", self.env.contract.address)
            .emit("customer", result.customer)
            .emit_percent_amount("air", result.annual_interest_rate)
            .emit("currency", result.currency)
            .emit("loan-pool-id", result.loan_pool_id)
            .emit_coin("loan", result.loan_amount)
            .emit("downpayment-symbol", self.downpayment.symbol())
            .emit_to_string_value("downpayment-amount", self.downpayment.amount()))
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
