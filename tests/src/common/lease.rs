use currencies::{LeaseGroup, Lpn};
use currency::{Currency, CurrencyDTO, CurrencyDef};
use dex::{ConnectionParams, Ics20Channel};
use finance::{
    coin::Coin,
    duration::{Duration, Seconds},
    fraction::Unit,
    liability::Liability,
    percent::{Percent, Percent100},
};
use lease::{
    api::{
        open::{LoanForm, NewLeaseContract, NewLeaseForm, PositionSpecDTO},
        query::{QueryMsg, StateResponse, opening::OngoingTrx as OpeningOngoingTrx},
    },
    contract,
};
use platform::{coin_legacy, contract::Code};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin},
    cw_multi_test::AppResponse,
    testing,
};

use super::{
    ADMIN, CwContractWrapper, USER, ibc,
    remote_lease_controller_stub::SwapFill,
    test_case::{
        TestCase,
        app::App,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
    },
};

pub(crate) struct Instantiator;

impl Instantiator {
    pub fn store(app: &mut App) -> Code {
        let endpoints =
            CwContractWrapper::new(contract::execute, contract::instantiate, contract::query)
                .with_reply(contract::reply)
                .with_sudo(contract::sudo);

        app.store_code(Box::new(endpoints))
    }

    #[track_caller]
    pub fn instantiate<D>(
        app: &mut App,
        code: Code,
        addresses: InstantiatorAddresses,
        lease_config: InitConfig<D>,
        config: InstantiatorConfig,
    ) -> Addr
    where
        D: CurrencyDef,
    {
        let downpayment = lease_config.downpayment;
        let msg = Self::lease_instantiate_msg(
            lease_config.lease_currency,
            addresses,
            config,
            lease_config.max_ltd,
        );

        let mut response: ResponseWithInterChainMsgs<'_, Addr> = app
            .instantiate(
                code,
                testing::user(ADMIN),
                &msg,
                &[coin_legacy::to_cosmwasm_on_nolus(downpayment)],
                "lease",
                None,
            )
            .unwrap();

        // The Ok-mode controller stand-in acks `OpenLease` inline, so the
        // lease converts the minted PDA to its remote account and funds it
        // with the downpayment and the drawn principal in the same tx.
        let transfers = [
            response.unwrap_ibc_transfer(),
            response.unwrap_ibc_transfer(),
        ];
        let lease = response.unwrap_response();

        assert_open_funding(app, &lease, downpayment, transfers);

        lease
    }

    fn lease_instantiate_msg(
        lease_currency: CurrencyDTO<LeaseGroup>,
        addresses: InstantiatorAddresses,
        config: InstantiatorConfig,
        max_ltd: Option<Percent>,
    ) -> NewLeaseContract {
        NewLeaseContract {
            form: NewLeaseForm {
                customer: config.customer,
                currency: lease_currency,
                max_ltd,
                position_spec: PositionSpecDTO::new(
                    Liability::new(
                        config.liability_init_percent,
                        config.liability_healthy_percent,
                        config.liability_first_liq_warn,
                        config.liability_second_liq_warn,
                        config.liability_third_liq_warn,
                        config.liability_max_percent,
                        config.liability_recalc_time,
                    ),
                    super::lpn_coin_dto(1478),
                    super::lpn_coin_dto(345),
                ),
                loan: LoanForm {
                    lpp: addresses.lpp,
                    profit: addresses.profit,
                    annual_margin_interest: config.annual_margin_interest,
                    due_period: config.lease_due_period,
                },
                reserve: addresses.reserve,
                time_alarms: addresses.time_alarms,
                market_price_oracle: addresses.oracle,
            },
            dex: config.dex,
            finalizer: addresses.finalizer,
            remote_lease_controller: addresses.remote_lease_controller,
            expected_instance_ordinal: 1,
        }
    }
}

pub(crate) struct InitConfig<D>
where
    D: Currency,
{
    lease_currency: CurrencyDTO<LeaseGroup>,
    downpayment: Coin<D>,
    max_ltd: Option<Percent>,
}

impl<D> InitConfig<D>
where
    D: Currency,
{
    pub fn new(
        lease_currency: CurrencyDTO<LeaseGroup>,
        downpayment: Coin<D>,
        max_ltd: Option<Percent>,
    ) -> Self {
        Self {
            lease_currency,
            downpayment,
            max_ltd,
        }
    }
}

pub(crate) struct InstantiatorConfig {
    //NewLeaseForm
    pub customer: Addr,
    // Liability
    pub liability_init_percent: Percent100,
    pub liability_healthy_percent: Percent100,
    pub liability_first_liq_warn: Percent100,
    pub liability_second_liq_warn: Percent100,
    pub liability_third_liq_warn: Percent100,
    pub liability_max_percent: Percent100,
    pub liability_recalc_time: Duration,
    // LoanForm
    pub annual_margin_interest: Percent100,
    pub lease_due_period: Duration,
    // Dex
    pub dex: ConnectionParams,
}

impl Default for InstantiatorConfig {
    fn default() -> Self {
        Self {
            customer: testing::user(USER),
            liability_init_percent: Percent100::from_percent(65),
            liability_healthy_percent: Percent100::from_percent(70),
            liability_first_liq_warn: Percent100::from_percent(73),
            liability_second_liq_warn: Percent100::from_percent(75),
            liability_third_liq_warn: Percent100::from_percent(78),
            liability_max_percent: Percent100::from_percent(80),
            liability_recalc_time: Duration::from_days(20),

            annual_margin_interest: Percent100::from_permille(31),
            lease_due_period: Duration::from_secs(100),

            dex: ConnectionParams {
                connection_id: "connection-0".into(),
                transfer_channel: Ics20Channel {
                    local_endpoint: "channel-0".into(),
                    remote_endpoint: "channel-2048".into(),
                },
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstantiatorAddresses {
    pub lpp: Addr,
    pub time_alarms: Addr,
    pub oracle: Addr,
    pub profit: Addr,
    pub reserve: Addr,
    pub finalizer: Addr,
    pub remote_lease_controller: Addr,
}

pub(crate) fn complete_initialization<DownpaymentC, Lpn>(
    app: &mut App,
    controller: &Addr,
    lease_addr: Addr,
    downpayment: Coin<DownpaymentC>,
    exp_borrow: Coin<Lpn>,
) where
    DownpaymentC: CurrencyDef,
    Lpn: CurrencyDef,
{
    check_state_opening(app, lease_addr.clone());

    let remote: Addr = opening_remote(app, lease_addr.clone());

    // The buy-asset swap is a remote-output swap (`StateRemoteOut`): the asset
    // stays remote (no transfer-in). `amount_out` is the FULL position the
    // counterparty reports — it folds in the coins already in the asset currency
    // (a same-currency downpayment is excluded from the swap request), so the
    // fill cannot be derived from the swapped inputs alone. Under the tests'
    // identity prices the position equals downpayment + drawn principal.
    super::swap::set_fill(
        app,
        controller,
        SwapFill::Fixed(exp_borrow.to_primitive() + downpayment.to_primitive()),
    );

    // The swap fires and finishes inline on the second funding transfer's ack,
    // driving the lease straight to `Opened`.
    () = transfer_out_and_reach_swap(app, lease_addr.clone(), remote, (downpayment, exp_borrow))
        .ignore_response()
        .unwrap_response();

    assert_lease_balance_eq(app, &lease_addr, super::native_cwcoin(0));

    check_state_opened(app, lease_addr);
}

/// Simulate the completion of the two funding transfers (downpayment and
/// drawn principal) the lease emits on the `OpenLease` ack, advancing it to
/// the buy-asset swap. With the controller stand-in in `Ok` mode the swap ack
/// arrives inline and the returned response is post-swap; callers that need to
/// observe the swap-pending state set the controller's swap op to
/// `ResponseMode::Delayed` beforehand.
pub(crate) fn transfer_out_and_reach_swap<DownpaymentC, Lpn>(
    app: &mut App,
    lease_addr: Addr,
    remote: Addr,
    (exp_downpayment, exp_borrow): (Coin<DownpaymentC>, Coin<Lpn>),
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    DownpaymentC: CurrencyDef,
    Lpn: CurrencyDef,
{
    () = ibc::do_transfer(
        app,
        lease_addr.clone(),
        remote.clone(),
        false,
        &coin_legacy::to_cosmwasm_on_nolus(exp_downpayment),
    )
    .ignore_response()
    .unwrap_response();

    ibc::do_transfer(
        app,
        lease_addr,
        remote,
        false,
        &coin_legacy::to_cosmwasm_on_nolus(exp_borrow),
    )
}

/// Assert the two funding transfers the lease emits on the `OpenLease` ack —
/// the downpayment and the drawn principal, both from the lease to its minted
/// StubPda — and return that remote account.
#[track_caller]
pub(crate) fn assert_open_funding<D>(
    app: &App,
    lease: &Addr,
    downpayment: Coin<D>,
    [downpayment_trx, borrow_trx]: [(String, String, String, CwCoin); 2],
) -> Addr
where
    D: CurrencyDef,
{
    let (remote, principal) = opening_remote_and_principal(app, lease.clone());

    for (channel, sender, receiver, _token) in [&downpayment_trx, &borrow_trx] {
        assert_eq!(channel.as_str(), TestCase::LEASER_IBC_CHANNEL);
        assert_eq!(sender.as_str(), lease.as_str());
        assert_eq!(receiver.as_str(), remote.as_str());
    }

    assert_eq!(
        downpayment_trx.3,
        coin_legacy::to_cosmwasm_on_nolus(downpayment)
    );
    assert_eq!(borrow_trx.3, coin_legacy::to_cosmwasm_on_nolus(principal));

    remote
}

#[track_caller]
fn opening_remote(app: &App, lease: Addr) -> Addr {
    opening_remote_and_principal(app, lease).0
}

#[track_caller]
fn opening_remote_and_principal(app: &App, lease: Addr) -> (Addr, Coin<Lpn>) {
    match fetch_state(app, lease) {
        StateResponse::Opening {
            loan, in_progress, ..
        } => {
            let remote = match in_progress {
                OpeningOngoingTrx::TransferOut { remote_lease }
                | OpeningOngoingTrx::BuyAsset { remote_lease } => {
                    Addr::unchecked(String::from(remote_lease))
                }
                other => panic!("expected a funded opening trx, got {other:?}"),
            };
            (
                remote,
                Coin::<Lpn>::try_from(loan).expect("drawn principal is an Lpn amount"),
            )
        }
        other => panic!("expected StateResponse::Opening, got {other:?}"),
    }
}

pub(crate) fn assert_lease_balance_eq(app: &App, lease: &Addr, balance: CwCoin) {
    assert_eq!(
        super::query_all_balances(lease, app.query()),
        (!balance.amount.is_zero()).then_some(balance).as_slice(),
    );
}

#[track_caller]
pub(crate) fn fetch_state(app: &App, lease: Addr) -> StateResponse {
    app.query()
        .query_wasm_smart(
            lease,
            &QueryMsg::State {
                due_projection: Seconds::default(),
            },
        )
        .unwrap()
}

#[track_caller]
fn check_state_opening(app: &mut App, lease: Addr) {
    if !matches!(fetch_state(app, lease), StateResponse::Opening { .. }) {
        panic!("Opening lease failed! Lease is expected to be in opening state!");
    }
}

#[track_caller]
fn check_state_opened(app: &mut App, lease: Addr) {
    if !matches!(fetch_state(app, lease), StateResponse::Opened { .. }) {
        panic!("Opening lease failed! Lease is not yet it opened state!");
    }
}
