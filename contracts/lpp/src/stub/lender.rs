use std::{marker::PhantomData, result::Result as StdResult};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::lpn::Lpns;
use finance::{
    coin::Coin,
    currency::{visit_any_on_ticker, AnyVisitor, AnyVisitorResult, Currency, Symbol, SymbolOwned},
};
use platform::{
    batch::{Batch, ReplyId},
    reply::from_execute,
};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Reply};

use crate::{
    error::{ContractError, Result},
    msg::{ExecuteMsg, LoanResponse, QueryLoanResponse, QueryMsg, QueryQuoteResponse},
    state::Config,
    stub::LppBatch,
};

// pub struct LppLoan<'a, Lpn>
// where
//     Lpn: Currency,
// {
//     loan: Loan<Lpn>,
//     stub: LppLenderStub<'a, Lpn>,
// }

pub trait LppLender<Lpn>
where
    Self: Into<LppBatch<LppLenderRef>>,
    Lpn: Currency,
{
    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<()>;
    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>>;
    fn repay_loan_req(&mut self, repayment: Coin<Lpn>) -> Result<()>;

    fn loan(&self, lease: impl Into<Addr>) -> Result<QueryLoanResponse<Lpn>>;

    fn quote(&self, amount: Coin<Lpn>) -> Result<QueryQuoteResponse>;
}

pub trait WithLppLender {
    type Output;
    type Error;

    fn exec<C, L>(self, lpp: L) -> StdResult<Self::Output, Self::Error>
    where
        L: LppLender<C>,
        C: Currency + Serialize;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LppLenderRef {
    addr: Addr,
    currency: SymbolOwned,
    open_loan_req_id: ReplyId,
}

impl LppLenderRef {
    pub fn try_new(
        addr: Addr,
        querier: &QuerierWrapper<'_>,
        open_loan_req_id: ReplyId,
    ) -> Result<Self> {
        let resp: Config = querier.query_wasm_smart(addr.clone(), &QueryMsg::Config())?;

        let currency = resp.lpn_ticker().into();

        Ok(Self {
            addr,
            currency,
            open_loan_req_id,
        })
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn currency(&self) -> Symbol<'_> {
        &self.currency
    }

    pub fn execute<Cmd>(
        self,
        cmd: Cmd,
        querier: &QuerierWrapper<'_>,
    ) -> StdResult<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLppLender,
        finance::error::Error: Into<Cmd::Error>,
    {
        struct CurrencyVisitor<'a, Cmd> {
            cmd: Cmd,
            lpp_ref: LppLenderRef,
            querier: &'a QuerierWrapper<'a>,
        }

        impl<'a, Cmd> AnyVisitor for CurrencyVisitor<'a, Cmd>
        where
            Cmd: WithLppLender,
        {
            type Output = Cmd::Output;
            type Error = Cmd::Error;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency + Serialize + DeserializeOwned,
            {
                self.cmd.exec(self.lpp_ref.into_stub::<C>(self.querier))
            }
        }

        visit_any_on_ticker::<Lpns, _>(
            &self.currency.clone(),
            CurrencyVisitor {
                cmd,
                lpp_ref: self,
                querier,
            },
        )
    }

    fn into_stub<'a, C>(self, querier: &'a QuerierWrapper<'a>) -> LppLenderStub<'a, C>
    where
        C: Currency,
    {
        LppLenderStub {
            lpp_ref: self,
            currency: PhantomData::<C>,
            querier,
            batch: Batch::default(),
        }
    }
}

#[cfg(feature = "testing")]
impl LppLenderRef {
    pub fn unchecked<A, Lpn>(addr: A, open_loan_req_id: ReplyId) -> Self
    where
        A: Into<String>,
        Lpn: Currency,
    {
        Self {
            addr: Addr::unchecked(addr),
            currency: Lpn::TICKER.into(),
            open_loan_req_id,
        }
    }
}

struct LppLenderStub<'a, Lpn> {
    lpp_ref: LppLenderRef,
    currency: PhantomData<Lpn>,
    querier: &'a QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, Lpn> LppLenderStub<'a, Lpn> {
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
                self.lpp_ref.open_loan_req_id,
            )
            .map_err(ContractError::from)
    }

    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>> {
        debug_assert_eq!(resp.id, self.lpp_ref.open_loan_req_id);

        from_execute(resp)
            .map_err(Into::into)
            .and_then(|maybe_data| {
                maybe_data.ok_or_else(|| ContractError::CustomError {
                    val: "No data passed as response!".into(),
                })
            })
    }

    fn repay_loan_req(&mut self, repayment: Coin<Lpn>) -> Result<()> {
        self.batch
            .schedule_execute_wasm_no_reply(&self.id(), ExecuteMsg::RepayLoan(), Some(repayment))
            .map_err(ContractError::from)
    }

    fn loan(&self, lease: impl Into<Addr>) -> Result<QueryLoanResponse<Lpn>> {
        let msg = QueryMsg::Loan {
            lease_addr: lease.into(),
        };
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(ContractError::from)
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

impl<'a, C> From<LppLenderStub<'a, C>> for LppBatch<LppLenderRef> {
    fn from(stub: LppLenderStub<'a, C>) -> Self {
        Self {
            lpp_ref: stub.lpp_ref,
            batch: stub.batch,
        }
    }
}

#[cfg(test)]
mod test {
    use finance::{coin::Coin, currency::Currency, test::currency::Nls};
    use platform::response::{self};
    use sdk::{
        cosmwasm_ext::{CosmosMsg, Response as CwResponse},
        cosmwasm_std::{from_binary, testing::MockQuerier, Addr, QuerierWrapper, ReplyOn, WasmMsg},
    };

    use crate::{msg::ExecuteMsg, stub::LppBatch};

    use super::{LppLender, LppLenderRef};

    #[test]
    fn open_loan_req() {
        // Magic number to test correctly against default zero.
        const OPEN_LOAN_REQ_ID: u64 = 0xC0FFEE;

        let addr = Addr::unchecked("defd2r2");
        let lpp = LppLenderRef {
            addr: addr.clone(),
            currency: ToOwned::to_owned(Nls::TICKER),
            open_loan_req_id: OPEN_LOAN_REQ_ID,
        };
        let borrow_amount = Coin::<Nls>::new(10);
        let querier = MockQuerier::default();
        let wrapper = QuerierWrapper::new(&querier);
        let mut lpp_stub = lpp.into_stub(&wrapper);
        lpp_stub
            .open_loan_req(borrow_amount)
            .expect("open new loan request failed");
        let LppBatch { lpp_ref: _, batch } = lpp_stub.into();
        let resp: CwResponse = response::response_only_messages(batch);
        assert_eq!(1, resp.messages.len());
        let msg = &resp.messages[0];
        assert_eq!(msg.id, OPEN_LOAN_REQ_ID);
        assert_eq!(ReplyOn::Success, msg.reply_on);
        if let CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) = &msg.msg
        {
            assert_eq!(addr.as_str(), contract_addr);
            assert!(funds.is_empty());
            let lpp_msg: ExecuteMsg = from_binary(msg).expect("invalid Lpp message");
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
