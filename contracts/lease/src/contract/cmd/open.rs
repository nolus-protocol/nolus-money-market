use std::marker::PhantomData;

use cosmwasm_std::{Coin as CwCoin, Env, QuerierWrapper, Reply};
use currency::payment::PaymentGroup;
use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::{
    lender::{LppLender as LppLenderTrait, WithLppLender},
    LppBatch,
};
use market_price_oracle::{convert, stub::OracleRef};
use platform::{
    bank,
    batch::{Batch, Emit, Emitter},
    coin_legacy::CoinVisitor,
};

use crate::{error::ContractError, event::TYPE, lease::DownpaymentDTO, msg::NewLeaseForm};

pub struct OpenLoanReq<'a> {
    form: &'a NewLeaseForm,
    funds_in: Vec<CwCoin>,
    oracle: OracleRef,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> OpenLoanReq<'a> {
    pub fn new(
        form: &'a NewLeaseForm,
        funds_in: Vec<CwCoin>,
        oracle: OracleRef,
        querier: &'a QuerierWrapper<'a>,
    ) -> Self {
        Self {
            form,
            funds_in,
            oracle,
            querier,
        }
    }
}

impl<'a> WithLppLender for OpenLoanReq<'a> {
    type Output = OpenLoanReqResult;

    type Error = ContractError;

    fn exec<Lpn, LppLender>(self, mut lpp: LppLender) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        LppLender: LppLenderTrait<Lpn>,
    {
        let (downpayment, downpayment_lpn) = bank::received_any::<PaymentGroup, _>(
            self.funds_in,
            DownpaymentHandler {
                oracle: self.oracle,
                _lpn: PhantomData::<Lpn> {},
                querier: self.querier,
            },
        )?;
        let borrow_lpn = self.form.liability.init_borrow_amount(downpayment_lpn);

        lpp.open_loan_req(borrow_lpn)?;

        Ok(OpenLoanReqResult {
            batch: lpp.into().batch,
            downpayment,
        })
    }
}

struct DownpaymentHandler<'a, Lpn> {
    oracle: OracleRef,
    _lpn: PhantomData<Lpn>,
    querier: &'a QuerierWrapper<'a>,
}
impl<'a, Lpn> CoinVisitor for DownpaymentHandler<'a, Lpn>
where
    Lpn: Currency,
{
    type Output = (DownpaymentDTO, Coin<Lpn>);

    type Error = ContractError;

    fn on<C>(&self, in_amount: Coin<C>) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
    {
        let downpayment_lpn = convert::to_base(self.oracle.clone(), in_amount, self.querier)?;
        let downpayment = DownpaymentDTO::new(in_amount.into());
        Ok((downpayment, downpayment_lpn))
    }
}

pub struct OpenLoanReqResult {
    pub(in crate::contract) batch: Batch,
    pub(in crate::contract) downpayment: DownpaymentDTO,
}

pub struct OpenLoanResp<'a> {
    reply: Reply,
    form: &'a NewLeaseForm,
    downpayment: DownpaymentDTO,
    env: &'a Env,
}

impl<'a> OpenLoanResp<'a> {
    pub fn new(
        reply: Reply,
        form: &'a NewLeaseForm,
        downpayment: DownpaymentDTO,
        env: &'a Env,
    ) -> Self {
        Self {
            reply,
            form,
            downpayment,
            env,
        }
    }
}

impl<'a> WithLppLender for OpenLoanResp<'a> {
    type Output = Emitter;

    type Error = ContractError;

    fn exec<Lpn, LppLender>(self, lpp: LppLender) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        LppLender: LppLenderTrait<Lpn>,
    {
        let loan_resp = lpp.open_loan_resp(self.reply)?;
        let LppBatch { lpp_ref, batch } = lpp.into();
        Ok(batch
            .into_emitter(TYPE::Open)
            .emit_tx_info(self.env)
            .emit("id", self.env.contract.address.clone())
            .emit("customer", self.form.customer.clone())
            //TODO get rid of the manual calculation when the event got emitted after the lease creation
            .emit_percent_amount(
                "air",
                loan_resp.annual_interest_rate + self.form.loan.annual_margin_interest,
            )
            .emit("currency", self.form.currency.clone())
            .emit("loan-pool-id", lpp_ref.addr())
            .emit_coin("loan", loan_resp.principal_due)
            .emit("downpayment-symbol", self.downpayment.symbol())
            .emit_to_string_value("downpayment-amount", self.downpayment.amount()))
    }
}
