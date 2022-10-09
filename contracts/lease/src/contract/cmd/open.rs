use std::marker::PhantomData;

use cosmwasm_std::{Coin as CwCoin, QuerierWrapper, Reply};
use currency::payment::PaymentGroup;
use serde::Serialize;

use finance::{
    coin::{Coin, CoinDTO},
    currency::Currency,
    percent::Percent,
};
use lpp::stub::{
    lender::{LppLender as LppLenderTrait, LppLenderRef, WithLppLender},
    LppBatch,
};
use market_price_oracle::{convert, stub::OracleRef};
use platform::{bank, batch::Batch, coin_legacy::CoinVisitor};

use crate::{error::ContractError, msg::NewLeaseForm};

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
        if downpayment_lpn.is_zero() {
            Err(Self::Error::NoDownpaymentError())
        } else {
            let borrow_lpn = self.form.liability.init_borrow_amount(downpayment_lpn);

            lpp.open_loan_req(borrow_lpn)?;

            Ok(Self::Output {
                batch: lpp.into().batch,
                downpayment,
            })
        }
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
    type Output = (CoinDTO, Coin<Lpn>);

    type Error = ContractError;

    fn on<C>(&self, in_amount: Coin<C>) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
    {
        let downpayment_lpn = convert::to_base(self.oracle.clone(), in_amount, self.querier)?;
        let downpayment = in_amount.into();
        Ok((downpayment, downpayment_lpn))
    }
}

pub struct OpenLoanReqResult {
    pub(in crate::contract) batch: Batch,
    pub(in crate::contract) downpayment: CoinDTO,
}

pub struct OpenLoanResp {
    reply: Reply,
    downpayment: CoinDTO,
}

impl OpenLoanResp {
    pub fn new(reply: Reply, downpayment: CoinDTO) -> Self {
        Self { reply, downpayment }
    }
}

impl WithLppLender for OpenLoanResp {
    type Output = OpenLoanRespResult;

    type Error = ContractError;

    fn exec<Lpn, LppLender>(self, lpp: LppLender) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        LppLender: LppLenderTrait<Lpn>,
    {
        let loan_resp = lpp.open_loan_resp(self.reply)?;
        let LppBatch { lpp_ref, batch } = lpp.into();
        debug_assert_eq!(Batch::default(), batch);
        Ok(OpenLoanRespResult {
            lpp: lpp_ref,
            downpayment: self.downpayment,
            principal: loan_resp.principal_due.into(),
            annual_interest_rate: loan_resp.annual_interest_rate,
        })
    }
}

pub struct OpenLoanRespResult {
    pub(in crate::contract) lpp: LppLenderRef,
    pub(in crate::contract) downpayment: CoinDTO,
    pub(in crate::contract) principal: CoinDTO,
    pub(in crate::contract) annual_interest_rate: Percent,
}
