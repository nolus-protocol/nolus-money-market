use cosmwasm_std::{
    to_binary, Addr, Api, QuerierWrapper, Reply, StdResult, SubMsg, Timestamp, WasmMsg,
};
use finance::{
    coin::Coin,
    coin_legacy::to_cosmwasm,
    currency::{visit_any, AnyVisitor, Currency, SymbolOwned, Usdc},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::msg::{ExecuteMsg, QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryMsg};

pub const REPLY_ID: u64 = 28;

pub trait Lpp<C>: Serialize + DeserializeOwned {
    fn open_loan_req(&self, amount: Coin<C>) -> StdResult<SubMsg>;
    fn open_loan_resp(&self, resp: Reply) -> Result<(), String>;
    fn repay_loan_req(&self, repayment: Coin<C>) -> StdResult<SubMsg>;

    fn loan(
        &self,
        querier: &QuerierWrapper,
        lease: impl Into<Addr>,
    ) -> StdResult<QueryLoanResponse>;
    fn loan_outstanding_interest(
        &self,
        querier: &QuerierWrapper,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> StdResult<QueryLoanOutstandingInterestResponse>;
}

pub trait LppVisitor {
    type Output;
    type Error;

    fn on<C, L>(self, lpp: &L) -> Result<Self::Output, Self::Error>
    where
        L: Lpp<C>,
        C: Currency,;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LppStub {
    addr: Addr,
    currency: SymbolOwned,
}

impl LppStub {
    pub fn try_from<A>(addr_raw: String, api: &A, _querier: &QuerierWrapper) -> StdResult<Self>
    where
        A: ?Sized + Api,
    {
        let addr = api.addr_validate(&addr_raw)?;
        // let resp : QueryConfigResponse = querier.query_wasm_smart(addr.clone(), &QueryMsg::QueryConfig())?;
        // let currency = resp.lpn;
        let currency = Usdc::SYMBOL.to_owned();
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
                C: Currency,
            {
                self.0.on::<C, LppStub>(self.1)
            }

            fn on_unknown(self) -> Result<Self::Output, Self::Error> {
                unreachable!("The LPN is unknown for the LPP stub!")
            }
        }
        visit_any(&self.currency, CurrencyVisitor(v, self))
    }
}

impl<C> Lpp<C> for LppStub
where
    C: Currency,
{
    fn open_loan_req(&self, amount: Coin<C>) -> StdResult<SubMsg> {
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

    fn repay_loan_req(&self, repayment: Coin<C>) -> StdResult<SubMsg> {
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
    ) -> StdResult<QueryLoanResponse> {
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
    ) -> StdResult<QueryLoanOutstandingInterestResponse> {
        let msg = QueryMsg::LoanOutstandingInterest {
            lease_addr: lease.into(),
            outstanding_time: by,
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
