use std::{marker::PhantomData, result::Result as StdResult};

use cosmwasm_std::{Addr, QuerierWrapper};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use finance::currency::{visit_any, AnyVisitor, Currency, SymbolOwned};
use platform::batch::{Batch, ReplyId};

use crate::{
    error::ContractError,
    msg::{
        BalanceResponse, LppBalanceResponse, PriceResponse, QueryConfigResponse, QueryMsg,
        RewardsResponse,
    },
};

pub mod lender;

pub type Result<T> = StdResult<T, ContractError>;

// TODO split into LppBorrow, LppLend, and LppAdmin traits
pub trait Lpp<Lpn>
where
    Self: Into<LppBatch<LppRef>>,
    Lpn: Currency,
{
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
}

impl LppRef {
    pub fn try_from(addr: Addr, querier: &QuerierWrapper) -> Result<Self> {
        Self::try_from_maybe_borrow(addr, querier, None)
    }

    pub fn try_borrow_from(
        addr: Addr,
        querier: &QuerierWrapper,
        open_loan_req_id: ReplyId,
    ) -> Result<Self> {
        Self::try_from_maybe_borrow(addr, querier, Some(open_loan_req_id))
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
    ) -> Result<Self> {
        let resp: QueryConfigResponse =
            querier.query_wasm_smart(addr.clone(), &QueryMsg::Config())?;

        let currency = resp.lpn_symbol;

        Ok(Self {
            addr,
            currency,
            open_loan_req_id,
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
    pub fn unchecked<A, Lpn>(addr: A, open_loan_req_id: Option<ReplyId>) -> Self
    where
        A: Into<String>,
        Lpn: Currency,
    {
        Self {
            addr: Addr::unchecked(addr),
            currency: Lpn::SYMBOL.into(),
            open_loan_req_id,
        }
    }
}

struct LppStub<'a, C> {
    lpp_ref: LppRef,
    currency: PhantomData<C>,
    querier: &'a QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, C> LppStub<'a, C> {
    fn id(&self) -> Addr {
        self.lpp_ref.addr.clone()
    }
}

impl<'a, Lpn> Lpp<Lpn> for LppStub<'a, Lpn>
where
    Lpn: Currency + DeserializeOwned,
{
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

pub struct LppBatch<Ref> {
    pub lpp_ref: Ref,
    pub batch: Batch,
}

impl<'a, C> From<LppStub<'a, C>> for LppBatch<LppRef> {
    fn from(stub: LppStub<'a, C>) -> Self {
        Self {
            lpp_ref: stub.lpp_ref,
            batch: stub.batch,
        }
    }
}
