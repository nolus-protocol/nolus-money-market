use std::result::Result as StdResult;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currencies::Lpns;
use currency::{
    error::CmdError, AnyVisitor, AnyVisitorResult, Currency, GroupVisit, SymbolOwned, SymbolSlice,
    Tickers,
};
use platform::batch::Batch;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{ContractError, Result},
    msg::{LoanResponse, QueryLoanResponse, QueryMsg},
    state::Config,
};

use self::{
    lender::{LppLenderStub, WithLppLender},
    loan::{LppLoanImpl, WithLppLoan},
};

pub mod lender;
pub mod loan;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct LppRef {
    addr: Addr,
    currency: SymbolOwned,
}

impl LppRef {
    pub fn try_new(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self> {
        let resp: Config = querier.query_wasm_smart(addr.clone(), &QueryMsg::Config())?;

        let currency = resp.lpn_ticker().into();

        Ok(Self { addr, currency })
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn currency(&self) -> &SymbolSlice {
        &self.currency
    }

    pub fn execute_loan<Cmd>(
        self,
        cmd: Cmd,
        lease: impl Into<Addr>,
        querier: QuerierWrapper<'_>,
    ) -> StdResult<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLppLoan,
        ContractError: Into<Cmd::Error>,
    {
        struct CurrencyVisitor<'a, Cmd, Lease> {
            cmd: Cmd,
            lpp_ref: LppRef,
            lease: Lease,
            querier: QuerierWrapper<'a>,
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

        // TODO push the group
        Tickers
            .visit_any::<Lpns, _>(
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
        querier: QuerierWrapper<'_>,
    ) -> StdResult<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLppLender,
        ContractError: Into<Cmd::Error>,
    {
        struct CurrencyVisitor<'a, Cmd> {
            cmd: Cmd,
            lpp_ref: LppRef,
            querier: QuerierWrapper<'a>,
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

        // TODO push the group
        Tickers
            .visit_any::<Lpns, _>(
                &self.currency.clone(),
                CurrencyVisitor {
                    cmd,
                    lpp_ref: self,
                    querier,
                },
            )
            .map_err(CmdError::into_customer_err)
    }

    fn into_loan<Lpn>(
        self,
        lease: impl Into<Addr>,
        querier: QuerierWrapper<'_>,
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

    fn into_lender<C>(self, querier: QuerierWrapper<'_>) -> LppLenderStub<'_, C>
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

pub struct LppBatch<Ref> {
    pub lpp_ref: Ref,
    pub batch: Batch,
}
