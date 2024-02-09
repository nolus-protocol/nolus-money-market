use std::{marker::PhantomData, result::Result as StdResult};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::{
    error::CmdError, AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit, SymbolOwned,
    SymbolSlice, Tickers,
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
#[cfg_attr(any(test, feature = "testing"), derive(Eq, PartialEq))]
pub struct LppRef<Lpns> {
    addr: Addr,
    lpn: SymbolOwned,
    #[serde(skip)]
    _lpns: PhantomData<Lpns>,
}

impl<Lpns> LppRef<Lpns>
where
    Lpns: Group + Serialize,
{
    pub fn try_new(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self> {
        let resp: Config = querier.query_wasm_smart(addr.clone(), &QueryMsg::<Lpns>::Config())?;

        let lpn = resp.lpn_ticker();

        currency::validate::<Lpns>(lpn)
            .map(|()| Self {
                addr,
                lpn: lpn.into(),
                _lpns: PhantomData,
            })
            .map_err(Into::into)
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn lpn(&self) -> &SymbolSlice {
        &self.lpn
    }

    pub fn execute_loan<Cmd>(
        self,
        cmd: Cmd,
        lease: impl Into<Addr>,
        querier: QuerierWrapper<'_>,
    ) -> StdResult<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLppLoan<Lpns>,
        ContractError: Into<Cmd::Error>,
    {
        struct CurrencyVisitor<'a, Cmd, Lpns, Lease> {
            cmd: Cmd,
            lpp_ref: LppRef<Lpns>,
            lease: Lease,
            querier: QuerierWrapper<'a>,
        }

        impl<'a, Cmd, Lpns, Lease> AnyVisitor for CurrencyVisitor<'a, Cmd, Lpns, Lease>
        where
            Cmd: WithLppLoan<Lpns>,
            ContractError: Into<Cmd::Error>,
            Lpns: Group + Serialize,
            Lease: Into<Addr>,
        {
            type Output = Cmd::Output;
            type Error = CmdError<Cmd::Error, ContractError>;

            fn on<Lpn>(self) -> AnyVisitorResult<Self>
            where
                Lpn: Currency + Serialize + DeserializeOwned,
            {
                self.lpp_ref
                    .into_loan::<Lpn>(self.lease, self.querier)
                    .map_err(CmdError::from_api_err)
                    .and_then(|lpp_loan| {
                        self.cmd.exec(lpp_loan).map_err(CmdError::from_customer_err)
                    })
            }
        }

        Tickers
            .visit_any::<Lpns, _>(
                &self.lpn.clone(),
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
        Cmd: WithLppLender<Lpns>,
        ContractError: Into<Cmd::Error>,
    {
        struct CurrencyVisitor<'a, Cmd, Lpns> {
            cmd: Cmd,
            lpp_ref: LppRef<Lpns>,
            querier: QuerierWrapper<'a>,
        }

        impl<'a, Cmd, Lpns> AnyVisitor for CurrencyVisitor<'a, Cmd, Lpns>
        where
            Cmd: WithLppLender<Lpns>,
            Lpns: Group + Serialize,
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

        Tickers
            .visit_any::<Lpns, _>(
                &self.lpn.clone(),
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
    ) -> Result<LppLoanImpl<Lpn, Lpns>>
    where
        Lpn: Currency + DeserializeOwned,
        Lpns: Group + Serialize,
    {
        querier
            .query_wasm_smart(
                self.addr(),
                &QueryMsg::<Lpns>::Loan {
                    lease_addr: lease.into(),
                },
            )
            .map_err(Into::into)
            .and_then(|may_loan: QueryLoanResponse<Lpn>| may_loan.ok_or(ContractError::NoLoan {}))
            .map(|loan: LoanResponse<Lpn>| LppLoanImpl::new(self, loan))
    }

    fn into_lender<Lpn>(self, querier: QuerierWrapper<'_>) -> LppLenderStub<'_, Lpn, Lpns>
    where
        Lpn: Currency,
        Lpns: Group,
    {
        LppLenderStub::new(self, querier)
    }
}

#[cfg(any(test, feature = "testing"))]
impl<Lpns> LppRef<Lpns>
where
    Lpns: Group,
{
    pub fn unchecked<A, Lpn>(addr: A) -> Self
    where
        A: Into<String>,
        Lpn: Currency,
    {
        Self {
            addr: Addr::unchecked(addr),
            lpn: Lpn::TICKER.into(),
            _lpns: PhantomData::<Lpns>,
        }
    }
}

pub struct LppBatch<Ref> {
    pub lpp_ref: Ref,
    pub batch: Batch,
}
