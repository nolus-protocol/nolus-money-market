use std::{marker::PhantomData, result::Result as StdResult};

use currency::error::CmdError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::{self, lpn::Lpns, AnyVisitor, AnyVisitorResult, Currency, Symbol, SymbolOwned};
use platform::batch::Batch;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{ContractError, Result},
    msg::{LoanResponse, LppBalanceResponse, QueryLoanResponse, QueryMsg},
    state::Config,
};

use self::{
    lender::{LppLenderStub, WithLppLender},
    loan::{LppLoanImpl, WithLppLoan},
};

pub mod lender;
pub mod loan;

pub trait Lpp<Lpn>
where
    Lpn: Currency,
{
    fn lpp_balance(&self) -> Result<LppBalanceResponse<Lpn>>;
}

pub trait WithLpp {
    type Output;
    type Error;

    fn exec<C, L>(self, lpp: L) -> StdResult<Self::Output, Self::Error>
    where
        L: Lpp<C>,
        C: Currency;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct LppRef {
    addr: Addr,
    currency: SymbolOwned,
}

impl LppRef {
    pub fn try_new(addr: Addr, querier: &QuerierWrapper<'_>) -> Result<Self> {
        let resp: Config = querier.query_wasm_smart(addr.clone(), &QueryMsg::Config())?;

        let currency = resp.lpn_ticker().into();

        Ok(Self { addr, currency })
    }

    #[cfg(feature = "migration")]
    pub fn new(addr: Addr, currency: SymbolOwned) -> Self {
        Self { addr, currency }
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn currency(&self) -> Symbol<'_> {
        &self.currency
    }

    pub fn execute<V>(self, cmd: V, querier: &QuerierWrapper<'_>) -> StdResult<V::Output, V::Error>
    where
        V: WithLpp,
        ContractError: Into<V::Error>,
    {
        struct CurrencyVisitor<'a, V> {
            cmd: V,
            lpp_ref: LppRef,
            querier: &'a QuerierWrapper<'a>,
        }

        impl<'a, V> AnyVisitor for CurrencyVisitor<'a, V>
        where
            V: WithLpp,
        {
            type Output = V::Output;
            type Error = CmdError<V::Error, ContractError>;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency + Serialize + DeserializeOwned,
            {
                self.cmd
                    .exec(self.lpp_ref.into_stub::<C>(self.querier))
                    .map_err(CmdError::from_customer_err)
            }
        }

        currency::visit_any_on_ticker::<Lpns, _>(
            &self.currency.clone(),
            CurrencyVisitor {
                cmd,
                lpp_ref: self,
                querier,
            },
        )
        .map_err(CmdError::into_customer_err)
    }

    pub fn execute_loan<Cmd>(
        self,
        cmd: Cmd,
        lease: impl Into<Addr>,
        querier: &QuerierWrapper<'_>,
    ) -> StdResult<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLppLoan,
        ContractError: Into<Cmd::Error>,
    {
        struct CurrencyVisitor<'a, Cmd, Lease> {
            cmd: Cmd,
            lpp_ref: LppRef,
            lease: Lease,
            querier: &'a QuerierWrapper<'a>,
        }

        impl<'a, Cmd, Lease> AnyVisitor for CurrencyVisitor<'a, Cmd, Lease>
        where
            Cmd: WithLppLoan,
            ContractError: Into<Cmd::Error>,
            Lease: Into<Addr>,
        {
            type Output = Cmd::Output;
            type Error = CmdError<Cmd::Error, ContractError>;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency + Serialize + DeserializeOwned,
            {
                self.lpp_ref
                    .into_loan::<C>(self.lease, self.querier)
                    .map_err(CmdError::from_api_err)
                    .and_then(|lpp_loan| {
                        self.cmd.exec(lpp_loan).map_err(CmdError::from_customer_err)
                    })
            }
        }

        currency::visit_any_on_ticker::<Lpns, _>(
            &self.currency.clone(),
            CurrencyVisitor {
                cmd,
                lpp_ref: self,
                lease,
                querier,
            },
        )
        .map_err(CmdError::into_customer_err)
    }

    pub fn execute_lender<Cmd>(
        self,
        cmd: Cmd,
        querier: &QuerierWrapper<'_>,
    ) -> StdResult<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLppLender,
        ContractError: Into<Cmd::Error>,
    {
        struct CurrencyVisitor<'a, Cmd> {
            cmd: Cmd,
            lpp_ref: LppRef,
            querier: &'a QuerierWrapper<'a>,
        }

        impl<'a, Cmd> AnyVisitor for CurrencyVisitor<'a, Cmd>
        where
            Cmd: WithLppLender,
        {
            type Output = Cmd::Output;
            type Error = CmdError<Cmd::Error, ContractError>;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency + Serialize + DeserializeOwned,
            {
                self.cmd
                    .exec(self.lpp_ref.into_lender::<C>(self.querier))
                    .map_err(CmdError::from_customer_err)
            }
        }

        currency::visit_any_on_ticker::<Lpns, _>(
            &self.currency.clone(),
            CurrencyVisitor {
                cmd,
                lpp_ref: self,
                querier,
            },
        )
        .map_err(CmdError::into_customer_err)
    }

    fn into_stub<'a, C>(self, querier: &'a QuerierWrapper<'_>) -> LppStub<'a, C> {
        LppStub {
            lpp_ref: self,
            currency: PhantomData::<C>,
            querier,
        }
    }

    fn into_loan<Lpn>(
        self,
        lease: impl Into<Addr>,
        querier: &QuerierWrapper<'_>,
    ) -> Result<LppLoanImpl<Lpn>>
    where
        Lpn: Currency + DeserializeOwned,
    {
        querier
            .query_wasm_smart(
                self.addr(),
                &QueryMsg::Loan {
                    lease_addr: lease.into(),
                },
            )
            .map_err(Into::into)
            .and_then(|may_loan: QueryLoanResponse<Lpn>| may_loan.ok_or(ContractError::NoLoan {}))
            .map(|loan: LoanResponse<Lpn>| LppLoanImpl::new(self, loan))
    }

    fn into_lender<'a, C>(self, querier: &'a QuerierWrapper<'a>) -> LppLenderStub<'a, C>
    where
        C: Currency,
    {
        LppLenderStub::new(self, querier)
    }
}

#[cfg(any(test, feature = "testing"))]
impl LppRef {
    pub fn unchecked<A, Lpn>(addr: A) -> Self
    where
        A: Into<String>,
        Lpn: Currency,
    {
        Self {
            addr: Addr::unchecked(addr),
            currency: Lpn::TICKER.into(),
        }
    }
}

struct LppStub<'a, C> {
    lpp_ref: LppRef,
    currency: PhantomData<C>,
    querier: &'a QuerierWrapper<'a>,
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
}

pub struct LppBatch<Ref> {
    pub lpp_ref: Ref,
    pub batch: Batch,
}
