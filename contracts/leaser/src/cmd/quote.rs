use std::marker::PhantomData;

use crate::msg::QuoteResponse;
use crate::ContractError;

use cosmwasm_std::StdResult;
use finance::coin::Coin;
use finance::liability::Liability;
use finance::percent::Percent;
use finance::{coin::CoinDTO, currency::Currency};

use lpp::msg::QueryQuoteResponse;
use lpp::stub::{Lpp as LppTrait, WithLpp};
use serde::Serialize;

use super::Quote;

impl WithLpp for Quote {
    type Output = QuoteResponse;
    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        let lpp_quote = LppQuote::new(lpp)?;

        let downpayment_lpn: Coin<Lpn> = self.downpayment.try_into()?;

        if downpayment_lpn.is_zero() {
            return Err(ContractError::ZeroDownpayment {});
        }

        let borrow = self.liability.init_borrow_amount(downpayment_lpn);
        let total = borrow + downpayment_lpn;

        let annual_interest_rate = lpp_quote.with(borrow)?;
        Ok(QuoteResponse {
            total: total.into(),
            borrow: borrow.into(),
            annual_interest_rate,
            annual_interest_rate_margin: self.lease_interest_rate_margin,
        })
    }

    fn unknown_lpn(
        self,
        symbol: finance::currency::SymbolOwned,
    ) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}

impl Quote {
    pub fn new(
        downpayment: CoinDTO,
        liability: Liability,
        lease_interest_rate_margin: Percent,
    ) -> StdResult<Quote> {
        Ok(Self {
            downpayment,
            liability,
            lease_interest_rate_margin,
        })
    }
}

pub struct LppQuote<'a, Lpn, Lpp> {
    lpn: PhantomData<&'a Lpn>,
    lpp: Lpp,
}

impl<'a, Lpn, Lpp> LppQuote<'a, Lpn, Lpp>
where
    Lpp: LppTrait<Lpn>,
    Lpn: Currency,
{
    pub fn new(lpp: Lpp) -> StdResult<LppQuote<'a, Lpn, Lpp>> {
        Ok(Self {
            lpn: PhantomData,
            lpp,
        })
    }

    pub fn with(&self, downpayment: Coin<Lpn>) -> Result<Percent, ContractError> {
        if downpayment.is_zero() {
            return Err(ContractError::ZeroDownpayment {});
        }

        let annual_interest_rate = match self.lpp.quote(downpayment)? {
            QueryQuoteResponse::QuoteInterestRate(rate) => rate,
            QueryQuoteResponse::NoLiquidity => return Err(ContractError::NoLiquidity {}),
        };

        Ok(annual_interest_rate)
    }
}
