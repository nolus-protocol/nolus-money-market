use cosmwasm_std::{Coin as CwCoin, Env, Reply, Storage};

use finance::{
    coin::CoinDTO,
    currency::{Currency, SymbolOwned}
};
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
    storage: &'a mut dyn Storage,
}

impl<'a> OpenLoanReq<'a> {
    pub fn new(downpayment: &'a [CwCoin], storage: &'a mut dyn Storage) -> Self {
        Self { downpayment, storage }
    }
}

impl<'a> WithLease for OpenLoanReq<'a> {
    type Output = Batch;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        // TODO 'receive' the downpayment from the bank using any currency it might be in
        let downpayment_lpn = bank::received::<Lpn>(self.downpayment)?;

        DownpaymentDTO::new(CoinDTO::from(downpayment_lpn)).store(self.storage)?;

        lease
            .open_loan_req(downpayment_lpn)
            .map_err(Self::Error::from)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

pub struct OpenLoanResp<'a> {
    resp: Reply,
    storage: &'a mut dyn Storage,
    env: Env,
}

impl<'a> OpenLoanResp<'a> {
    pub fn new(resp: Reply, storage: &'a mut dyn Storage, env: Env) -> Self {
        Self { resp, storage, env }
    }
}

impl<'a> WithLease for OpenLoanResp<'a> {
    type Output = Emitter;

    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        let result = lease.open_loan_resp(self.resp)?;

        let downpayment = DownpaymentDTO::load(self.storage)?;

        DownpaymentDTO::remove(self.storage);

        Ok(result.batch
            .into_emitter(TYPE::Open)
            .emit_tx_info(&self.env)
            .emit("id", self.env.contract.address)
            .emit("customer", result.customer)
            .emit_percent_amount("air", result.annual_interest_rate + result.annual_interest_rate_margin)
            .emit("currency", result.currency)
            .emit("loan-pool-id", result.loan_pool_id)
            .emit_coin("loan", result.loan_amount)
            .emit("downpayment-symbol", downpayment.symbol())
            .emit_to_string_value("downpayment-amount", downpayment.amount()))
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
