use currency::{self, Currency};
use finance::{coin::Coin, duration::Duration, liability::Liability, percent::Percent};
use lease::{
    api::{
        ConnectionParams, Ics20Channel, InterestPaymentSpec, LoanForm, NewLeaseContract,
        NewLeaseForm, PositionSpecDTO, StateQuery, StateResponse,
    },
    contract::{execute, instantiate, query, reply, sudo},
};
use platform::{
    coin_legacy::{self, to_cosmwasm},
    contract::CodeId,
};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin},
    cw_multi_test::AppResponse,
    neutron_sdk::sudo::msg::SudoMsg,
};
use swap::trx as swap_trx;

use super::{
    ibc,
    test_case::{
        app::App,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
        TestCase,
    },
    CwContractWrapper, ADMIN, USER,
};

pub(crate) struct Instantiator;

impl Instantiator {
    pub fn store(app: &mut App) -> CodeId {
        let endpoints = CwContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        app.store_code(Box::new(endpoints))
    }

    #[track_caller]
    pub fn instantiate<D>(
        app: &mut App,
        code_id: CodeId,
        addresses: InstantiatorAddresses,
        lease_config: InitConfig<'_, D>,
        config: InstantiatorConfig,
        dex_connection_id: &str,
        lease_ica_id: &str,
    ) -> Addr
    where
        D: Currency,
    {
        let msg = Self::lease_instantiate_msg(
            lease_config.lease_currency,
            addresses,
            config,
            lease_config.max_ltd,
        );

        let mut response: ResponseWithInterChainMsgs<'_, Addr> = app
            .instantiate(
                code_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[coin_legacy::to_cosmwasm(lease_config.downpayment)],
                "lease",
                None,
            )
            .unwrap();

        response.expect_register_ica(dex_connection_id, lease_ica_id);

        response.unwrap_response()
    }

    fn lease_instantiate_msg(
        lease_currency: &str,
        addresses: InstantiatorAddresses,
        config: InstantiatorConfig,
        max_ltd: Option<Percent>,
    ) -> NewLeaseContract {
        NewLeaseContract {
            form: NewLeaseForm {
                customer: config.customer,
                currency: lease_currency.into(),
                max_ltd,
                position_spec: PositionSpecDTO::new(
                    Liability::new(
                        config.liability_init_percent,
                        config.liability_delta_to_healthy_percent,
                        config.liability_delta_to_max_percent,
                        config.liability_minus_delta_to_first_liq_warn,
                        config.liability_minus_delta_to_second_liq_warn,
                        config.liability_minus_delta_to_third_liq_warn,
                        config.liability_recalc_time,
                    ),
                    super::lpn_coin(23_456_986),
                    super::lpn_coin(23_456),
                ),
                loan: LoanForm {
                    annual_margin_interest: config.annual_margin_interest,
                    lpp: addresses.lpp,
                    interest_payment: config.interest_payment,
                    profit: addresses.profit,
                },
                time_alarms: addresses.time_alarms,
                market_price_oracle: addresses.oracle,
            },
            dex: config.dex,
            finalizer: addresses.finalizer,
        }
    }
}

pub(crate) struct InitConfig<'r, D>
where
    D: Currency,
{
    lease_currency: &'r str,
    downpayment: Coin<D>,
    max_ltd: Option<Percent>,
}

impl<'r, D> InitConfig<'r, D>
where
    D: Currency,
{
    pub fn new(lease_currency: &'r str, downpayment: Coin<D>, max_ltd: Option<Percent>) -> Self {
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
    pub liability_init_percent: Percent,
    pub liability_delta_to_healthy_percent: Percent,
    pub liability_delta_to_max_percent: Percent,
    pub liability_minus_delta_to_first_liq_warn: Percent,
    pub liability_minus_delta_to_second_liq_warn: Percent,
    pub liability_minus_delta_to_third_liq_warn: Percent,
    pub liability_recalc_time: Duration,
    // LoanForm
    pub annual_margin_interest: Percent,
    pub interest_payment: InterestPaymentSpec,
    // Dex
    pub dex: ConnectionParams,
}

impl Default for InstantiatorConfig {
    fn default() -> Self {
        Self {
            customer: Addr::unchecked(USER),
            liability_init_percent: Percent::from_percent(65),
            liability_delta_to_healthy_percent: Percent::from_percent(5),
            liability_delta_to_max_percent: Percent::from_percent(10),
            liability_minus_delta_to_first_liq_warn: Percent::from_percent(2),
            liability_minus_delta_to_second_liq_warn: Percent::from_percent(3),
            liability_minus_delta_to_third_liq_warn: Percent::from_percent(2),
            liability_recalc_time: Duration::from_days(20),

            annual_margin_interest: Percent::from_percent(0), // 3.1%
            interest_payment: InterestPaymentSpec::new(
                Duration::from_secs(100),
                Duration::from_secs(10),
            ),

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
    pub finalizer: Addr,
}

pub(crate) fn complete_initialization<DownpaymentC, Lpn>(
    app: &mut App,
    connection_id: &str,
    lease_addr: Addr,
    downpayment: Coin<DownpaymentC>,
    exp_borrow: Coin<Lpn>,
) where
    DownpaymentC: Currency,
    Lpn: Currency,
{
    check_state_opening(app, lease_addr.clone());

    let ica_addr: Addr = TestCase::ica_addr(lease_addr.as_str(), TestCase::LEASE_ICA_ID);
    let ica_port: String = format!("icacontroller-{ica_addr}");
    let ica_channel: String = format!("channel-{ica_addr}");

    let mut response: ResponseWithInterChainMsgs<'_, ()> = confirm_ica_and_transfer_funds(
        app,
        lease_addr.clone(),
        connection_id,
        (&ica_channel, &ica_port, ica_addr.clone()),
        (downpayment, exp_borrow),
    );

    let requests: Vec<swap_trx::RequestMsg> = super::swap::expect_swap(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    () = response.unwrap_response();

    check_state_opening(app, lease_addr.clone());

    assert_eq!(
        app.query()
            .query_all_balances(lease_addr.clone())
            .unwrap()
            .as_slice(),
        [],
    );

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
) -> ResponseWithInterChainMsgs<'r, ()>
where
    DownpaymentC: Currency,
    Lpn: Currency,
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

    () = response.unwrap_response();

    assert_eq!(downpayment, to_cosmwasm(exp_downpayment));

    check_state_opening(app, lease_addr.clone());

    let mut response: ResponseWithInterChainMsgs<'_, ()> = ibc::do_transfer(
        app,
        lease_addr.clone(),
        ica_addr.clone(),
        false,
        &downpayment,
    )
    .ignore_response();

    let borrow: CwCoin = ibc::expect_transfer(
        &mut response,
        TestCase::LEASER_IBC_CHANNEL,
        lease_addr.as_str(),
        ica_addr.as_str(),
    );

    () = response.unwrap_response();

    assert_eq!(borrow, to_cosmwasm(exp_borrow));

    check_state_opening(app, lease_addr.clone());

    ibc::do_transfer(app, lease_addr, ica_addr, false, &borrow).ignore_response()
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

#[track_caller]
fn fetch_state(app: &mut App, lease: Addr) -> StateResponse {
    app.query().query_wasm_smart(lease, &StateQuery {}).unwrap()
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
