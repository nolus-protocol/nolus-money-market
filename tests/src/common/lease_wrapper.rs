use std::collections::VecDeque;

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
        ConnectionParams, ExecuteMsg, Ics20Channel, InterestPaymentSpec, LoanForm,
        NewLeaseContract, NewLeaseForm, StateQuery, StateResponse,
    },
    contract::{execute, instantiate, query, reply, sudo},
    error::ContractError,
};
use platform::{coin_legacy, trx};
use sdk::{
    cosmwasm_ext::CustomMsg,
    cosmwasm_std::{to_binary, Addr, Binary, Coin as CwCoin, QueryRequest, WasmQuery},
    cw_multi_test::{AppResponse, Executor},
    neutron_sdk::{
        bindings::msg::NeutronMsg,
        sudo::msg::{RequestPacket, SudoMsg},
    },
};
use swap::trx as swap_trx;

use super::{
    cwcoin,
    test_case::{WrappedApp, WrappedResponse},
    ContractWrapper, MockApp, ADMIN, USER,
};

pub(crate) struct LeaseInitConfig<'r, D>
where
    D: Currency,
{
    lease_currency: &'r str,
    downpayment: Coin<D>,
    max_ltd: Option<Percent>,
}

impl<'r, D> LeaseInitConfig<'r, D>
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

pub(crate) struct LeaseWrapper {
    contract_wrapper: LeaseContractWrapperReply,
}

pub(crate) struct LeaseWrapperConfig {
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

impl Default for LeaseWrapperConfig {
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

impl LeaseWrapper {
    pub fn store(self, app: &mut WrappedApp) -> u64 {
        app.store_code(self.contract_wrapper)
    }

    #[track_caller]
    pub fn instantiate<'r, D>(
        self,
        app: &'r mut WrappedApp,
        code_id: Option<u64>,
        addresses: LeaseWrapperAddresses,
        lease_config: LeaseInitConfig<'_, D>,
        config: LeaseWrapperConfig,
        leaser_connection_id: &str,
    ) -> Addr
    where
        D: Currency,
    {
        let code_id = match code_id {
            Some(id) => id,
            None => app
                .with_app(|app: &mut MockApp| Ok(app.store_code(self.contract_wrapper)))
                .unwrap()
                .unwrap_response(),
        };

        let msg = Self::lease_instantiate_msg(
            lease_config.lease_currency,
            addresses,
            config,
            lease_config.max_ltd,
        );

        let mut response: WrappedResponse<'_, Addr> = app
            .with_app(|app: &mut MockApp| {
                app.instantiate_contract(
                    code_id,
                    Addr::unchecked(ADMIN),
                    &msg,
                    &[coin_legacy::to_cosmwasm(lease_config.downpayment)],
                    "lease",
                    None,
                )
            })
            .unwrap();

        response
            .receiver()
            .assert_register_ica(leaser_connection_id);

        response.unwrap_response()
    }

    fn lease_instantiate_msg(
        lease_currency: &str,
        addresses: LeaseWrapperAddresses,
        config: LeaseWrapperConfig,
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

impl Default for LeaseWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LeaseWrapperAddresses {
    pub lpp: Addr,
    pub time_alarms: Addr,
    pub oracle: Addr,
    pub profit: Addr,
}

type LeaseContractWrapperReply = Box<
    ContractWrapper<
        ExecuteMsg,
        ContractError,
        NewLeaseContract,
        ContractError,
        StateQuery,
        ContractError,
        SudoMsg,
        ContractError,
        ContractError,
    >,
>;

pub(crate) fn complete_lease_initialization<Lpn, DownpaymentC, LeaseC>(
    wrapped_app: &mut WrappedApp,
    lease_addr: &Addr,
    messages: VecDeque<NeutronMsg>,
    downpayment: Coin<DownpaymentC>,
    exp_borrow: Coin<Lpn>,
    exp_lease: Coin<LeaseC>,
) where
    Lpn: Currency,
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    check_state_opening(wrapped_app, lease_addr);

    let ica_addr = "ica0";
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_port = ica_port.as_str();
    let ica_channel = format!("channel-{ica_addr}");
    let ica_channel = ica_channel.as_str();
    let mut response: WrappedResponse<'_, (String, String)> = open_ica(
        wrapped_app,
        lease_addr,
        messages,
        ica_channel,
        ica_port,
        ica_addr,
    );
    let messages: VecDeque<CustomMsg> = response.iter().collect();
    let (connection_id, interchain_account_id) = response.unwrap_response();
    check_state_opening(wrapped_app, lease_addr);

    let messages: VecDeque<CustomMsg> = transfer_out(
        wrapped_app,
        lease_addr,
        messages,
        downpayment,
        ica_channel,
        ica_addr,
    )
    .clear_result()
    .collect();

    check_state_opening(wrapped_app, lease_addr);

    let messages: VecDeque<CustomMsg> = transfer_out(
        wrapped_app,
        lease_addr,
        messages,
        exp_borrow,
        ica_channel,
        ica_addr,
    )
    .clear_result()
    .collect();

    check_state_opening(wrapped_app, lease_addr);

    let exp_swap_out = if currency::equal::<DownpaymentC, LeaseC>() {
        exp_lease - price::total(downpayment, Price::identity())
    } else {
        exp_lease
    };

    swap::<DownpaymentC, LeaseC>(
        wrapped_app,
        lease_addr,
        messages,
        exp_swap_out,
        connection_id,
        interchain_account_id,
    );

    check_state_opened(wrapped_app, lease_addr);
}

fn open_ica<'r>(
    wrapped_app: &'r mut WrappedApp,
    lease_addr: &Addr,
    mut messages: VecDeque<NeutronMsg>,
    ica_channel: &str,
    ica_port: &str,
    ica_addr: &str,
) -> WrappedResponse<'r, (String, String)> {
    let NeutronMsg::RegisterInterchainAccount {
        connection_id,
        interchain_account_id,
    } = ({
        let msg: NeutronMsg = messages.pop_front().unwrap();

        assert_eq!(messages.as_slices(), (&[] as &[NeutronMsg], &[] as &[NeutronMsg]));

        msg
    }) else {
        unreachable!("Unexpected message type!")
    };

    wrapped_app
        .sudo(
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
        .clear_result()
        .set_result((connection_id, interchain_account_id))
}

fn transfer_out<'r, OutC>(
    wrapped_app: &'r mut WrappedApp,
    lease: &Addr,
    messages: VecDeque<NeutronMsg>,
    amount_out: Coin<OutC>,
    ica_channel: &str,
    ica_addr: &str,
) -> WrappedResponse<'r, AppResponse>
where
    OutC: Currency,
{
    let coin = expect_ibc_transfer(messages, ica_channel, lease.as_str(), ica_addr);

    assert_eq!(coin, cwcoin(amount_out));

    send_blank_response(wrapped_app, lease)
}

fn check_state_opening(wrapped_app: &mut WrappedApp, lease: &Addr) {
    let StateResponse::Opening { .. } = wrapped_app.query().query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: lease.to_string(),
        msg: to_binary(&StateQuery {}).unwrap(),
    })).unwrap() else {
        panic!("Opening lease failed! Lease is expected to be in opening state!");
    };
}

fn check_state_opened(wrapped_app: &mut WrappedApp, lease: &Addr) {
    let StateResponse::Opened { .. } = wrapped_app.query().query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: lease.to_string(),
        msg: to_binary(&StateQuery {}).unwrap(),
    })).unwrap() else {
        panic!("Opening lease failed! Lease is not yet it opened state!");
    };
}

fn swap<DownpaymentC, LeaseC>(
    wrapped_app: &mut WrappedApp,
    lease: &Addr,
    mut messages: VecDeque<CustomMsg>,
    swap_out: Coin<LeaseC>,
    connection_id: String,
    interchain_account_id: String,
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
    {
        let NeutronMsg::SubmitTx {
            connection_id: tx_conn_id,
            interchain_account_id: tx_ica_id,
            msgs,
            ..
        } = messages.pop_front().expect("Expected to receive a `SubmitTx` message but no message was available!") else {
            unreachable!("Unexpected message type!")
        };

        assert_eq!(
            messages.as_slices(),
            (&[] as &[CustomMsg], &[] as &[CustomMsg])
        );

        assert_eq!(tx_conn_id, connection_id);
        assert_eq!(tx_ica_id, interchain_account_id);
        assert_eq!(msgs.len(), amounts_out.len());
    }
    check_state_opening(wrapped_app, lease);

    let swap_resp = swap_exact_in_resp(amounts_out);
    let _: AppResponse = send_response(wrapped_app, lease, swap_resp).unwrap_response();
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
    wrapped_app: &'r mut WrappedApp,
    lease_addr: &Addr,
) -> WrappedResponse<'r, AppResponse> {
    send_response(wrapped_app, lease_addr, Default::default())
}

fn send_response<'r>(
    wrapped_app: &'r mut WrappedApp,
    lease_addr: &Addr,
    resp: Binary,
) -> WrappedResponse<'r, AppResponse> {
    wrapped_app
        .sudo(
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

pub fn expect_ibc_transfer(
    mut messages: VecDeque<NeutronMsg>,
    ica_channel: &str,
    sender_addr: &str,
    ica_addr: &str,
) -> CwCoin {
    let NeutronMsg::IbcTransfer {
        source_port, source_channel, token, sender, receiver, ..
    } = messages.pop_front().unwrap() else {
        unreachable!("Unexpected message type!")
    };

    assert_eq!(
        messages.as_slices(),
        (&[] as &[CustomMsg], &[] as &[CustomMsg]),
        "Expected queue to be empty, but other message(s) has been sent!"
    );

    assert_eq!(&source_port, "transfer");
    assert_ne!(&source_channel, ica_channel);
    assert_eq!(&sender, sender_addr);
    assert_eq!(&receiver, ica_addr);

    token
}
