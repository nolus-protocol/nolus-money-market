use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::payment::PaymentGroup;
use finance::{
    coin::{Coin, WithCoin},
    currency::Currency,
    percent::Percent,
};
use lpp::stub::lender::{LppLender as LppLenderTrait, WithLppLender};
use market_price_oracle::{convert, stub::OracleRef};
use platform::{bank, batch::Batch};
use sdk::cosmwasm_std::{Coin as CwCoin, QuerierWrapper, Reply};

use crate::{
    api::{DownpaymentCoin, NewLeaseForm},
    error::ContractError,
};

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
impl<'a, Lpn> WithCoin for DownpaymentHandler<'a, Lpn>
where
    Lpn: Currency,
{
    type Output = (DownpaymentCoin, Coin<Lpn>);

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
    pub(in crate::contract) downpayment: DownpaymentCoin,
}

pub struct OpenLoanResp {
    reply: Reply,
}

impl OpenLoanResp {
    pub fn new(reply: Reply) -> Self {
        Self { reply }
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

        #[cfg(debug_assertions)]
        {
            use lpp::stub::LppBatch;

            let LppBatch { lpp_ref: _, batch } = lpp.into();
            debug_assert_eq!(Batch::default(), batch);
        }
        Ok(OpenLoanRespResult {
            principal: loan_resp.principal_due.into(),
            annual_interest_rate: loan_resp.annual_interest_rate,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct OpenLoanRespResult {
    pub(in crate::contract) principal: DownpaymentCoin,
    pub(in crate::contract) annual_interest_rate: Percent,
}
