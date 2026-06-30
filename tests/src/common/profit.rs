use dex::{ConnectionParams, Ics20Channel};
use platform::contract::{Code, CodeId};
use profit::{CadenceHours, contract, msg::InstantiateMsg};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, instantiate2_address};
use sdk::testing;

use super::{
    ADMIN, CwContractWrapper,
    remote_profit_controller_stub::Instantiator as RemoteProfitControllerStubInstantiator,
    test_case::{TestCase, app::App},
};

/// The `Instantiate2` salt the profit precomputes and instantiates its drain
/// vault under — must match `profit::contract`'s `VAULT_SALT`.
const VAULT_SALT: &[u8] = b"profit-drain-vault";

pub(crate) struct Instantiator;

/// The profit instance plus the stand-in remote-profit controller authorised to
/// deliver its callbacks. The controller is instantiated against the profit's
/// stored `Code` so its `profit_code` authorisation accepts the profit's
/// outbound packets.
pub(crate) struct ProfitInstance {
    pub profit: Addr,
    pub controller: Addr,
    /// The deterministic drain-vault address the profit precomputed via
    /// `Instantiate2` and drains into. Recomputed from the same inputs the
    /// contract uses so the cycle drivers can fund the vault to release the
    /// arrival gate.
    pub drain_vault: Addr,
}

/// Recompute the profit's drain-vault `Instantiate2` address exactly as the
/// contract does (`profit::contract::precompute_vault_address`): the vault
/// code's checksum, the profit's canonical address (the creator), and the
/// fixed salt. Lets a cycle driver land the bridged NLS in the vault to
/// release the funds-arrival gate.
pub(crate) fn drain_vault_address(
    querier: QuerierWrapper<'_>,
    vault_code: Code,
    profit: &Addr,
) -> Addr {
    use sdk::cosmwasm_std::Api as _;

    let api = sdk::cosmwasm_std::testing::MockApi::default().with_prefix("nolus");
    let checksum = querier
        .query_wasm_code_info(CodeId::from(vault_code))
        .expect("the vault code info must resolve")
        .checksum;
    let creator = api
        .addr_canonicalize(profit.as_str())
        .expect("the profit address canonicalizes");
    let canonical = instantiate2_address(checksum.as_ref(), &creator, VAULT_SALT)
        .expect("the vault address derives");
    api.addr_humanize(&canonical)
        .expect("the vault address humanizes")
}

impl Instantiator {
    /// Instantiate the remote-swap profit and its controller stand-in.
    ///
    /// The profit's instantiate precomputes its `drain_vault` via `Instantiate2`,
    /// instantiates it (resolved through the reply path), then emits the
    /// `OpenProfit` packet to the controller. The Ok-mode stand-in synthesises
    /// the `OpenProfit` acknowledgment inline, so the profit learns its Solana
    /// authority and reaches `Idle` within this transaction.
    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        cadence_hours: CadenceHours,
        treasury: Addr,
        oracle: Addr,
        timealarms: Addr,
    ) -> ProfitInstance {
        let profit_endpoints =
            CwContractWrapper::new(contract::execute, contract::instantiate, contract::query)
                .with_reply(contract::reply)
                .with_sudo(contract::sudo);
        let profit_code: Code = app.store_code(Box::new(profit_endpoints));

        let vault_endpoints = CwContractWrapper::new(
            drain_vault::contract::execute,
            drain_vault::contract::instantiate,
            drain_vault::contract::query,
        );
        let vault_code: Code = app.store_code(Box::new(vault_endpoints));

        // The controller authorises by `profit_code`, so it must be installed
        // first — the profit's `OpenProfit` packet is sent during its own
        // instantiate and is rejected unless the controller already knows the
        // profit's code id.
        let controller = RemoteProfitControllerStubInstantiator::instantiate(app, profit_code);

        let msg = InstantiateMsg {
            cadence_hours,
            treasury,
            oracle,
            timealarms,
            dex: ConnectionParams {
                connection_id: TestCase::DEX_CONNECTION_ID.into(),
                transfer_channel: Ics20Channel {
                    local_endpoint: TestCase::PROFIT_IBC_CHANNEL.into(),
                    remote_endpoint: "channel-262".into(),
                },
            },
            remote_profit_controller: controller.clone(),
            vault_code_id: CodeId::from(vault_code).into(),
        };

        let profit = app
            .instantiate(profit_code, testing::user(ADMIN), &msg, &[], "profit", None)
            .map(|response| response.unwrap_response())
            .expect("the remote-swap profit must instantiate");

        let drain_vault = drain_vault_address(app.query(), vault_code, &profit);

        ProfitInstance {
            profit,
            controller,
            drain_vault,
        }
    }
}
