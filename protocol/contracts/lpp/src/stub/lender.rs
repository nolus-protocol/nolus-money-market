use std::{marker::PhantomData, result::Result as StdResult};

use serde::de::DeserializeOwned;

use currency::Currency;
use finance::coin::Coin;
use platform::{
    batch::{Batch, ReplyId},
    reply::from_execute,
};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Reply};

use crate::{
    error::{ContractError, Result},
    msg::{ExecuteMsg, LoanResponse, QueryMsg, QueryQuoteResponse},
};

use super::{LppBatch, LppRef};

pub trait LppLender<Lpn>
where
    Self: Into<LppBatch<LppRef>>,
    Lpn: Currency,
{
    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<()>;
    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>>;

    fn quote(&self, amount: Coin<Lpn>) -> Result<QueryQuoteResponse>;
}

pub trait WithLppLender {
    type Output;
    type Error;

    fn exec<C, L>(self, lpp: L) -> StdResult<Self::Output, Self::Error>
    where
        L: LppLender<C>,
        C: Currency;
}

pub(super) struct LppLenderStub<'a, Lpn> {
    lpp_ref: LppRef,
    currency: PhantomData<Lpn>,
    querier: &'a QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, Lpn> LppLenderStub<'a, Lpn>
where
    Lpn: Currency,
{
    const OPEN_LOAN_REQ_ID: ReplyId = 0;

    pub(super) fn new(lpp_ref: LppRef, querier: &'a QuerierWrapper<'a>) -> Self {
        Self {
            lpp_ref,
            currency: PhantomData,
            querier,
            batch: Batch::default(),
        }
    }

    fn id(&self) -> Addr {
        self.lpp_ref.addr.clone()
    }
}

impl<'a, Lpn> LppLender<Lpn> for LppLenderStub<'a, Lpn>
where
    Lpn: Currency + DeserializeOwned,
{
    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<()> {
        self.batch
            .schedule_execute_wasm_on_success_reply::<_, Lpn>(
                &self.id(),
                ExecuteMsg::OpenLoan {
                    amount: amount.into(),
                },
                None,
                Self::OPEN_LOAN_REQ_ID,
            )
            .map_err(ContractError::from)
    }

    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>> {
        debug_assert_eq!(resp.id, Self::OPEN_LOAN_REQ_ID);

        from_execute(resp)
            .map_err(Into::into)
            .and_then(|maybe_data| maybe_data.ok_or(ContractError::NoResponseStubError))
    }

    fn quote(&self, amount: Coin<Lpn>) -> Result<QueryQuoteResponse> {
        let msg = QueryMsg::Quote {
            amount: amount.into(),
        };
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(ContractError::from)
    }
}

impl<'a, C> From<LppLenderStub<'a, C>> for LppBatch<LppRef> {
    fn from(stub: LppLenderStub<'a, C>) -> Self {
        Self {
            lpp_ref: stub.lpp_ref,
            batch: stub.batch,
        }
    }
}

#[cfg(test)]
mod test {
    use currency::{dex::test::StableC1, Currency};
    use finance::coin::Coin;
    use platform::response::{self};
    use sdk::{
        cosmwasm_ext::{CosmosMsg, Response as CwResponse},
        cosmwasm_std::{from_json, testing::MockQuerier, Addr, QuerierWrapper, ReplyOn, WasmMsg},
    };

    use crate::{
        msg::ExecuteMsg,
        stub::{lender::LppLender, LppBatch, LppRef},
    };
    #[test]
    fn open_loan_req() {
        let addr = Addr::unchecked("defd2r2");
        let lpp = LppRef {
            addr: addr.clone(),
            currency: ToOwned::to_owned(StableC1::TICKER),
        };
        let borrow_amount = Coin::<StableC1>::new(10);
        let querier = MockQuerier::default();
        let wrapper = QuerierWrapper::new(&querier);
        let mut lpp_stub = lpp.into_lender(&wrapper);
        lpp_stub
            .open_loan_req(borrow_amount)
            .expect("open new loan request failed");
        let LppBatch { lpp_ref: _, batch } = lpp_stub.into();
        let resp: CwResponse = response::response_only_messages(batch);
        assert_eq!(1, resp.messages.len());
        let msg = &resp.messages[0];
        assert_eq!(ReplyOn::Success, msg.reply_on);
        if let CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) = &msg.msg
        {
            assert_eq!(addr.as_str(), contract_addr);
            assert!(funds.is_empty());
            let lpp_msg: ExecuteMsg = from_json(msg).expect("invalid Lpp message");
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
