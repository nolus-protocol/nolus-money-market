use serde::{Deserialize, Serialize};

use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use remote_lease::response::RemoteLeaseId;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    finance::{LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    loan::LoanDTO,
    position::{Position, PositionDTO, PositionError},
};

use super::{
    Lease,
    with_lease::WithLease,
    with_lease_deps::{self, WithLeaseDeps},
};

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "contract_testing", derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeaseDTO {
    pub(crate) addr: Addr,
    pub(crate) customer: Addr,
    pub(crate) position: PositionDTO,
    pub(crate) loan: LoanDTO,
    pub(crate) time_alarms: TimeAlarmsRef,
    pub(crate) oracle: OracleRef,
    pub(crate) reserve: ReserveRef,
    /// Solana-side Lease PDA, populated from the remote-lease
    /// controller's OpenLease ack before the live lease is ever
    /// constructed. Every `LeaseDTO` carries a value: remote-lease
    /// protocols are instantiated fresh, so no pre-remote-lease (v9)
    /// lease is ever deserialised under this layout.
    pub(crate) remote_lease_id: RemoteLeaseId,
    /// The remote-lease controller the post-opening legs emit operations
    /// to, pinned at lease open from the opening spec. Persisted rather
    /// than re-queried so a leaser re-configuration can never repoint a
    /// live lease at a different controller mid-workflow.
    pub(crate) remote_lease_controller: Addr,
}

impl LeaseDTO {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        addr: Addr,
        customer: Addr,
        position: PositionDTO,
        loan: LoanDTO,
        time_alarms: TimeAlarmsRef,
        oracle: OracleRef,
        reserve: ReserveRef,
        remote_lease_id: RemoteLeaseId,
        remote_lease_controller: Addr,
    ) -> Self {
        Self {
            addr,
            customer,
            position,
            loan,
            time_alarms,
            oracle,
            reserve,
            remote_lease_id,
            remote_lease_controller,
        }
    }

    pub(crate) fn execute<Cmd>(
        self,
        cmd: Cmd,
        querier: QuerierWrapper<'_>,
    ) -> Result<Cmd::Output, Cmd::Error>
    where
        Cmd: WithLease,
        PositionError: Into<Cmd::Error>,
        lpp::stub::lender::Error: Into<Cmd::Error>,
        currency::error::Error: Into<Cmd::Error>,
        timealarms::stub::Error: Into<Cmd::Error>,
    {
        let lease = self.addr.clone();
        let position = self.position.clone();
        let lpp = self.loan.lpp().clone();
        let oracle = self.oracle.clone();

        with_lease_deps::execute(
            Factory::new(cmd, self),
            lease,
            position,
            lpp,
            oracle,
            querier,
        )
    }
}

struct Factory<Cmd> {
    cmd: Cmd,
    lease_dto: LeaseDTO,
}
impl<Cmd> Factory<Cmd> {
    pub(super) fn new(cmd: Cmd, lease_dto: LeaseDTO) -> Self {
        Self { cmd, lease_dto }
    }
}

impl<Cmd> WithLeaseDeps for Factory<Cmd>
where
    Cmd: WithLease,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        position: Position<Asset>,
        lpp_loan: LppLoan,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        self.cmd.exec(Lease::<Asset, _, _>::from_dto(
            self.lease_dto,
            position,
            lpp_loan,
            oracle,
        ))
    }
}
