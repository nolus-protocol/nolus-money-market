use cosmwasm_std::{
    to_binary, Addr, Api, QuerierWrapper, Reply, StdResult, SubMsg, Timestamp, WasmMsg,
};
use finance::{
    coin::Coin,
    coin_legacy::to_cosmwasm,
    currency::{visit_any, AnyVisitor, Currency, SymbolOwned},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::msg::{
    BalanceResponse, ExecuteMsg, LppBalanceResponse, PriceResponse, QueryConfigResponse,
    QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryMsg, QueryQuoteResponse,
    RewardsResponse,
};

pub const REPLY_ID: u64 = 28;

pub trait Lpp<Lpn>
where
    Lpn: Currency,
{
    fn open_loan_req(&self, amount: Coin<Lpn>) -> StdResult<SubMsg>;
    fn open_loan_resp(&self, resp: Reply) -> Result<(), String>;
    fn repay_loan_req(&self, repayment: Coin<Lpn>) -> StdResult<SubMsg>;

    fn loan(
        &self,
        querier: &QuerierWrapper,
        lease: impl Into<Addr>,
    ) -> StdResult<QueryLoanResponse<Lpn>>;
    fn loan_outstanding_interest(
        &self,
        querier: &QuerierWrapper,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> StdResult<QueryLoanOutstandingInterestResponse<Lpn>>;
    fn quote(&self, querier: &QuerierWrapper, amount: Coin<Lpn>) -> StdResult<QueryQuoteResponse>;
    fn lpp_balance(&self, querier: &QuerierWrapper) -> StdResult<LppBalanceResponse<Lpn>>;
    fn nlpn_price(&self, querier: &QuerierWrapper) -> StdResult<PriceResponse>;
    fn config(&self, querier: &QuerierWrapper) -> StdResult<QueryConfigResponse>;
    fn nlpn_balance(
        &self,
        querier: &QuerierWrapper,
        lender: impl Into<Addr>,
    ) -> StdResult<BalanceResponse>;
    fn rewards(
        &self,
        querier: &QuerierWrapper,
        lender: impl Into<Addr>,
    ) -> StdResult<RewardsResponse>;
}

pub trait LppVisitor {
    type Output;
    type Error;

    fn on<C, L>(self, lpp: &L) -> Result<Self::Output, Self::Error>
    where
        L: Lpp<C>,
        C: Currency;

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LppStub {
    addr: Addr,
    currency: SymbolOwned,
}

impl LppStub {
    pub fn try_from<A>(addr_raw: String, api: &A, querier: &QuerierWrapper) -> StdResult<Self>
    where
        A: ?Sized + Api,
    {
        let addr = api.addr_validate(&addr_raw)?;
        let resp: QueryConfigResponse =
            querier.query_wasm_smart(addr.clone(), &QueryMsg::Config())?;
        let currency = resp.lpn_symbol;
        Ok(Self { addr, currency })
    }

    pub fn execute<V, O, E>(&self, v: V) -> Result<O, E>
    where
        V: LppVisitor<Output = O, Error = E>,
    {
        struct CurrencyVisitor<'a, V, O, E>(V, &'a LppStub)
        where
            V: LppVisitor<Output = O, Error = E>;

        impl<'a, V, O, E> AnyVisitor for CurrencyVisitor<'a, V, O, E>
        where
            V: LppVisitor<Output = O, Error = E>,
        {
            type Output = O;

            type Error = E;

            fn on<C>(self) -> Result<Self::Output, Self::Error>
            where
                C: Currency + DeserializeOwned,
            {
                self.0.on::<C, LppStub>(self.1)
            }

            fn on_unknown(self) -> Result<Self::Output, Self::Error> {
                self.0.unknown_lpn(self.1.currency.clone())
            }
        }
        visit_any(&self.currency, CurrencyVisitor(v, self))
    }
}

impl<Lpn> Lpp<Lpn> for LppStub
where
    Lpn: Currency + DeserializeOwned,
{
    fn open_loan_req(&self, amount: Coin<Lpn>) -> StdResult<SubMsg> {
        let msg = to_binary(&ExecuteMsg::OpenLoan {
            amount: to_cosmwasm(amount),
        })?;
        Ok(SubMsg::reply_on_success(
            WasmMsg::Execute {
                contract_addr: self.addr.as_ref().into(),
                funds: vec![],
                msg,
            },
            REPLY_ID,
        ))
    }

    fn open_loan_resp(&self, resp: Reply) -> Result<(), String> {
        debug_assert_eq!(REPLY_ID, resp.id);
        resp.result.into_result().map(|_| ())
    }

    fn repay_loan_req(&self, repayment: Coin<Lpn>) -> StdResult<SubMsg> {
        let msg = to_binary(&ExecuteMsg::RepayLoan {})?;
        Ok(SubMsg::new(WasmMsg::Execute {
            contract_addr: self.addr.as_ref().into(),
            funds: vec![to_cosmwasm(repayment)],
            msg,
        }))
    }

    fn loan(
        &self,
        querier: &QuerierWrapper,
        lease: impl Into<Addr>,
    ) -> StdResult<QueryLoanResponse<Lpn>> {
        let msg = QueryMsg::Loan {
            lease_addr: lease.into(),
        };
        querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn loan_outstanding_interest(
        &self,
        querier: &QuerierWrapper,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> StdResult<QueryLoanOutstandingInterestResponse<Lpn>> {
        let msg = QueryMsg::LoanOutstandingInterest {
            lease_addr: lease.into(),
            outstanding_time: by,
        };
        querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn quote(&self, querier: &QuerierWrapper, amount: Coin<Lpn>) -> StdResult<QueryQuoteResponse> {
        let msg = QueryMsg::Quote {
            amount: to_cosmwasm(amount),
        };
        querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn lpp_balance(&self, querier: &QuerierWrapper) -> StdResult<LppBalanceResponse<Lpn>> {
        let msg = QueryMsg::LppBalance();
        querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn nlpn_price(&self, querier: &QuerierWrapper) -> StdResult<PriceResponse> {
        let msg = QueryMsg::Price();
        querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn config(&self, querier: &QuerierWrapper) -> StdResult<QueryConfigResponse> {
        let msg = QueryMsg::Config();
        querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn nlpn_balance(
        &self,
        querier: &QuerierWrapper,
        lender: impl Into<Addr>,
    ) -> StdResult<BalanceResponse> {
        let msg = QueryMsg::Balance {
            address: lender.into(),
        };
        querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn rewards(
        &self,
        querier: &QuerierWrapper,
        lender: impl Into<Addr>,
    ) -> StdResult<RewardsResponse> {
        let msg = QueryMsg::Rewards {
            address: lender.into(),
        };
        querier.query_wasm_smart(self.addr.clone(), &msg)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{from_binary, Addr, CosmosMsg, ReplyOn, WasmMsg};
    use finance::{
        coin::Coin,
        coin_legacy::from_cosmwasm,
        currency::{Currency, Nls},
    };

    use crate::{msg::ExecuteMsg, stub::REPLY_ID};

    use super::{Lpp, LppStub};

    #[test]
    fn open_loan_req() {
        let addr = Addr::unchecked("defd2r2");
        let lpp = LppStub {
            addr: addr.clone(),
            currency: Nls::SYMBOL.to_owned(),
        };
        let borrow_amount = Coin::<Nls>::new(10);
        let msg = lpp
            .open_loan_req(borrow_amount)
            .expect("open new loan request failed");
        assert_eq!(REPLY_ID, msg.id);
        assert_eq!(ReplyOn::Success, msg.reply_on);
        if let CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) = msg.msg
        {
            assert_eq!(addr, contract_addr);
            assert!(funds.is_empty());
            let lpp_msg: ExecuteMsg = from_binary(&msg).expect("invalid Lpp message");
            if let ExecuteMsg::OpenLoan { amount } = lpp_msg {
                assert_eq!(borrow_amount, from_cosmwasm(amount).unwrap());
            } else {
                panic!("Bad Lpp message type!");
            }
        } else {
            panic!("Bad Cosmos message!");
        }
    }
}
