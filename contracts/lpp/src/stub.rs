use std::{marker::PhantomData, result::Result as StdResult};

use cosmwasm_std::{Addr, QuerierWrapper, Reply, Timestamp};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use finance::{
    coin::Coin,
    currency::{visit_any, AnyVisitor, Currency, SymbolOwned},
};
use platform::{
    batch::{Batch, ReplyId},
    reply::from_execute,
};

use crate::{
    error::ContractError,
    msg::{
        BalanceResponse, ExecuteMsg, LoanResponse, LppBalanceResponse, PriceResponse,
        QueryConfigResponse, QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryMsg,
        QueryQuoteResponse, RewardsResponse,
    },
};

pub type Result<T> = StdResult<T, ContractError>;

// TODO split into LppBorrow, LppLend, and LppAdmin traits
pub trait Lpp<Lpn>
where
    Self: Into<LppBatch>,
    Lpn: Currency,
{
    fn id(&self) -> Addr;

    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<()>;
    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>>;
    fn repay_loan_req(&mut self, repayment: Coin<Lpn>, liquidation: bool) -> Result<()>;

    fn loan(&self, lease: impl Into<Addr>) -> Result<QueryLoanResponse<Lpn>>;

    fn loan_outstanding_interest(
        &self,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> Result<QueryLoanOutstandingInterestResponse<Lpn>>;
    fn quote(&self, amount: Coin<Lpn>) -> Result<QueryQuoteResponse>;
    fn lpp_balance(&self) -> Result<LppBalanceResponse<Lpn>>;
    fn nlpn_price(&self) -> Result<PriceResponse<Lpn>>;
    fn config(&self) -> Result<QueryConfigResponse>;
    fn nlpn_balance(&self, lender: impl Into<Addr>) -> Result<BalanceResponse>;
    fn rewards(&self, lender: impl Into<Addr>) -> Result<RewardsResponse>;
}

pub trait WithLpp {
    type Output;
    type Error;

    fn exec<C, L>(self, lpp: L) -> StdResult<Self::Output, Self::Error>
    where
        L: Lpp<C>,
        C: Currency + Serialize;

    fn unknown_lpn(self, symbol: SymbolOwned) -> StdResult<Self::Output, Self::Error>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LppRef {
    addr: Addr,
    currency: SymbolOwned,
    open_loan_req_id: Option<ReplyId>,
    repay_req_id: Option<ReplyId>,
    liquidation_repay_id: Option<ReplyId>,
}

impl LppRef {
    pub fn try_from(addr: Addr, querier: &QuerierWrapper) -> Result<Self> {
        Self::try_from_maybe_borrow(addr, querier, None, None, None)
    }

    pub fn try_borrow_from(
        addr: Addr,
        querier: &QuerierWrapper,
        open_loan_req_id: ReplyId,
        repay_req_id: ReplyId,
        liquidation_repay_id: ReplyId,
    ) -> Result<Self> {
        Self::try_from_maybe_borrow(
            addr,
            querier,
            Some(open_loan_req_id),
            Some(repay_req_id),
            Some(liquidation_repay_id),
        )
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn execute<V, O, E>(self, cmd: V, querier: &QuerierWrapper) -> StdResult<O, E>
    where
        V: WithLpp<Output = O, Error = E>,
    {
        struct CurrencyVisitor<'a, V, O, E>
        where
            V: WithLpp<Output = O, Error = E>,
        {
            cmd: V,
            lpp_ref: LppRef,
            querier: &'a QuerierWrapper<'a>,
        }

        impl<'a, V, O, E> AnyVisitor for CurrencyVisitor<'a, V, O, E>
        where
            V: WithLpp<Output = O, Error = E>,
        {
            type Output = O;
            type Error = E;

            fn on<C>(self) -> StdResult<Self::Output, Self::Error>
            where
                C: Currency + Serialize + DeserializeOwned,
            {
                self.cmd.exec(self.lpp_ref.into_stub::<C>(self.querier))
            }

            fn on_unknown(self) -> StdResult<Self::Output, Self::Error> {
                self.cmd.unknown_lpn(self.lpp_ref.currency)
            }
        }

        visit_any(
            &self.currency.clone(),
            CurrencyVisitor {
                cmd,
                lpp_ref: self,
                querier,
            },
        )
    }

    fn try_from_maybe_borrow(
        addr: Addr,
        querier: &QuerierWrapper,
        open_loan_req_id: Option<ReplyId>,
        repay_req_id: Option<ReplyId>,
        liquidation_repay_id: Option<ReplyId>,
    ) -> Result<Self> {
        let resp: QueryConfigResponse =
            querier.query_wasm_smart(addr.clone(), &QueryMsg::Config())?;

        let currency = resp.lpn_symbol;

        Ok(Self {
            addr,
            currency,
            open_loan_req_id,
            repay_req_id,
            liquidation_repay_id,
        })
    }

    fn into_stub<'a, C>(self, querier: &'a QuerierWrapper) -> LppStub<'a, C> {
        LppStub {
            lpp_ref: self,
            currency: PhantomData::<C>,
            querier,
            batch: Batch::default(),
        }
    }
}

#[cfg(feature = "testing")]
impl LppRef {
    pub fn unchecked<A, Lpn>(
        addr: A,
        open_loan_req_id: Option<ReplyId>,
        repay_req_id: Option<ReplyId>,
        liquidation_repay_id: Option<ReplyId>,
    ) -> Self
    where
        A: Into<String>,
        Lpn: Currency,
    {
        Self {
            addr: Addr::unchecked(addr),
            currency: Lpn::SYMBOL.into(),
            open_loan_req_id,
            repay_req_id,
            liquidation_repay_id,
        }
    }
}

struct LppStub<'a, C> {
    lpp_ref: LppRef,
    currency: PhantomData<C>,
    querier: &'a QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, Lpn> LppStub<'a, Lpn> {
    fn open_loan_req_id(&self) -> ReplyId {
        self.lpp_ref
            .open_loan_req_id
            .expect("LPP Ref not created with borrow feature!")
    }

    fn repay_req_id(&self) -> ReplyId {
        self.lpp_ref
            .repay_req_id
            .expect("LPP Ref not created with borrow feature!")
    }

    fn liquidation_repay_id(&self) -> ReplyId {
        self.lpp_ref
            .liquidation_repay_id
            .expect("LPP Ref not created with borrow feature!")
    }
}

impl<'a, Lpn> Lpp<Lpn> for LppStub<'a, Lpn>
where
    Lpn: Currency + DeserializeOwned,
{
    fn id(&self) -> Addr {
        self.lpp_ref.addr.clone()
    }

    fn open_loan_req(&mut self, amount: Coin<Lpn>) -> Result<()> {
        self.batch
            .schedule_execute_wasm_on_success_reply::<_, Lpn>(
                &self.id(),
                ExecuteMsg::OpenLoan {
                    amount: amount.into(),
                },
                None,
                self.open_loan_req_id(),
            )
            .map_err(ContractError::from)
    }

    fn open_loan_resp(&self, resp: Reply) -> Result<LoanResponse<Lpn>> {
        debug_assert_eq!(resp.id, self.open_loan_req_id(),);

        from_execute(resp)
            .map_err(Into::into)
            .and_then(|maybe_data| {
                maybe_data.ok_or_else(|| ContractError::CustomError {
                    val: "No data passed as response!".into(),
                })
            })
    }

    fn repay_loan_req(&mut self, repayment: Coin<Lpn>, liquidation: bool) -> Result<()> {
        self.batch
            .schedule_execute_wasm_on_success_reply(
                &self.id(),
                ExecuteMsg::RepayLoan(),
                Some(repayment),
                if liquidation {
                    self.liquidation_repay_id()
                } else {
                    self.repay_req_id()
                },
            )
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

    fn loan_outstanding_interest(
        &self,
        lease: impl Into<Addr>,
        by: Timestamp,
    ) -> Result<QueryLoanOutstandingInterestResponse<Lpn>> {
        let msg = QueryMsg::LoanOutstandingInterest {
            lease_addr: lease.into(),
            outstanding_time: by,
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

    fn lpp_balance(&self) -> Result<LppBalanceResponse<Lpn>> {
        let msg = QueryMsg::LppBalance();
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(ContractError::from)
    }

    fn nlpn_price(&self) -> Result<PriceResponse<Lpn>> {
        let msg = QueryMsg::Price();
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(ContractError::from)
    }

    fn config(&self) -> Result<QueryConfigResponse> {
        let msg = QueryMsg::Config();
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(ContractError::from)
    }

    fn nlpn_balance(&self, lender: impl Into<Addr>) -> Result<BalanceResponse> {
        let msg = QueryMsg::Balance {
            address: lender.into(),
        };
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(ContractError::from)
    }

    fn rewards(&self, lender: impl Into<Addr>) -> Result<RewardsResponse> {
        let msg = QueryMsg::Rewards {
            address: lender.into(),
        };
        self.querier
            .query_wasm_smart(self.id(), &msg)
            .map_err(ContractError::from)
    }
}

pub struct LppBatch {
    pub lpp_ref: LppRef,
    pub batch: Batch,
}

impl<'a, C> From<LppStub<'a, C>> for LppBatch {
    fn from(stub: LppStub<'a, C>) -> Self {
        Self {
            lpp_ref: stub.lpp_ref,
            batch: stub.batch,
        }
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        from_binary, testing::MockQuerier, Addr, CosmosMsg, QuerierWrapper, ReplyOn, Response,
        WasmMsg,
    };

    use finance::{
        coin::Coin,
        currency::{Currency, Nls},
    };

    use crate::{
        msg::ExecuteMsg,
        stub::{LppBatch, LppRef},
    };

    use super::Lpp;

    #[test]
    fn open_loan_req() {
        // Magic number to test correctly against default zero.
        const OPEN_LOAN_REQ_ID: u64 = 0xC0FFEE;

        let addr = Addr::unchecked("defd2r2");
        let lpp = LppRef {
            addr: addr.clone(),
            currency: ToOwned::to_owned(Nls::SYMBOL),
            open_loan_req_id: Some(OPEN_LOAN_REQ_ID),
            repay_req_id: None,
            liquidation_repay_id: None,
        };
        let borrow_amount = Coin::<Nls>::new(10);
        let querier = MockQuerier::default();
        let wrapper = QuerierWrapper::new(&querier);
        let mut lpp_stub = lpp.into_stub(&wrapper);
        lpp_stub
            .open_loan_req(borrow_amount)
            .expect("open new loan request failed");
        let LppBatch { lpp_ref: _, batch } = lpp_stub.into();
        let resp: Response = batch.into();
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
