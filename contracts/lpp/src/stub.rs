use std::marker::PhantomData;

use cosmwasm_std::{
    to_binary, Addr, Api, QuerierWrapper, Reply, StdError, StdResult, SubMsg, Timestamp, WasmMsg,
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

const REPLY_ID: u64 = 28;

// TODO split into LppBorrow, LppLend, and LppAdmin traits
pub trait Lpp<Lpn>
where
    Lpn: Currency,
{
    fn open_loan_req(&self, amount: Coin<Lpn>) -> StdResult<SubMsg>;
    fn open_loan_resp(&self, resp: Reply) -> StdResult<()>;
    fn repay_loan_req(&self, repayment: Coin<Lpn>) -> StdResult<SubMsg>;

    fn loan(&self, lease: impl Into<Addr>) -> StdResult<QueryLoanResponse<Lpn>>;

    fn loan_outstanding_interest(
        &self,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> StdResult<QueryLoanOutstandingInterestResponse<Lpn>>;
    fn quote(&self, amount: Coin<Lpn>) -> StdResult<QueryQuoteResponse>;
    fn lpp_balance(&self) -> StdResult<LppBalanceResponse<Lpn>>;
    fn nlpn_price(&self) -> StdResult<PriceResponse<Lpn>>;
    fn config(&self) -> StdResult<QueryConfigResponse>;
    fn nlpn_balance(&self, lender: impl Into<Addr>) -> StdResult<BalanceResponse>;
    fn rewards(&self, lender: impl Into<Addr>) -> StdResult<RewardsResponse>;
}

pub trait WithLpp {
    type Output;
    type Error;

    fn exec<C, L>(self, lpp: L) -> Result<Self::Output, Self::Error>
    where
        L: Lpp<C>,
        C: Currency + Serialize;

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LppRef {
    addr: Addr,
    currency: SymbolOwned,
}

impl LppRef {
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

    pub fn execute<V, O, E>(&self, cmd: V, querier: &QuerierWrapper) -> Result<O, E>
    where
        V: WithLpp<Output = O, Error = E>,
    {
        struct CurrencyVisitor<'a, V, O, E>
        where
            V: WithLpp<Output = O, Error = E>,
        {
            cmd: V,
            lpp_ref: &'a LppRef,
            querier: &'a QuerierWrapper<'a>,
        }

        impl<'a, V, O, E> AnyVisitor for CurrencyVisitor<'a, V, O, E>
        where
            V: WithLpp<Output = O, Error = E>,
        {
            type Output = O;
            type Error = E;

            fn on<C>(self) -> Result<Self::Output, Self::Error>
            where
                C: Currency + Serialize + DeserializeOwned,
            {
                self.cmd.exec(self.lpp_ref.as_stub::<C>(self.querier))
            }

            fn on_unknown(self) -> Result<Self::Output, Self::Error> {
                self.cmd.unknown_lpn(self.lpp_ref.currency.clone())
            }
        }
        visit_any(
            &self.currency,
            CurrencyVisitor {
                cmd,
                lpp_ref: self,
                querier,
            },
        )
    }

    fn as_stub<'a, C>(&'a self, querier: &'a QuerierWrapper) -> LppStub<'a, C> {
        LppStub {
            addr: self.addr.clone(),
            currency: PhantomData::<C>,
            querier,
        }
    }
}

#[cfg(feature = "testing")]
impl LppRef {
    pub fn unchecked<A, Lpn>(addr: A) -> Self
    where
        A: Into<String>,
        Lpn: Currency,
    {
        Self {
            addr: Addr::unchecked(addr),
            currency: Lpn::SYMBOL.into(),
        }
    }
}

struct LppStub<'a, C> {
    addr: Addr,
    currency: PhantomData<C>,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a, Lpn> Lpp<Lpn> for LppStub<'a, Lpn>
where
    Lpn: Currency + DeserializeOwned,
{
    fn open_loan_req(&self, amount: Coin<Lpn>) -> StdResult<SubMsg> {
        let msg = to_binary(&ExecuteMsg::OpenLoan {
            amount: amount.into(),
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

    fn open_loan_resp(&self, resp: Reply) -> StdResult<()> {
        debug_assert_eq!(REPLY_ID, resp.id);
        resp.result
            .into_result()
            .map(|_| ())
            .map_err(StdError::generic_err)
    }

    fn repay_loan_req(&self, repayment: Coin<Lpn>) -> StdResult<SubMsg> {
        let msg = to_binary(&ExecuteMsg::RepayLoan {})?;
        Ok(SubMsg::new(WasmMsg::Execute {
            contract_addr: self.addr.as_ref().into(),
            funds: vec![to_cosmwasm(repayment)],
            msg,
        }))
    }

    fn loan(&self, lease: impl Into<Addr>) -> StdResult<QueryLoanResponse<Lpn>> {
        let msg = QueryMsg::Loan {
            lease_addr: lease.into(),
        };
        self.querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn loan_outstanding_interest(
        &self,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> StdResult<QueryLoanOutstandingInterestResponse<Lpn>> {
        let msg = QueryMsg::LoanOutstandingInterest {
            lease_addr: lease.into(),
            outstanding_time: by,
        };
        self.querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn quote(&self, amount: Coin<Lpn>) -> StdResult<QueryQuoteResponse> {
        let msg = QueryMsg::Quote {
            amount: amount.into(),
        };
        self.querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn lpp_balance(&self) -> StdResult<LppBalanceResponse<Lpn>> {
        let msg = QueryMsg::LppBalance();
        self.querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn nlpn_price(&self) -> StdResult<PriceResponse<Lpn>> {
        let msg = QueryMsg::Price();
        self.querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn config(&self) -> StdResult<QueryConfigResponse> {
        let msg = QueryMsg::Config();
        self.querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn nlpn_balance(&self, lender: impl Into<Addr>) -> StdResult<BalanceResponse> {
        let msg = QueryMsg::Balance {
            address: lender.into(),
        };
        self.querier.query_wasm_smart(self.addr.clone(), &msg)
    }

    fn rewards(&self, lender: impl Into<Addr>) -> StdResult<RewardsResponse> {
        let msg = QueryMsg::Rewards {
            address: lender.into(),
        };
        self.querier.query_wasm_smart(self.addr.clone(), &msg)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        from_binary, testing::MockQuerier, Addr, CosmosMsg, QuerierWrapper, ReplyOn, WasmMsg,
    };
    use finance::{
        coin::Coin,
        currency::{Currency, Nls},
    };

    use crate::{
        msg::ExecuteMsg,
        stub::{LppRef, REPLY_ID},
    };

    use super::Lpp;

    #[test]
    fn open_loan_req() {
        let addr = Addr::unchecked("defd2r2");
        let lpp = LppRef {
            addr: addr.clone(),
            currency: Nls::SYMBOL.to_owned(),
        };
        let borrow_amount = Coin::<Nls>::new(10);
        let msg = lpp
            .as_stub(&QuerierWrapper::new(&MockQuerier::default()))
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
                assert_eq!(borrow_amount, amount.try_into().unwrap());
            } else {
                panic!("Bad Lpp message type!");
            }
        } else {
            panic!("Bad Cosmos message!");
        }
    }
}
