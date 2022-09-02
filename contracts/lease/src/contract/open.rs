use cosmwasm_std::{Coin as CwCoin, Env, Reply};
use serde::Serialize;

use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::{
    bank::{self, BankAccountView},
    batch::{Batch, Emit, Emitter},
};

use crate::{
    error::ContractError,
    event::TYPE,
    lease::{DownpaymentDTO, Lease, WithLease},
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

    fn exec<Lpn, Lpp, Oracle>(
        self,
        lease: Lease<Lpn, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
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

pub struct OpenLoanResp<'a, B>
where
    B: BankAccountView,
{
    resp: Reply,
    downpayment: DownpaymentDTO,
    account: B,
    env: &'a Env,
}

impl<'a, B> OpenLoanResp<'a, B>
where
    B: BankAccountView,
{
    pub fn new(resp: Reply, downpayment: DownpaymentDTO, account: B, env: &'a Env) -> Self {
        Self {
            resp,
            downpayment,
            account,
            env,
        }
    }
}

impl<'a, B> WithLease for OpenLoanResp<'a, B>
where
    B: BankAccountView,
{
    type Output = Emitter;

    type Error = ContractError;

    fn exec<Lpn, Lpp, Oracle>(
        self,
        lease: Lease<Lpn, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
    {
        let result = lease.open_loan_resp(
            self.env.contract.address.clone(),
            self.resp,
            self.account,
            &self.env.block.time,
        )?;

        Ok(result
            .batch
            .into_emitter(TYPE::Open)
            .emit_tx_info(self.env)
            .emit("id", self.env.contract.address.clone())
            .emit("customer", result.lease_dto.customer)
            .emit_percent_amount("air", result.receipt.annual_interest_rate)
            .emit("currency", result.lease_dto.currency)
            .emit("loan-pool-id", result.lease_dto.loan.lpp().addr())
            .emit_coin("loan", result.receipt.borrowed)
            .emit("downpayment-symbol", self.downpayment.symbol())
            .emit_to_string_value("downpayment-amount", self.downpayment.amount()))
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
