use super::testing;
use currencies::{LeaseGroup, PaymentGroup};
use currency::{Currency, CurrencyDTO, CurrencyDef, DexSymbols};
use dex::{ConnectionParams, Ics20Channel};
use finance::{
    coin::Coin,
    duration::{Duration, Seconds},
    liability::Liability,
    percent::{Percent, Percent100},
};
use lease::{
    api::{
        open::{LoanForm, NewLeaseContract, NewLeaseForm, PositionSpecDTO},
        query::{QueryMsg, StateResponse},
    },
    contract,
};
use platform::{coin_legacy, contract::Code};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin},
    cw_multi_test::AppResponse,
};

use super::{
    ADMIN, CwContractWrapper, USER, ibc,
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
                &[coin_legacy::to_cosmwasm_on_nolus(lease_config.downpayment)],
                "lease",
                None,
            )
            .unwrap();

        // The synchronous `OpenLease` ack progresses the opening straight to
        // funding, which emits the downpayment transfer to the lease's
        // `LeaseAuthority`; consume it so the response unwraps clean.
        let _funding = response.take_ibc_transfer(TestCase::LEASER_IBC_CHANNEL);

        response.unwrap_response()
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

    let ica_addr: Addr = TestCase::ica_addr(&lease_addr, TestCase::LEASE_ICA_ID);

    // The last funding acknowledgment hands off to the remote swap leg; the
    // controller stand-in acknowledges each emitted `Swap` inline, so the
    // whole opening settles within this call.
    let response = fund_remote_lease(
        app,
        lease_addr.clone(),
        ica_addr.clone(),
        (downpayment, exp_borrow),
    );
    () = response.ignore_response().unwrap_response();

    assert_lease_balance_eq(app, &lease_addr, super::native_cwcoin(0));

    settle_remote_swaps_on_ica(app, controller, &lease_addr, &ica_addr);

    check_state_opened(app, lease_addr);
}

/// Mirror the remotely-acknowledged swaps onto the ICA's bank balances.
///
/// The remote-lease transport acknowledges swaps without moving any coins
/// inside the test app, while the still-ICA-based close and liquidation
/// simulations expect the swapped-out asset on the ICA account. The moved
/// amounts come verbatim from the `SwapParams` the lease emitted — the
/// stand-in pays exactly `min_out` — keeping a single source of truth.
fn settle_remote_swaps_on_ica(app: &mut App, controller: &Addr, lease: &Addr, ica_addr: &Addr) {
    super::remote_lease_controller_stub::recorded_swaps(app, controller, lease)
        .iter()
        .for_each(|params| {
            let coin_in = params.coin_in();
            app.send_tokens(
                ica_addr.clone(),
                testing::user(ADMIN),
                &[CwCoin::new(
                    coin_in.amount(),
                    coin_in.currency().into_symbol::<DexSymbols<PaymentGroup>>(),
                )],
            )
            .unwrap();

            let min_out = params.min_out();
            app.send_tokens(
                testing::user(ADMIN),
                ica_addr.clone(),
                &[CwCoin::new(
                    min_out.amount(),
                    min_out.currency().into_symbol::<DexSymbols<PaymentGroup>>(),
                )],
            )
            .unwrap();
        });
}

/// Drive the two-coin funding of the remote lease and return the response of
/// the principal's acknowledgment.
///
/// The opening's `OpenLease` ack already scheduled the downpayment transfer
/// inline, and the caller consumed it. This acknowledges the downpayment
/// (which schedules the principal), asserts the principal's amount, then
/// acknowledges it too — the last acknowledgment releases the arrival gate and
/// the opening swaps settle inline through the controller stand-in.
///
/// `ica_addr` is the holdings stand-in the funds land on inside the test app
/// (the non-base58 ICA address the opened-state simulations expect). The wire
/// receiver — the per-lease `LeaseAuthority` — rides the emitted transfer and
/// is asserted only by the dedicated funding drivers, not here.
pub(crate) fn fund_remote_lease<'r, DownpaymentC, Lpn>(
    app: &'r mut App,
    lease_addr: Addr,
    ica_addr: Addr,
    (exp_downpayment, exp_borrow): (Coin<DownpaymentC>, Coin<Lpn>),
) -> ResponseWithInterChainMsgs<'r, AppResponse>
where
    DownpaymentC: CurrencyDef,
    Lpn: CurrencyDef,
{
    let downpayment_cw: CwCoin = coin_legacy::to_cosmwasm_on_nolus(exp_downpayment);
    let borrow_cw: CwCoin = coin_legacy::to_cosmwasm_on_nolus(exp_borrow);

    let mut after_downpayment = ibc::do_transfer(
        app,
        lease_addr.clone(),
        ica_addr.clone(),
        false,
        &downpayment_cw,
    );
    let (_sender, _receiver, principal) =
        after_downpayment.take_ibc_transfer(TestCase::LEASER_IBC_CHANNEL);
    assert_eq!(principal, borrow_cw);
    () = after_downpayment.ignore_response().unwrap_response();

    check_state_opening(app, lease_addr.clone());

    ibc::do_transfer(app, lease_addr, ica_addr, false, &borrow_cw)
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
