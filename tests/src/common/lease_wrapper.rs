use currency::{
    self,
    lpn::{Lpns, Usdc},
    Currency,
};
use finance::{
    coin::{Amount, Coin, CoinDTO},
    duration::Duration,
    liability::dto::LiabilityDTO,
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
    cosmwasm_std::{to_binary, Addr, Binary, Coin as CwCoin, QueryRequest, WasmQuery},
    cw_multi_test::{AppResponse, Executor},
    neutron_sdk::{
        bindings::msg::NeutronMsg,
        sudo::msg::{RequestPacket, SudoMsg},
    },
    testing::WrappedCustomMessageReceiver,
};
use swap::trx as swap_trx;

use crate::common::cwcoin;

use super::{ContractWrapper, MockApp, ADMIN, USER};

type LpnCoin = Coin<Usdc>;
pub type LpnCoinDTO = CoinDTO<Lpns>;

pub struct LeaseInitConfig<'r, D>
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

pub struct LeaseWrapper {
    contract_wrapper: LeaseContractWrapperReply,
}

pub struct LeaseWrapperConfig {
    //NewLeaseForm
    pub customer: Addr,
    // Liability
    pub liability_init_percent: Percent,
    pub liability_delta_to_healthy_percent: Percent,
    pub liability_delta_to_max_percent: Percent,
    pub liability_minus_delta_to_first_liq_warn: Percent,
    pub liability_minus_delta_to_second_liq_warn: Percent,
    pub liability_minus_delta_to_third_liq_warn: Percent,
    pub liability_min_liquidation: LpnCoinDTO,
    pub liability_min_asset: LpnCoinDTO,
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
            liability_min_liquidation: LpnCoinDTO::from(LpnCoin::new(10_000)),
            liability_min_asset: LpnCoinDTO::from(LpnCoin::new(15_000_000)),
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
    pub fn store(self, app: &mut MockApp) -> u64 {
        app.store_code(self.contract_wrapper)
    }

    #[track_caller]
    pub fn instantiate<D>(
        self,
        app: &mut MockApp,
        code_id: Option<u64>,
        addresses: LeaseWrapperAddresses,
        lease_config: LeaseInitConfig<'_, D>,
        config: LeaseWrapperConfig,
    ) -> Addr
    where
        D: Currency,
    {
        let code_id = match code_id {
            Some(id) => id,
            None => app.store_code(self.contract_wrapper),
        };

        let msg = Self::lease_instantiate_msg(
            lease_config.lease_currency,
            addresses,
            config,
            lease_config.max_ltd,
        );

        let result = app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[coin_legacy::to_cosmwasm(lease_config.downpayment)],
            "lease",
            None,
        );

        if let Err(error) = result.as_ref() {
            eprintln!("Error: {:?}", error);

            if let Some(source) = error.source() {
                eprintln!("Source Error: {:?}", source);
            }
        }

        result.unwrap()
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
                liability: LiabilityDTO::new(
                    config.liability_init_percent,
                    config.liability_delta_to_healthy_percent,
                    config.liability_delta_to_max_percent,
                    config.liability_minus_delta_to_first_liq_warn,
                    config.liability_minus_delta_to_second_liq_warn,
                    config.liability_minus_delta_to_third_liq_warn,
                    config.liability_min_liquidation,
                    config.liability_min_asset,
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

pub fn complete_lease_initialization<Lpn, DownpaymentC, LeaseC>(
    mock_app: &mut MockApp,
    message_receiver: &WrappedCustomMessageReceiver,
    lease: &Addr,
    downpayment: Coin<DownpaymentC>,
    exp_borrow: Coin<Lpn>,
    exp_lease: Coin<LeaseC>,
) where
    Lpn: Currency,
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    check_state_opening(mock_app, lease);

    let ica_addr = "ica0";
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_port = ica_port.as_str();
    let ica_channel = format!("channel-{ica_addr}");
    let ica_channel = ica_channel.as_str();
    let (connection_id, interchain_account_id) = open_ica(
        message_receiver,
        mock_app,
        lease,
        ica_channel,
        ica_port,
        ica_addr,
    );
    check_state_opening(mock_app, lease);

    transfer_out(
        message_receiver,
        mock_app,
        lease,
        downpayment,
        ica_channel,
        ica_addr,
        false,
    );
    check_state_opening(mock_app, lease);

    transfer_out(
        message_receiver,
        mock_app,
        lease,
        exp_borrow,
        ica_channel,
        ica_addr,
        true,
    );
    check_state_opening(mock_app, lease);

    let exp_swap_out = if currency::equal::<DownpaymentC, LeaseC>() {
        exp_lease - price::total(downpayment, Price::identity())
    } else {
        exp_lease
    };
    swap::<DownpaymentC, LeaseC>(
        mock_app,
        message_receiver,
        lease,
        exp_swap_out,
        connection_id,
        interchain_account_id,
    );

    check_state_opened(mock_app, lease);
}

fn open_ica(
    message_receiver: &WrappedCustomMessageReceiver,
    mock_app: &mut MockApp,
    lease_addr: &Addr,
    ica_channel: &str,
    ica_port: &str,
    ica_addr: &str,
) -> (String, String) {
    let NeutronMsg::RegisterInterchainAccount {
        connection_id,
        interchain_account_id,
    } = ({
        let msg: NeutronMsg = message_receiver.try_recv().expect("Expected a Neutron message, but no was available!");

        let _ = message_receiver.try_recv().expect_err("Expected queue to be empty, but a second message has been sent!");

        msg
    }) else {
        unreachable!("Unexpected message type!")
    };

    mock_app
        .wasm_sudo(
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
        .unwrap();
    (connection_id, interchain_account_id)
}

fn transfer_out<OutC>(
    message_receiver: &WrappedCustomMessageReceiver,
    mock_app: &mut MockApp,
    lease: &Addr,
    amount_out: Coin<OutC>,
    ica_channel: &str,
    ica_addr: &str,
    last_transfer: bool,
) where
    OutC: Currency,
{
    assert_eq!(
        expect_ibc_transfer(
            message_receiver,
            ica_channel,
            lease.as_str(),
            ica_addr,
            last_transfer,
        ),
        cwcoin(amount_out)
    );
    send_blank_response(mock_app, lease);
}

fn check_state_opening(mock_app: &mut MockApp, lease: &Addr) {
    let StateResponse::Opening { .. } = mock_app.wrap().query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: lease.to_string(),
        msg: to_binary(&StateQuery {}).unwrap(),
    })).unwrap() else {
        panic!("Opening lease failed! Lease is expected to be in opening state!");
    };
}

fn check_state_opened(mock_app: &mut MockApp, lease: &Addr) {
    let StateResponse::Opened { .. } = mock_app.wrap().query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: lease.to_string(),
        msg: to_binary(&StateQuery {}).unwrap(),
    })).unwrap() else {
        panic!("Opening lease failed! Lease is not yet it opened state!");
    };
}

fn swap<DownpaymentC, LeaseC>(
    mock_app: &mut MockApp,
    message_receiver: &WrappedCustomMessageReceiver,
    lease: &Addr,
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
        } = message_receiver.try_recv().expect("Expected to receive a `SubmitTx` message but no message was available!") else {
            unreachable!("Unexpected message type!")
        };

        assert_eq!(tx_conn_id, connection_id);
        assert_eq!(tx_ica_id, interchain_account_id);
        assert_eq!(msgs.len(), amounts_out.len());
    }
    check_state_opening(mock_app, lease);

    let swap_resp = swap_exact_in_resp(amounts_out);
    send_response(mock_app, lease, swap_resp);
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

fn send_blank_response(mock_app: &mut MockApp, lease_addr: &Addr) -> AppResponse {
    send_response(mock_app, lease_addr, Default::default())
}

fn send_response(mock_app: &mut MockApp, lease_addr: &Addr, resp: Binary) -> AppResponse {
    mock_app
        .wasm_sudo(
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
    message_receiver: &WrappedCustomMessageReceiver,
    ica_channel: &str,
    sender_addr: &str,
    ica_addr: &str,
    last_msg: bool,
) -> CwCoin {
    let NeutronMsg::IbcTransfer {
        source_port, source_channel, token, sender, receiver, ..
    } = message_receiver.try_recv().unwrap() else {
        unreachable!("Unexpected message type!")
    };

    if last_msg {
        message_receiver
            .try_recv()
            .expect_err("Expected queue to be empty, but other message(s) has been sent!");
    }

    assert_eq!(&source_port, "transfer");
    assert_ne!(&source_channel, ica_channel);
    assert_eq!(&sender, sender_addr);
    assert_eq!(&receiver, ica_addr);

    token
}
