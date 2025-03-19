use serde::{Deserialize, Serialize};

use dex::{Account, Connectable, ConnectionParams};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    lease::{LeaseDTO, with_lease::WithLease},
    position::PositionError,
};

pub use self::endpoins::{execute, instantiate, migrate, query, reply, sudo};
use self::finalize::FinalizerRef;

mod api;
mod cmd;
mod endpoins;
mod finalize;
pub mod msg;
mod state;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct Lease {
    lease: LeaseDTO,
    dex: Account,
    finalizer: FinalizerRef,
}

pub(crate) trait SplitDTOOut {
    type Other;

    fn split_into(self) -> (LeaseDTO, Self::Other);
}

impl Lease {
    fn new(lease: LeaseDTO, dex: Account, finalizer: FinalizerRef) -> Self {
        Self {
            lease,
            dex,
            finalizer,
        }
    }

    fn update<Cmd>(
        self,
        cmd: Cmd,
        querier: QuerierWrapper<'_>,
    ) -> Result<(Self, <Cmd::Output as SplitDTOOut>::Other), Cmd::Error>
    where
        Cmd: WithLease,
        Cmd::Output: SplitDTOOut,
        Cmd::Error: From<lpp::error::Error> + From<finance::error::Error> + From<PositionError>,
        currency::error::Error: Into<Cmd::Error>,
        timealarms::stub::Error: Into<Cmd::Error>,
        oracle_platform::error::Error: Into<Cmd::Error>,
    {
        self.lease.execute(cmd, querier).map(|result| {
            let (lease, other) = result.split_into();
            (Self::new(lease, self.dex, self.finalizer), other)
        })
    }
}

impl Connectable for Lease {
    fn dex(&self) -> &ConnectionParams {
        self.dex.dex()
    }
}
