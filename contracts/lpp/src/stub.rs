use cosmwasm_std::{
    to_binary, Addr, Api, QuerierWrapper, Reply, StdResult, SubMsg, Timestamp, WasmMsg,
};
use finance::{coin::{Currency, Coin}, coin_legacy::to_cosmwasm};
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LppStub {
    addr: Addr,
}

impl LppStub {
    pub fn try_from<A>(addr_raw: String, api: &A) -> StdResult<Self>
    where
        A: ?Sized + Api,
    {
        let addr = api.addr_validate(&addr_raw)?;
        Ok(Self { addr })
    }
}

impl<C> Lpp<C> for LppStub
where
    C: Currency,
{
    fn open_loan_req(&self, amount: Coin<C>) -> StdResult<SubMsg> {
        let msg = to_binary(&ExecuteMsg::OpenLoan { amount: to_cosmwasm(amount) })?;
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
    use finance::{coin::{Nls, Coin}, coin_legacy::from_cosmwasm};

    use crate::{msg::ExecuteMsg, stub::REPLY_ID};

    use super::{Lpp, LppStub};

    #[test]
    fn open_loan_req() {
        let addr = Addr::unchecked("defd2r2");
        let lpp = LppStub { addr: addr.clone() };
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
