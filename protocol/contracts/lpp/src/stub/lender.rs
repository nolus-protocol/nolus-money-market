use std::{marker::PhantomData, result::Result as StdResult};

use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::Coin;
use platform::{
    batch::{Batch, ReplyId},
    reply,
};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Reply};

use crate::{
    error::Error,
    msg::{ExecuteMsg, LoanResponse, QueryMsg, QueryQuoteResponse},
};

use super::{LppBatch, LppRef};

pub trait LppLender<Lpn, Lpns>
where
    Lpns: Group,
    Self: Into<LppBatch<LppRef<Lpn, Lpns>>>,
{
    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<(), Error>;
    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>, Error>;

    fn quote(&self, amount: Coin<Lpn>) -> Result<QueryQuoteResponse, Error>;
}

pub trait WithLppLender<Lpn, Lpns>
where
    Lpns: Group,
{
    type Output;
    type Error;

    fn exec<Lpp>(self, lpp: Lpp) -> StdResult<Self::Output, Self::Error>
    where
        Lpp: LppLender<Lpn, Lpns>;
}

pub(super) struct LppLenderStub<'a, Lpn, Lpns> {
    lpp_ref: LppRef<Lpn, Lpns>,
    lpn: PhantomData<Lpn>,
    querier: QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, Lpn, Lpns> LppLenderStub<'a, Lpn, Lpns>
where
    Lpns: Group,
{
    const OPEN_LOAN_REQ_ID: ReplyId = 0;

    pub(super) fn new(lpp_ref: LppRef<Lpn, Lpns>, querier: QuerierWrapper<'a>) -> Self {
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

impl<Lpn, Lpns> LppLender<Lpn, Lpns> for LppLenderStub<'_, Lpn, Lpns>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
    Lpns: Group,
{
    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<(), Error> {
        self.batch
            .schedule_execute_wasm_reply_on_success_no_funds(
                self.id().clone(),
                &ExecuteMsg::<Lpns>::OpenLoan {
                    amount: amount.into(),
                },
                Self::OPEN_LOAN_REQ_ID,
            )
            .map_err(Error::from)
    }

    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>, Error> {
        debug_assert_eq!(resp.id, Self::OPEN_LOAN_REQ_ID);

        reply::from_execute(resp)
            .map_err(Into::into)
            .and_then(|maybe_data| maybe_data.ok_or(Error::NoResponseStubError))
    }

    fn quote(&self, amount: Coin<Lpn>) -> Result<QueryQuoteResponse, Error> {
        let msg = QueryMsg::<Lpns>::Quote {
            amount: amount.into(),
        };
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(Error::from)
    }
}

impl<Lpn, Lpns> From<LppLenderStub<'_, Lpn, Lpns>> for LppBatch<LppRef<Lpn, Lpns>> {
    fn from(stub: LppLenderStub<'_, Lpn, Lpns>) -> Self {
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
        cosmwasm_std::{Addr, QuerierWrapper, ReplyOn, WasmMsg, from_json, testing::MockQuerier},
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
            _lpns: PhantomData::<Lpns>,
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
            let lpp_msg: ExecuteMsg<Lpns> = from_json(msg).expect("invalid Lpp message");
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
