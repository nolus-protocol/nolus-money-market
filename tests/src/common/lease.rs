use currencies::{LeaseGroup, PaymentGroup};
use currency::{Currency, CurrencyDTO, CurrencyDef};
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
    api::SudoMsg,
    cosmwasm_std::{Addr, Coin as CwCoin},
    cw_multi_test::AppResponse,
    testing,
};
use swap::testing::SwapRequest;

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
        dex_connection_id: &str,
        lease_ica_id: &str,
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

        response.expect_register_ica(dex_connection_id, lease_ica_id);

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
}

pub(crate) fn complete_initialization<DownpaymentC, Lpn>(
    app: &mut App,
    connection_id: &str,
    lease_addr: Addr,
    downpayment: Coin<DownpaymentC>,
    exp_borrow: Coin<Lpn>,
) where
    DownpaymentC: CurrencyDef,
    Lpn: CurrencyDef,
{
    check_state_opening(app, lease_addr.clone());

    let ica_addr: Addr = TestCase::ica_addr(&lease_addr, TestCase::LEASE_ICA_ID);
    let ica_port: String = format!("icacontroller-{ica_addr}");
    let ica_channel: String = format!("channel-{ica_addr}");

    let response = confirm_ica_and_transfer_funds(
        app,
        lease_addr.clone(),
        connection_id,
        (&ica_channel, &ica_port, ica_addr.clone()),
        (downpayment, exp_borrow),
    );

    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = super::swap::expect_swap(
        response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
        |_| {},
    );

    check_state_opening(app, lease_addr.clone());

    assert_lease_balance_eq(app, &lease_addr, super::native_cwcoin(0));

    () = super::swap::do_swap(
        app,
        lease_addr.clone(),
        ica_addr,
        requests.into_iter(),
        |price, _, _| price,
    )
    .ignore_response()
    .unwrap_response();

    check_state_opened(app, lease_addr);
}

fn confirm_ica_and_transfer_funds<'r, DownpaymentC, Lpn>(
    app: &'r mut App,
    lease_addr: Addr,
    connection_id: &str,
    (ica_channel, ica_port, ica_addr): (&str, &str, Addr),
    (exp_downpayment, exp_borrow): (Coin<DownpaymentC>, Coin<Lpn>),
) -> ResponseWithInterChainMsgs<'r, AppResponse>
where
    DownpaymentC: CurrencyDef,
    Lpn: CurrencyDef,
{
    let mut response: ResponseWithInterChainMsgs<'_, ()> = send_open_ica_response(
        app,
        lease_addr.clone(),
        connection_id,
        ica_channel,
        ica_port,
        ica_addr.as_str(),
    )
    .ignore_response();

    let downpayment: CwCoin = ibc::expect_transfer(
        &mut response,
        TestCase::LEASER_IBC_CHANNEL,
        lease_addr.as_str(),
        ica_addr.as_str(),
    );

    assert_eq!(
        downpayment,
        coin_legacy::to_cosmwasm_on_nolus(exp_downpayment)
    );

    let borrow: CwCoin = ibc::expect_transfer(
        &mut response,
        TestCase::LEASER_IBC_CHANNEL,
        lease_addr.as_str(),
        ica_addr.as_str(),
    );

    assert_eq!(borrow, coin_legacy::to_cosmwasm_on_nolus(exp_borrow));

    () = response.unwrap_response();

    check_state_opening(app, lease_addr.clone());

    () = ibc::do_transfer(
        app,
        lease_addr.clone(),
        ica_addr.clone(),
        false,
        &downpayment,
    )
    .ignore_response()
    .unwrap_response();

    ibc::do_transfer(app, lease_addr.clone(), ica_addr.clone(), false, &borrow)
}

fn send_open_ica_response<'r>(
    app: &'r mut App,
    lease_addr: Addr,
    connection_id: &str,
    ica_channel: &str,
    ica_port: &str,
    ica_addr: &str,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    app.sudo(
        lease_addr,
        &SudoMsg::OpenAck {
            port_id: ica_port.to_string(),
            channel_id: ica_channel.to_string(),
            counterparty_channel_id: format!("counter-{ica_channel}"),
            counterparty_version: format!(
                // TODO fill-in with real/valid `OpenAck` data
                r#"{{
                        "version":"???",
                        "controller_connection_id":"{connection_id}",
                        "host_connection_id":"???",
                        "address":"{ica_addr}",
                        "encoding":"???",
                        "tx_type":"???"
                    }}"#
            ),
        },
    )
    .unwrap()
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
