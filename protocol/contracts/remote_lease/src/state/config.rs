use std::mem;

use serde::{Deserialize, Serialize};

use platform::contract::{self, Code, Validator as _};
use sdk::{
    cosmwasm_std::{Addr, QuerierWrapper, Storage},
    cw_storage_plus::Item,
};

use crate::error::{Error, Result};

const TRANSFER_CHANNEL_NAME_PREFIX: &str = "channel-";

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Config {
    connection_id: String,
    dex_label: String,
    transfer_channel: String,
    lease_code: Code,
}

impl Config {
    const STORAGE: Item<Self> = Item::new("config");

    pub fn new(
        connection_id: String,
        dex_label: String,
        transfer_channel: String,
        lease_code: Code,
    ) -> Self {
        let obj = Self {
            connection_id,
            dex_label,
            transfer_channel,
            lease_code,
        };
        debug_assert!(obj.invariant_held());
        obj
    }

    pub fn invariant_held(&self) -> bool {
        !self.connection_id.is_empty()
            && !self.dex_label.is_empty()
            && canonical_transfer_channel(&self.transfer_channel)
    }

    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    pub fn dex_label(&self) -> &str {
        &self.dex_label
    }

    pub fn transfer_channel(&self) -> &str {
        &self.transfer_channel
    }

    pub const fn lease_code(&self) -> Code {
        self.lease_code
    }

    pub(super) fn into_parts(self) -> (String, String, String, Code) {
        (
            self.connection_id,
            self.dex_label,
            self.transfer_channel,
            self.lease_code,
        )
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update_lease_code(storage: &mut dyn Storage, lease_code: Code) -> Result<()> {
        Self::STORAGE
            .update(storage, |config: Self| {
                let updated = Self {
                    lease_code,
                    ..config
                };
                debug_assert!(updated.invariant_held());
                Ok(updated)
            })
            .map(mem::drop)
    }

    /// Prove the stored config deserializes under the current schema.
    ///
    /// Run by `migrate` so an instance whose stored config predates a required
    /// field refuses the upgrade instead of bricking on the first
    /// post-upgrade load.
    pub fn require_current_schema(storage: &dyn Storage) -> Result<()> {
        Self::STORAGE
            .load(storage)
            .map(mem::drop)
            .map_err(Error::IncompatibleStoredConfig)
    }

    /// Verify the caller is a contract instance of `Config.lease_code`.
    ///
    /// Both failure paths (the caller is not a contract; the caller has a different
    /// code id) collapse to a single `UnauthorisedCaller` — the controller does not
    /// distinguish them at the protocol layer, and surfacing the underlying platform
    /// error would leak internal cosmwasm shape into the public error surface.
    pub fn auth_caller(&self, querier: QuerierWrapper<'_>, caller: Addr) -> Result<Addr> {
        contract::validator(querier)
            .check_contract_code(caller, &self.lease_code)
            .map_err(|_| Error::UnauthorisedCaller)
    }
}

/// `true` only for the canonical decimal rendering of a `u16` ordinal behind
/// the `channel-` prefix — the counterparty's responder rejects leading zeros,
/// signs, and ordinals beyond its 16-bit entity range.
pub(crate) fn canonical_transfer_channel(channel_id: &str) -> bool {
    channel_id
        .strip_prefix(TRANSFER_CHANNEL_NAME_PREFIX)
        .and_then(|ordinal| {
            ordinal
                .parse::<u16>()
                .ok()
                .filter(|parsed| parsed.to_string() == ordinal)
        })
        .is_some()
}

#[cfg(test)]
mod test {
    use platform::contract::{Code, CodeId};
    use sdk::{
        cosmwasm_std::{
            Addr, ContractInfoResponse, ContractResult, QuerierWrapper, Storage, SystemError,
            SystemResult, WasmQuery,
            testing::{MockQuerier, MockStorage},
        },
        testing as sdk_testing,
    };

    use crate::error::Error;

    use super::Config;

    const CONNECTION_ID: &str = "connection-0";
    const DEX_LABEL: &str = "osmosis";
    const TRANSFER_CHANNEL: &str = "channel-4";
    const LEASE_USER: &str = "lease";

    #[test]
    fn store_load() {
        let lease_code = Code::unchecked(12);
        let mut store = MockStorage::new();
        config(lease_code).store(&mut store).unwrap();
        let loaded = Config::load(&store).unwrap();
        assert_eq!(CONNECTION_ID, loaded.connection_id());
        assert_eq!(DEX_LABEL, loaded.dex_label());
        assert_eq!(TRANSFER_CHANNEL, loaded.transfer_channel());
        assert_lease_code(lease_code, &store);
    }

    #[test]
    fn update_load() {
        let lease_code = Code::unchecked(28);
        let new_lease_code = Code::unchecked(CodeId::from(lease_code) + 10);
        let mut store = MockStorage::new();
        config(lease_code).store(&mut store).unwrap();
        Config::update_lease_code(&mut store, new_lease_code).unwrap();
        assert_lease_code(new_lease_code, &store);
        let loaded = Config::load(&store).unwrap();
        assert_eq!(CONNECTION_ID, loaded.connection_id());
        assert_eq!(DEX_LABEL, loaded.dex_label());
        assert_eq!(TRANSFER_CHANNEL, loaded.transfer_channel());
    }

    #[test]
    fn update_lease_code_when_storage_empty_errors() {
        let mut store = MockStorage::new();
        let err = Config::update_lease_code(&mut store, Code::unchecked(7)).unwrap_err();
        assert!(matches!(err, Error::Std(_)), "got {err:?}");
        assert!(Config::load(&store).is_err());
    }

    #[test]
    fn auth_caller_matching_code() {
        let lease_code = Code::unchecked(11);
        let lease = sdk_testing::user(LEASE_USER);
        let querier = querier_with(lease.clone(), lease_code);
        let returned = config(lease_code)
            .auth_caller(QuerierWrapper::new(&querier), lease.clone())
            .unwrap();
        assert_eq!(lease, returned);
    }

    #[test]
    fn auth_caller_mismatched_code() {
        let configured = Code::unchecked(11);
        let actual = Code::unchecked(99);
        let lease = sdk_testing::user(LEASE_USER);
        let querier = querier_with(lease.clone(), actual);
        let err = config(configured)
            .auth_caller(QuerierWrapper::new(&querier), lease)
            .unwrap_err();
        assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
    }

    #[test]
    fn auth_caller_non_contract_caller() {
        let lease_code = Code::unchecked(11);
        let querier = MockQuerier::default();
        let err = config(lease_code)
            .auth_caller(QuerierWrapper::new(&querier), sdk_testing::user(LEASE_USER))
            .unwrap_err();
        assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
    }

    #[test]
    fn invariant_violations_detected() {
        assert!(config(Code::unchecked(1)).invariant_held());

        let non_canonical_channel = Config {
            connection_id: CONNECTION_ID.into(),
            dex_label: DEX_LABEL.into(),
            transfer_channel: "channel-007".into(),
            lease_code: Code::unchecked(1),
        };
        assert!(!non_canonical_channel.invariant_held());

        let empty_connection = Config {
            connection_id: String::new(),
            dex_label: DEX_LABEL.into(),
            transfer_channel: TRANSFER_CHANNEL.into(),
            lease_code: Code::unchecked(1),
        };
        assert!(!empty_connection.invariant_held());
    }

    fn config(lease_code: Code) -> Config {
        Config::new(
            CONNECTION_ID.into(),
            DEX_LABEL.into(),
            TRANSFER_CHANNEL.into(),
            lease_code,
        )
    }

    fn assert_lease_code(expected: Code, store: &dyn Storage) {
        assert_eq!(expected, Config::load(store).unwrap().lease_code());
    }

    fn querier_with(contract: Addr, code: Code) -> MockQuerier {
        let mut querier = MockQuerier::default();
        let code_id = CodeId::from(code);
        querier.update_wasm(move |query| match query {
            WasmQuery::ContractInfo { contract_addr }
                if Addr::unchecked(contract_addr) == contract =>
            {
                SystemResult::Ok(ContractResult::Ok(
                    sdk::cosmwasm_std::to_json_binary(&ContractInfoResponse::new(
                        code_id,
                        sdk_testing::user("creator"),
                        None,
                        false,
                        None,
                        None,
                    ))
                    .expect("serialization succeeds"),
                ))
            }
            WasmQuery::ContractInfo { contract_addr }
            | WasmQuery::Smart { contract_addr, .. }
            | WasmQuery::Raw { contract_addr, .. } => {
                SystemResult::Err(SystemError::NoSuchContract {
                    addr: contract_addr.clone(),
                })
            }
            _ => unimplemented!(),
        });
        querier
    }
}
