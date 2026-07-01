use serde::{Deserialize, Serialize};

use currencies::{Lpn as QuoteC, Lpns as QuoteG};
use dex::Account;
use platform::{contract::Code, ica::HostAccount};
use remote_profit_wire::profit_id::RemoteProfitId;
use sdk::cosmwasm_std::Addr;
use timealarms::stub::TimeAlarmsRef;

use crate::{CadenceHours, error::ContractError, result::ContractResult};

type OracleRef = oracle_platform::OracleRef<QuoteC, QuoteG>;

/// The `drain_vault` identity — its code id (for the `Instantiate2` precompute
/// and verify) and the precomputed address committed at instantiation.
pub(crate) struct VaultConfig {
    pub code_id: Code,
    pub address: Addr,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    cadence_hours: CadenceHours,
    treasury: Addr,
    oracle: OracleRef,
    time_alarms: TimeAlarmsRef,
    /// The funding account — profit's own address plus the paired ICS-20
    /// transfer channel. Funding rides this channel; no ICA is registered.
    account: Account,
    /// The remote-profit controller authorised to deliver `RemoteProfitCallback`.
    remote_profit_controller: Addr,
    /// Code id of the `drain_vault` the profit instantiates and drains into.
    vault_code_id: Code,
    /// The drain-vault address, precomputed via `Instantiate2` and committed at
    /// instantiation; the drain sweeps from it back into the profit account.
    drain_vault: Addr,
    /// The Solana profit authority, learned once from the `open_profit`
    /// acknowledgment. `None` until then; no funding cycle may start while it is
    /// `None`, so a cycle can never address its funding transfer to nowhere.
    profit_authority: Option<RemoteProfitId>,
}

impl Config {
    pub fn new(
        cadence_hours: CadenceHours,
        treasury: Addr,
        oracle: OracleRef,
        time_alarms: TimeAlarmsRef,
        account: Account,
        remote_profit_controller: Addr,
        vault: VaultConfig,
    ) -> Self {
        let VaultConfig {
            code_id: vault_code_id,
            address: drain_vault,
        } = vault;
        Self {
            cadence_hours,
            treasury,
            oracle,
            time_alarms,
            account,
            remote_profit_controller,
            vault_code_id,
            drain_vault,
            profit_authority: None,
        }
    }

    pub fn update(self, cadence_hours: CadenceHours) -> Self {
        Self {
            cadence_hours,
            ..self
        }
    }

    /// Persist the Solana profit authority learned from the `open_profit`
    /// acknowledgment. Learned once: a second establishment overwrites the same
    /// singleton authority rather than accumulating instances.
    pub fn with_profit_authority(self, profit_authority: RemoteProfitId) -> Self {
        Self {
            profit_authority: Some(profit_authority),
            ..self
        }
    }

    pub fn cadence_hours(&self) -> CadenceHours {
        self.cadence_hours
    }

    pub fn treasury(&self) -> &Addr {
        &self.treasury
    }

    pub fn time_alarms(&self) -> &TimeAlarmsRef {
        &self.time_alarms
    }

    pub fn account(&self) -> &Account {
        &self.account
    }

    pub fn remote_profit_controller(&self) -> &Addr {
        &self.remote_profit_controller
    }

    pub fn drain_vault(&self) -> &Addr {
        &self.drain_vault
    }

    /// Bridge the learned Solana authority into the `HostAccount` the funding
    /// ICS-20 transfer addresses, failing closed if no `open_profit`
    /// acknowledgment has set it yet.
    pub fn funding_receiver(&self) -> ContractResult<HostAccount> {
        self.profit_authority
            .as_ref()
            .ok_or(ContractError::SolanaAuthorityNotLearned)
            .and_then(|authority| {
                HostAccount::try_from(authority.as_str().to_owned()).map_err(Into::into)
            })
    }
}
