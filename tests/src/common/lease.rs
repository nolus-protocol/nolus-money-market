use currency::{self, Currency};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    liability::Liability,
    percent::Percent,
    price::{self, Price},
};
use lease::{
    api::{
        ConnectionParams, Ics20Channel, InterestPaymentSpec, LoanForm, NewLeaseContract,
        NewLeaseForm, StateQuery, StateResponse,
    },
    contract::{execute, instantiate, query, reply, sudo},
};
use platform::{coin_legacy, trx};
use sdk::{
    cosmwasm_std::{to_binary, Addr, Binary, QueryRequest, WasmQuery},
    cw_multi_test::AppResponse,
    neutron_sdk::sudo::msg::{RequestPacket, SudoMsg},
};
use swap::trx as swap_trx;

use super::{
    cwcoin,
    test_case::{
        app::App,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
        TestCase,
    },
    CwContractWrapper, ADMIN, USER,
};

pub(crate) struct Instantiator;

impl Instantiator {
    pub fn store(app: &mut App) -> u64 {
        let endpoints = CwContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        app.store_code(Box::new(endpoints))
    }

    #[track_caller]
    pub fn instantiate<D>(
        app: &mut App,
        code_id: u64,
        addresses: InstantiatorAddresses,
        lease_config: InitConfig<'_, D>,
        config: InstantiatorConfig,
        leaser_connection_id: &str,
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

        response.expect_register_ica(leaser_connection_id, "0");

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
                liability: Liability::new(
                    config.liability_init_percent,
                    config.liability_delta_to_healthy_percent,
                    config.liability_delta_to_max_percent,
                    config.liability_minus_delta_to_first_liq_warn,
                    config.liability_minus_delta_to_second_liq_warn,
                    config.liability_minus_delta_to_third_liq_warn,
                    config.liability_recalc_time,
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
}

pub(crate) fn complete_initialization<Lpn, DownpaymentC, LeaseC>(
    app: &mut App,
    connection_id: &str,
    lease_addr: &Addr,
    downpayment: Coin<DownpaymentC>,
    exp_borrow: Coin<Lpn>,
    exp_lease: Coin<LeaseC>,
) where
    Lpn: Currency,
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    check_state_opening(app, lease_addr);

    let ica_addr = "ica0";
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_channel = format!("channel-{ica_addr}");

    send_ibc_responses(
        app,
        lease_addr,
        connection_id,
        (&ica_channel, &ica_port, ica_addr),
        downpayment,
        exp_borrow,
    );

    do_swap(app, lease_addr, connection_id, downpayment, exp_lease);
}

fn do_swap<DownpaymentC, LeaseC>(
    app: &mut App,
    lease_addr: &Addr,
    connection_id: &str,
    downpayment: Coin<DownpaymentC>,
    exp_lease: Coin<LeaseC>,
) where
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    let mut response: ResponseWithInterChainMsgs<'_, ()> =
        send_blank_response(app, lease_addr).ignore_response();

    let remote_tx_count: usize = 1 + usize::from(!currency::equal::<DownpaymentC, LeaseC>());

    response.expect_submit_tx(connection_id, "0", remote_tx_count);

    () = response.unwrap_response();

    check_state_opening(app, lease_addr);

    let exp_swap_out = if currency::equal::<DownpaymentC, LeaseC>() {
        exp_lease - price::total(downpayment, Price::identity())
    } else {
        exp_lease
    };

    send_swap_response::<DownpaymentC, LeaseC>(app, lease_addr, exp_swap_out, remote_tx_count);

    check_state_opened(app, lease_addr);
}

fn send_ibc_responses<DownpaymentC, Lpn>(
    app: &mut App,
    lease_addr: &Addr,
    connection_id: &str,
    (ica_channel, ica_port, ica_addr): (&str, &str, &str),
    downpayment: Coin<DownpaymentC>,
    exp_borrow: Coin<Lpn>,
) where
    DownpaymentC: Currency,
    Lpn: Currency,
{
    send_response_and_expect(
        app,
        |app: &mut App| {
            send_open_ica_response(
                app,
                lease_addr,
                connection_id,
                ica_channel,
                ica_port,
                ica_addr,
            )
        },
        lease_addr,
        ica_addr,
        downpayment,
    );

    send_response_and_expect(
        app,
        |app: &mut App| send_blank_response(app, lease_addr),
        lease_addr,
        ica_addr,
        exp_borrow,
    );
}

fn send_response_and_expect<F, C>(
    app: &mut App,
    send_response: F,
    lease_addr: &Addr,
    ica_addr: &str,
    coin: Coin<C>,
) where
    F: for<'t> FnOnce(&'t mut App) -> ResponseWithInterChainMsgs<'t, AppResponse>,
    C: Currency,
{
    let mut response: ResponseWithInterChainMsgs<'_, ()> = send_response(app).ignore_response();

    response.expect_ibc_transfer(
        TestCase::LEASER_IBC_CHANNEL,
        cwcoin::<C, _>(coin),
        lease_addr.as_str(),
        ica_addr,
    );

    () = response.unwrap_response();

    check_state_opening(app, lease_addr);
}

fn send_open_ica_response<'r>(
    app: &'r mut App,
    lease_addr: &Addr,
    connection_id: &str,
    ica_channel: &str,
    ica_port: &str,
    ica_addr: &str,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    app.sudo(
        Addr::unchecked(lease_addr),
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

fn fetch_state(app: &mut App, lease: &Addr) -> StateResponse {
    app.query()
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: lease.to_string(),
            msg: to_binary(&StateQuery {}).unwrap(),
        }))
        .unwrap()
}

fn check_state_opening(app: &mut App, lease: &Addr) {
    if !matches!(fetch_state(app, lease), StateResponse::Opening { .. }) {
        panic!("Opening lease failed! Lease is expected to be in opening state!");
    }
}

fn check_state_opened(app: &mut App, lease: &Addr) {
    if !matches!(fetch_state(app, lease), StateResponse::Opened { .. }) {
        panic!("Opening lease failed! Lease is not yet it opened state!");
    }
}

fn send_swap_response<DownpaymentC, LeaseC>(
    app: &mut App,
    lease: &Addr,
    swap_out: Coin<LeaseC>,
    expected_amounts_count: usize,
) where
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    let amounts_out: Vec<Amount> = if currency::equal::<DownpaymentC, LeaseC>() {
        vec![swap_out.into()]
    } else {
        let downpayment_out = 1;
        let borrow_amount = Into::<Amount>::into(swap_out) - downpayment_out;
        vec![downpayment_out, borrow_amount]
    };

    assert_eq!(amounts_out.len(), expected_amounts_count);

    check_state_opening(app, lease);

    let swap_resp = swap_exact_in_resp(amounts_out);

    () = send_response(app, lease, swap_resp)
        .ignore_response()
        .unwrap_response();
}

fn swap_exact_in_resp<I>(amounts: I) -> Binary
where
    I: IntoIterator<Item = Amount>,
{
    let msgs = amounts
        .into_iter()
        .map(swap_trx::build_exact_amount_in_resp);
    trx::encode_msg_responses(msgs).into()
}

fn send_blank_response<'r>(
    app: &'r mut App,
    lease_addr: &Addr,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    send_response(app, lease_addr, Default::default())
}

fn send_response<'r>(
    app: &'r mut App,
    lease_addr: &Addr,
    resp: Binary,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    app.sudo(
        Addr::unchecked(lease_addr),
        &SudoMsg::Response {
            // TODO fill-in with real/valid response data
            request: RequestPacket {
                sequence: None,
                source_port: None,
                source_channel: None,
                destination_port: None,
                destination_channel: None,
                data: None,
                timeout_height: None,
                timeout_timestamp: None,
            },
            data: resp,
        },
    )
    .unwrap()
}
