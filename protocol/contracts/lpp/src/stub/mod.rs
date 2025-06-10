use std::marker::PhantomData;

use deposit::WithDepositer;
use serde::{Deserialize, Serialize};

use currency::{self, CurrencyDTO, CurrencyDef};
use platform::batch::Batch;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::Error,
    msg::{LoanResponse, QueryLoanResponse, QueryMsg},
};

use self::{
    deposit::Impl as DepositerImpl,
    lender::{LppLenderStub, WithLppLender},
    loan::{LppLoanImpl, WithLppLoan},
};

pub mod deposit;
pub mod lender;
pub mod loan;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Eq, PartialEq))]
pub struct LppRef<Lpn> {
    addr: Addr,
    #[serde(skip)]
    _lpn: PhantomData<Lpn>,
}

impl<Lpn> LppRef<Lpn>
where
    Lpn: CurrencyDef,
{
    pub fn try_new(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self, Error> {
        querier
            .query_wasm_smart(addr.clone(), &QueryMsg::<Lpn::Group>::Lpn())
            .map_err(Error::from)
            .and_then(|lpn: CurrencyDTO<Lpn::Group>| {
                lpn.of_currency(Lpn::dto()).map_err(Error::UnknownCurrency)
            })
            .map(|()| Self {
                addr,
                _lpn: PhantomData,
            })
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn execute_loan<Cmd>(
        self,
        cmd: Cmd,
        lease: impl Into<Addr>,
        querier: QuerierWrapper<'_>,
    ) -> Result<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLppLoan<Lpn>,
        Error: Into<Cmd::Error>,
    {
        self.into_loan(lease, querier)
            .map_err(Into::into)
            .and_then(|lpp_loan| cmd.exec(lpp_loan))
    }

    pub fn execute_lender<Cmd>(
        self,
        cmd: Cmd,
        querier: QuerierWrapper<'_>,
    ) -> Result<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLppLender<Lpn>,
        Error: Into<Cmd::Error>,
    {
        cmd.exec(self.into_lender(querier))
    }

    pub fn execute_depositer<Cmd>(self, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
    where
        Cmd: WithDepositer<Lpn>,
        Error: Into<Cmd::Error>,
    {
        cmd.exec(DepositerImpl::new(self))
    }

    fn into_loan<A>(self, lease: A, querier: QuerierWrapper<'_>) -> Result<LppLoanImpl<Lpn>, Error>
    where
        A: Into<Addr>,
    {
        querier
            .query_wasm_smart(
                self.addr().clone(),
                &QueryMsg::<Lpn::Group>::Loan {
                    lease_addr: lease.into(),
                },
            )
            .map_err(Into::into)
            .and_then(|may_loan: QueryLoanResponse<Lpn>| may_loan.ok_or(Error::NoLoan {}))
            .map(|loan: LoanResponse<Lpn>| LppLoanImpl::new(self, loan))
    }

    fn into_lender(self, querier: QuerierWrapper<'_>) -> LppLenderStub<'_, Lpn> {
        LppLenderStub::new(self, querier)
    }
}

#[cfg(any(test, feature = "testing"))]
impl<Lpn> LppRef<Lpn> {
    pub fn unchecked<A>(addr: A) -> Self
    where
        A: Into<String>,
    {
        Self {
            addr: Addr::unchecked(addr),
            _lpn: PhantomData,
        }
    }
}

pub struct LppBatch<Ref> {
    pub lpp_ref: Ref,
    pub batch: Batch,
}
