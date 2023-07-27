use osmosis_std::types::osmosis::gamm::v1beta1::MsgSwapExactAmountIn;

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
    cosmwasm_std::{to_binary, Addr, Binary, Coin as CwCoin, QueryRequest, WasmQuery},
    cw_multi_test::AppResponse,
    neutron_sdk::sudo::msg::{RequestPacket, SudoMsg},
};
use swap::trx as swap_trx;

use super::{
    cwcoin,
    test_case::{
        app::{App, Wasm as WasmTrait},
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
        TestCase,
    },
    CwContractWrapper, ADMIN, USER,
};

pub(crate) struct Instantiator;

impl Instantiator {
    pub fn store<Wasm>(app: &mut App<Wasm>) -> u64
    where
        Wasm: WasmTrait,
    {
        let endpoints = CwContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        app.store_code(Box::new(endpoints))
    }

    #[track_caller]
    pub fn instantiate<Wasm, D>(
        app: &mut App<Wasm>,
        code_id: u64,
        addresses: InstantiatorAddresses,
        lease_config: InitConfig<'_, D>,
        config: InstantiatorConfig,
        leaser_connection_id: &str,
    ) -> Addr
    where
        Wasm: WasmTrait,
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

pub(crate) fn complete_initialization<Wasm, Lpn, DownpaymentC, LeaseC>(
    app: &mut App<Wasm>,
    connection_id: &str,
    lease_addr: Addr,
    downpayment: Coin<DownpaymentC>,
    exp_borrow: Coin<Lpn>,
    exp_lease: Coin<LeaseC>,
) where
    Wasm: WasmTrait,
    Lpn: Currency,
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    check_state_opening(app, lease_addr.clone());

    let ica_addr = "ica0";
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_channel = format!("channel-{ica_addr}");

    let response: ResponseWithInterChainMsgs<'_, ()> = confirm_ica_and_transfer_funds(
        app,
        lease_addr.clone(),
        connection_id,
        (&ica_channel, &ica_port, ica_addr),
        (downpayment, exp_borrow),
    );

    expect_swap(
        response,
        connection_id,
        1 + usize::from(!currency::equal::<DownpaymentC, LeaseC>()),
    );

    check_state_opening(app, lease_addr.clone());

    assert_eq!(
        app.query().query_all_balances(lease_addr.clone()).unwrap(),
        vec![],
    );

    do_swap(
        app,
        lease_addr,
        ica_addr,
        (downpayment, exp_borrow, exp_lease),
    );
}

fn confirm_ica_and_transfer_funds<'r, Wasm, DownpaymentC, Lpn>(
    app: &'r mut App<Wasm>,
    lease_addr: Addr,
    connection_id: &str,
    (ica_channel, ica_port, ica_addr): (&str, &str, &str),
    (downpayment, exp_borrow): (Coin<DownpaymentC>, Coin<Lpn>),
) -> ResponseWithInterChainMsgs<'r, ()>
where
    Wasm: WasmTrait,
    DownpaymentC: Currency,
    Lpn: Currency,
{
    let response: ResponseWithInterChainMsgs<'_, ()> = send_open_ica_response(
        app,
        lease_addr.clone(),
        connection_id,
        ica_channel,
        ica_port,
        ica_addr,
    )
    .ignore_response();

    expect_ibc_transfer(response, &lease_addr, ica_addr, downpayment);

    check_state_opening(app, lease_addr.clone());

    let response: ResponseWithInterChainMsgs<'_, ()> = do_ibc_transfer(
        app,
        lease_addr.clone(),
        Addr::unchecked(ica_addr),
        &cwcoin(downpayment),
    )
    .ignore_response();

    expect_ibc_transfer(response, &lease_addr, ica_addr, exp_borrow);

    check_state_opening(app, lease_addr.clone());

    do_ibc_transfer(
        app,
        lease_addr,
        Addr::unchecked(ica_addr),
        &cwcoin(exp_borrow),
    )
    .ignore_response()
}

fn expect_ibc_transfer<C>(
    mut response: ResponseWithInterChainMsgs<'_, ()>,
    lease_addr: &Addr,
    ica_addr: &str,
    coin: Coin<C>,
) where
    C: Currency,
{
    response.expect_ibc_transfer(
        TestCase::LEASER_IBC_CHANNEL,
        cwcoin::<C, _>(coin),
        lease_addr.as_str(),
        ica_addr,
    );

    response.unwrap_response()
}

fn do_ibc_transfer<'r, Wasm>(
    app: &'r mut App<Wasm>,
    lease_addr: Addr,
    ica_addr: Addr,
    cw_coin: &CwCoin,
) -> ResponseWithInterChainMsgs<'r, AppResponse>
where
    Wasm: WasmTrait,
{
    app.send_tokens(lease_addr.clone(), ica_addr, std::slice::from_ref(cw_coin))
        .unwrap();

    send_blank_response(app, lease_addr)
}

fn send_open_ica_response<'r, Wasm>(
    app: &'r mut App<Wasm>,
    lease_addr: Addr,
    connection_id: &str,
    ica_channel: &str,
    ica_port: &str,
    ica_addr: &str,
) -> ResponseWithInterChainMsgs<'r, AppResponse>
where
    Wasm: WasmTrait,
{
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

fn expect_swap(
    mut response: ResponseWithInterChainMsgs<'_, ()>,
    connection_id: &str,
    remote_tx_count: usize,
) {
    response.expect_submit_tx(
        connection_id,
        "0",
        &[MsgSwapExactAmountIn::TYPE_URL; 2][..remote_tx_count],
    );

    response.unwrap_response()
}

fn do_swap<Wasm, DownpaymentC, Lpn, LeaseC>(
    app: &mut App<Wasm>,
    lease_addr: Addr,
    ica_addr: &str,
    (downpayment, exp_borrow, exp_lease): (Coin<DownpaymentC>, Coin<Lpn>, Coin<LeaseC>),
) where
    Wasm: WasmTrait,
    DownpaymentC: Currency,
    Lpn: Currency,
    LeaseC: Currency,
{
    let downpayment_equals_lease: bool = currency::equal::<DownpaymentC, LeaseC>();

    let exp_swap_out: Coin<LeaseC> = if downpayment_equals_lease {
        exp_lease - price::total(downpayment, Price::identity())
    } else {
        exp_lease
    };

    app.send_tokens(
        Addr::unchecked(ica_addr),
        Addr::unchecked(ADMIN),
        &[cwcoin(exp_borrow), cwcoin(downpayment)][..1 + usize::from(!downpayment_equals_lease)],
    )
    .unwrap();

    assert_eq!(
        app.query().query_all_balances(ica_addr).unwrap().as_slice(),
        &[cwcoin(downpayment)][..usize::from(downpayment_equals_lease)],
    );

    app.send_tokens(
        Addr::unchecked(ADMIN),
        Addr::unchecked(ica_addr),
        &[cwcoin(exp_swap_out)],
    )
    .unwrap();

    send_swap_response(
        app,
        lease_addr.clone(),
        exp_swap_out,
        downpayment_equals_lease,
    );

    check_state_opened(app, lease_addr);
}

fn send_swap_response<Wasm, LeaseC>(
    app: &mut App<Wasm>,
    lease: Addr,
    swap_out: Coin<LeaseC>,
    downpayment_equals_lease: bool,
) where
    Wasm: WasmTrait,
    LeaseC: Currency,
{
    let downpayment_equals_lease_as_amount: Amount = (!downpayment_equals_lease).into();

    let amounts_out = [
        Amount::from(swap_out) - downpayment_equals_lease_as_amount,
        downpayment_equals_lease_as_amount,
    ]
    .into_iter()
    .filter(|&amount: &Amount| amount != 0);

    send_response(
        app,
        lease,
        trx::encode_msg_responses(amounts_out.map(swap_trx::build_exact_amount_in_resp)).into(),
    )
    .ignore_response()
    .unwrap_response()
}

fn send_blank_response<Wasm>(
    app: &mut App<Wasm>,
    lease_addr: Addr,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    Wasm: WasmTrait,
{
    send_response(app, lease_addr, Default::default())
}

fn send_response<Wasm>(
    app: &mut App<Wasm>,
    lease_addr: Addr,
    resp: Binary,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    Wasm: WasmTrait,
{
    app.sudo(
        lease_addr,
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

fn fetch_state<Wasm>(app: &mut App<Wasm>, lease: Addr) -> StateResponse
where
    Wasm: WasmTrait,
{
    app.query()
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: lease.into_string(),
            msg: to_binary(&StateQuery {}).unwrap(),
        }))
        .unwrap()
}

fn check_state_opening<Wasm>(app: &mut App<Wasm>, lease: Addr)
where
    Wasm: WasmTrait,
{
    if !matches!(fetch_state(app, lease), StateResponse::Opening { .. }) {
        panic!("Opening lease failed! Lease is expected to be in opening state!");
    }
}

fn check_state_opened<Wasm>(app: &mut App<Wasm>, lease: Addr)
where
    Wasm: WasmTrait,
{
    if !matches!(fetch_state(app, lease), StateResponse::Opened { .. }) {
        panic!("Opening lease failed! Lease is not yet it opened state!");
    }
}
