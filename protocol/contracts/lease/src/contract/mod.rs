use access_control::{
    AccessPermission,
    permissions::{SameContractOnly, SingleUserPermission},
};
use serde::{Deserialize, Serialize};

use currency::{Currency, Group, MemberOf};
use dex::{Account, Connectable, ConnectionParams};
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    lease::{LeaseDTO, with_lease::WithLease},
    position::PositionError,
};

pub use self::endpoins::{execute, instantiate, migrate, query, reply, sudo};
use self::finalize::LeasesRef;

mod api;
mod cmd;
mod endpoins;
mod finalize;
pub mod msg;
mod state;

type DexResponseSafeDeliveryPermission<'a> = SameContractOnly<'a>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Lease {
    lease: LeaseDTO,
    dex: Account,
    leases: LeasesRef,
}

pub(crate) struct LeaseDTOResult<Result> {
    pub lease: LeaseDTO,
    pub result: Result,
}

impl Lease {
    fn new(lease: LeaseDTO, dex: Account, leases: LeasesRef) -> Self {
        Self { lease, dex, leases }
    }

    fn update<Cmd, CmdResult>(
        self,
        cmd: Cmd,
        querier: QuerierWrapper<'_>,
    ) -> Result<(Self, CmdResult), Cmd::Error>
    where
        Cmd: WithLease<Output = LeaseDTOResult<CmdResult>>,
        PositionError: Into<Cmd::Error>,
        lpp::stub::lender::Error: Into<Cmd::Error>,
        currency::error::Error: Into<Cmd::Error>,
        timealarms::stub::Error: Into<Cmd::Error>,
    {
        self.lease.execute(cmd, querier).map(|result| {
            (
                Self::new(result.lease, self.dex, self.leases),
                result.result,
            )
        })
    }
}

impl Connectable for Lease {
    fn dex(&self) -> &ConnectionParams {
        self.dex.dex()
    }
}

/// This is a permission given to deliver price alarms 
pub struct PriceAlarmDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    oracle_ref: &'a OracleRef<QuoteC, QuoteG>,
}

impl<'a, QuoteC, QuoteG> PriceAlarmDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn new(oracle_ref: &'a OracleRef<QuoteC, QuoteG>) -> Self {
        Self { oracle_ref }
    }
}

impl<QuoteC, QuoteG> AccessPermission for PriceAlarmDelivery<'_, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn is_granted_to(&self, caller: &Addr) -> bool {
        self.oracle_ref.owned_by(caller)
    }
}
