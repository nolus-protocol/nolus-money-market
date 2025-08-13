use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

use access_control::{
    permissions::{ContractOwnerPermission, DexResponseSafeDeliveryPermission},
    user::User,
};
use dex::{
    Account, CheckType, Connectable, ConnectionParams, Contract, Handler, IcaConnectee,
    Response as DexResponse, Result as DexResult,
};
use finance::duration::Duration;
use sdk::cosmwasm_std::{Addr, ContractInfo, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmDelivery;

use crate::{msg::ConfigResponse, result::ContractResult};

use super::{Config, State, idle::Idle};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct OpenIca {
    config: Config,
    dex: ConnectionParams,
}

impl OpenIca {
    pub fn new(config: Config, connection: ConnectionParams) -> Self {
        Self {
            config,
            dex: connection,
        }
    }
}

impl IcaConnectee for OpenIca {
    type State = State;
    type NextState = Idle;

    fn connected(self, account: Account) -> Self::NextState {
        Idle::new(self.config, account)
    }
}

impl Connectable for OpenIca {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}

impl Contract for OpenIca {
    type StateResponse = ConfigResponse;

    fn state(
        self,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        }
    }
}

impl Display for OpenIca {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("Idle"))
    }
}

impl Handler for OpenIca {
    type Response = State;
    type SwapResult = ContractResult<DexResponse<State>>;

    fn check_permission<U>(
        &self,
        user: &U,
        check_type: CheckType,
        contract_info: Option<ContractInfo>,
    ) -> DexResult<Self>
    where
        U: User,
    {
        match check_type {
            CheckType::Timealarm => {
                access_control::check(&TimeAlarmDelivery::new(&self.config.time_alarms()), user)?;
            }
            CheckType::ContractOwner => {
                access_control::check(
                    &ContractOwnerPermission::new(&self.config.contract_owner()),
                    user,
                )?;
            }
            CheckType::DexResponseSafeDelivery => {
                access_control::check(
                    &DexResponseSafeDeliveryPermission::new(&contract_info),
                    user,
                )?;
            }
            CheckType::None => {}
        }
    }
}
