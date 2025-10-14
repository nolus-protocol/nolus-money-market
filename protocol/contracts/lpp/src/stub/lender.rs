use std::marker::PhantomData;

use thiserror::Error;

use currency::CurrencyDef;
use finance::coin::Coin;
use platform::{
    batch::{Batch, ReplyId},
    reply,
};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Reply, StdError};

use crate::msg::{ExecuteMsg, LoanResponse, QueryMsg, QueryQuoteResponse};

use super::{LppBatch, LppRef};

pub trait LppLender<Lpn>
where
    Self: Into<LppBatch<LppRef<Lpn>>>,
{
    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<(), Error>;
    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>, Error>;

    fn quote(&self, amount: Coin<Lpn>) -> Result<QueryQuoteResponse, Error>;
}

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Lpp][Lender] [Std] {0}")]
    Std(StdError),

    #[error("[Lpp][Lender] {0}")]
    Platform(platform::error::Error),

    #[error("[Lpp][Lender] No response sent back from LPP contract")]
    NoResponseStubError,

    #[error("[Lpp][Lender] The loan does not exist")]
    NoLoan {},
}

pub trait WithLppLender<Lpn> {
    type Output;
    type Error;

    fn exec<Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppLender<Lpn>;
}

pub(super) struct LppLenderStub<'a, Lpn> {
    lpp_ref: LppRef<Lpn>,
    lpn: PhantomData<Lpn>,
    querier: QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, Lpn> LppLenderStub<'a, Lpn> {
    const OPEN_LOAN_REQ_ID: ReplyId = 0;

    pub(super) fn new(lpp_ref: LppRef<Lpn>, querier: QuerierWrapper<'a>) -> Self {
        Self {
            lpp_ref,
            lpn: PhantomData,
            querier,
            batch: Batch::default(),
        }
    }

    fn id(&self) -> Addr {
        self.lpp_ref.addr.clone()
    }
}

impl<Lpn> LppLender<Lpn> for LppLenderStub<'_, Lpn>
where
    Lpn: CurrencyDef,
{
    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<(), Error> {
        self.batch
            .schedule_execute_wasm_reply_on_success_no_funds(
                self.id().clone(),
                &ExecuteMsg::<Lpn::Group>::OpenLoan {
                    amount: amount.into(),
                },
                Self::OPEN_LOAN_REQ_ID,
            )
            .map_err(Error::Platform)
    }

    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>, Error> {
        debug_assert_eq!(resp.id, Self::OPEN_LOAN_REQ_ID);

        reply::from_execute(resp)
            .map_err(Error::Platform)
            .and_then(|maybe_data| maybe_data.ok_or(Error::NoResponseStubError))
    }

    fn quote(&self, amount: Coin<Lpn>) -> Result<QueryQuoteResponse, Error> {
        let msg = QueryMsg::<Lpn::Group>::Quote {
            amount: amount.into(),
        };
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(Error::Std)
    }
}

impl<Lpn> From<LppLenderStub<'_, Lpn>> for LppBatch<LppRef<Lpn>> {
    fn from(stub: LppLenderStub<'_, Lpn>) -> Self {
        Self {
            lpp_ref: stub.lpp_ref,
            batch: stub.batch,
        }
    }
}

#[cfg(test)]
mod test {
    use std::marker::PhantomData;

    use currencies::{Lpn, Lpns};
    use finance::coin::Coin;
    use platform::response::{self};
    use sdk::{
        cosmwasm_ext::{CosmosMsg, Response as CwResponse},
        cosmwasm_std::{self, Addr, QuerierWrapper, ReplyOn, WasmMsg, testing::MockQuerier},
    };

    use crate::{
        msg::ExecuteMsg,
        stub::{LppBatch, LppRef, lender::LppLender},
    };
    #[test]
    fn open_loan_req() {
        let addr = Addr::unchecked("defd2r2");
        let lpp = LppRef {
            addr: addr.clone(),
            _lpn: PhantomData::<Lpn>,
        };
        let borrow_amount = Coin::<Lpn>::new(10);
        let querier = MockQuerier::default();
        let wrapper = QuerierWrapper::new(&querier);
        let mut lpp_stub = lpp.into_lender(wrapper);
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
            let lpp_msg: ExecuteMsg<Lpns> =
                cosmwasm_std::from_json(msg).expect("invalid Lpp message");
            if let ExecuteMsg::<Lpns>::OpenLoan { amount } = lpp_msg {
                assert_eq!(borrow_amount, amount.try_into().unwrap());
            } else {
                panic!("Bad Lpp message type!");
            }
        } else {
            panic!("Bad Cosmos message!");
        }
    }
}
