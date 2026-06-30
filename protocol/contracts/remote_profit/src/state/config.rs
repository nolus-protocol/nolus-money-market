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
    profit_code: Code,
    profit_contract: Addr,
}

impl Config {
    const STORAGE: Item<Self> = Item::new("config");

    pub fn new(
        connection_id: String,
        dex_label: String,
        transfer_channel: String,
        profit_code: Code,
        profit_contract: Addr,
    ) -> Self {
        let obj = Self {
            connection_id,
            dex_label,
            transfer_channel,
            profit_code,
            profit_contract,
        };
        debug_assert!(obj.invariant_held());
        obj
    }

    pub fn invariant_held(&self) -> bool {
        !self.connection_id.is_empty()
            && !self.dex_label.is_empty()
            && canonical_transfer_channel(&self.transfer_channel)
            && !self.profit_contract.as_str().is_empty()
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

    pub const fn profit_code(&self) -> Code {
        self.profit_code
    }

    /// The single local profit instance every ack/timeout callback routes to.
    ///
    /// The remote profit is a SINGLETON (ADR-0008): there is exactly one profit
    /// per port/channel, so the callback target is fixed at instantiation rather
    /// than carried on the packet envelope (the way the multi-instance remote
    /// lease carries its addressee).
    pub const fn profit_contract(&self) -> &Addr {
        &self.profit_contract
    }

    pub(super) fn into_parts(self) -> (String, String, String, Code, Addr) {
        (
            self.connection_id,
            self.dex_label,
            self.transfer_channel,
            self.profit_code,
            self.profit_contract,
        )
    }

    pub fn store(&self, storage: &mut dyn Storage) -> Result<()> {
        Self::STORAGE.save(storage, self).map_err(Into::into)
    }

    pub fn load(storage: &dyn Storage) -> Result<Self> {
        Self::STORAGE.load(storage).map_err(Into::into)
    }

    pub fn update_profit_code(storage: &mut dyn Storage, profit_code: Code) -> Result<()> {
        Self::STORAGE
            .update(storage, |config: Self| {
                let updated = Self {
                    profit_code,
                    ..config
                };
                debug_assert!(updated.invariant_held());
                Ok(updated)
            })
            .map(mem::drop)
    }

    /// Prove the stored config deserializes under the current schema and
    /// upholds its invariant.
    ///
    /// Run by `migrate` so an instance whose stored config predates a required
    /// field, or carries values the current code would never have accepted,
    /// refuses the upgrade instead of bricking on the first post-upgrade load.
    pub fn require_current_schema(storage: &dyn Storage) -> Result<()> {
        Self::STORAGE
            .load(storage)
            .map_err(Error::IncompatibleStoredConfig)
            .and_then(|config| {
                if config.invariant_held() {
                    Ok(())
                } else {
                    Err(Error::MalformedStoredConfig)
                }
            })
    }

    /// Verify the caller is a contract instance of `Config.profit_code`.
    ///
    /// Both failure paths (the caller is not a contract; the caller has a different
    /// code id) collapse to a single `UnauthorisedCaller` — the controller does not
    /// distinguish them at the protocol layer, and surfacing the underlying platform
    /// error would leak internal cosmwasm shape into the public error surface.
    pub fn auth_caller(&self, querier: QuerierWrapper<'_>, caller: Addr) -> Result<Addr> {
        contract::validator(querier)
            .check_contract_code(caller, &self.profit_code)
            .map_err(|_err| Error::UnauthorisedCaller)
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
    const PROFIT_USER: &str = "profit";
    const PROFIT_CONTRACT: &str = "profit-contract";

    #[test]
    fn store_load() {
        let profit_code = Code::unchecked(12);
        let mut store = MockStorage::new();
        config(profit_code).store(&mut store).unwrap();
        let loaded = Config::load(&store).unwrap();
        assert_eq!(CONNECTION_ID, loaded.connection_id());
        assert_eq!(DEX_LABEL, loaded.dex_label());
        assert_eq!(TRANSFER_CHANNEL, loaded.transfer_channel());
        assert_eq!(
            &sdk_testing::user(PROFIT_CONTRACT),
            loaded.profit_contract()
        );
        assert_profit_code(profit_code, &store);
    }

    #[test]
    fn update_load() {
        let profit_code = Code::unchecked(28);
        let new_profit_code = Code::unchecked(CodeId::from(profit_code) + 10);
        let mut store = MockStorage::new();
        config(profit_code).store(&mut store).unwrap();
        Config::update_profit_code(&mut store, new_profit_code).unwrap();
        assert_profit_code(new_profit_code, &store);
        let loaded = Config::load(&store).unwrap();
        assert_eq!(CONNECTION_ID, loaded.connection_id());
        assert_eq!(DEX_LABEL, loaded.dex_label());
        assert_eq!(TRANSFER_CHANNEL, loaded.transfer_channel());
        assert_eq!(
            &sdk_testing::user(PROFIT_CONTRACT),
            loaded.profit_contract()
        );
    }

    #[test]
    fn update_profit_code_when_storage_empty_errors() {
        let mut store = MockStorage::new();
        let err = Config::update_profit_code(&mut store, Code::unchecked(7)).unwrap_err();
        assert!(matches!(err, Error::Std(_)), "got {err:?}");
        assert!(Config::load(&store).is_err());
    }

    #[test]
    fn auth_caller_matching_code() {
        let profit_code = Code::unchecked(11);
        let profit = sdk_testing::user(PROFIT_USER);
        let querier = querier_with(profit.clone(), profit_code);
        let returned = config(profit_code)
            .auth_caller(QuerierWrapper::new(&querier), profit.clone())
            .unwrap();
        assert_eq!(profit, returned);
    }

    #[test]
    fn auth_caller_mismatched_code() {
        let configured = Code::unchecked(11);
        let actual = Code::unchecked(99);
        let profit = sdk_testing::user(PROFIT_USER);
        let querier = querier_with(profit.clone(), actual);
        let err = config(configured)
            .auth_caller(QuerierWrapper::new(&querier), profit)
            .unwrap_err();
        assert!(matches!(err, Error::UnauthorisedCaller), "got {err:?}");
    }

    #[test]
    fn auth_caller_non_contract_caller() {
        let profit_code = Code::unchecked(11);
        let querier = MockQuerier::default();
        let err = config(profit_code)
            .auth_caller(
                QuerierWrapper::new(&querier),
                sdk_testing::user(PROFIT_USER),
            )
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
            profit_code: Code::unchecked(1),
            profit_contract: sdk_testing::user(PROFIT_CONTRACT),
        };
        assert!(!non_canonical_channel.invariant_held());

        let empty_connection = Config {
            connection_id: String::new(),
            dex_label: DEX_LABEL.into(),
            transfer_channel: TRANSFER_CHANNEL.into(),
            profit_code: Code::unchecked(1),
            profit_contract: sdk_testing::user(PROFIT_CONTRACT),
        };
        assert!(!empty_connection.invariant_held());
    }

    fn config(profit_code: Code) -> Config {
        Config::new(
            CONNECTION_ID.into(),
            DEX_LABEL.into(),
            TRANSFER_CHANNEL.into(),
            profit_code,
            sdk_testing::user(PROFIT_CONTRACT),
        )
    }

    fn assert_profit_code(expected: Code, store: &dyn Storage) {
        assert_eq!(expected, Config::load(store).unwrap().profit_code());
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
