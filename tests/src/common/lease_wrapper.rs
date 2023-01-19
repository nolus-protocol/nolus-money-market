use std::{cell::RefCell, collections::VecDeque};

use finance::{
    coin::Coin, currency::Currency, duration::Duration, liability::Liability, percent::Percent,
};
use lease::{
    api::{
        dex::{ConnectionParams, Ics20Channel},
        ExecuteMsg, InterestPaymentSpec, LoanForm, NewLeaseContract, NewLeaseForm, StateQuery, StateResponse,
    },
    contract::{execute, instantiate, query, reply, sudo},
    error::ContractError,
};
use platform::coin_legacy;
use sdk::{
    cosmwasm_std::{to_binary, Addr, Coin as CwCoin, QueryRequest, WasmQuery},
    cw_multi_test::{AppResponse, Executor},
    neutron_sdk::{
        bindings::msg::NeutronMsg,
        sudo::msg::{RequestPacket, SudoMsg},
    },
    testing::CustomMessageQueueRef,
};

use super::{ContractWrapper, MockApp, ADMIN, USER};

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
    pub fn store(self, app: &mut MockApp) -> u64 {
        app.store_code(self.contract_wrapper)
    }

    #[track_caller]
    pub fn instantiate<D>(
        self,
        app: &mut MockApp,
        code_id: Option<u64>,
        addresses: LeaseWrapperAddresses,
        lease_currency: &str,
        downpayment: Coin<D>,
        config: LeaseWrapperConfig,
    ) -> Addr
    where
        D: Currency,
    {
        let code_id = match code_id {
            Some(id) => id,
            None => app.store_code(self.contract_wrapper),
        };

        let msg = Self::lease_instantiate_msg(lease_currency, addresses, config);

        let result = app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[coin_legacy::to_cosmwasm(downpayment)],
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
    ) -> NewLeaseContract {
        NewLeaseContract {
            form: NewLeaseForm {
                customer: config.customer,
                currency: lease_currency.into(),
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

pub fn complete_lease_initialization<Lpn>(
    mock_app: &mut MockApp,
    custom_message_queue: CustomMessageQueueRef,
    lease_addr: &Addr,
    downpayment: CwCoin,
) where
    Lpn: Currency,
{
    let NeutronMsg::RegisterInterchainAccount {
        connection_id,
        interchain_account_id,
    } = ({
        let mut queue = custom_message_queue.borrow_mut();

        let msg = queue.pop_front().unwrap();

        assert!(queue.is_empty());

        msg
    }) else {
        unreachable!("Unexpected message type!")
    };

    let ica_addr = "ica0";
    let ica_port = format!("icacontroller-{ica_addr}");
    let ica_port = ica_port.as_str();
    let ica_channel = format!("channel-{ica_addr}");
    let ica_channel = ica_channel.as_str();

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

    assert_eq!(
        expect_ibc_transfer(
            &custom_message_queue,
            ica_channel,
            lease_addr.as_str(),
            ica_addr,
            false,
        ),
        downpayment
    );
    assert_eq!(
        expect_ibc_transfer(
            &custom_message_queue,
            ica_channel,
            lease_addr.as_str(),
            ica_addr,
            true,
        )
        .denom,
        Lpn::BANK_SYMBOL
    );

    send_blank_response(mock_app, lease_addr);

    {
        let NeutronMsg::SubmitTx {
            connection_id: tx_conn_id,
            interchain_account_id: tx_ica_id,
            msgs,
            ..
        } = custom_message_queue.borrow_mut().pop_front().unwrap() else {
            unreachable!("Unexpected message type!")
        };

        assert_eq!(tx_conn_id, connection_id);
        assert_eq!(tx_ica_id, interchain_account_id);
        // One for downpayment and one for LPP's funding
        assert_eq!(msgs.len(), 2);
    }

    let StateResponse::Opening { .. } = mock_app.wrap().query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: lease_addr.to_string(),
        msg: to_binary(&StateQuery {}).unwrap(),
    })).unwrap() else {
        panic!("Opening lease failed! Lease is expected to be in opening state!");
    };

    send_blank_response(mock_app, lease_addr);

    let StateResponse::Opened { .. } = mock_app.wrap().query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: lease_addr.to_string(),
        msg: to_binary(&StateQuery {}).unwrap(),
    })).unwrap() else {
        panic!("Opening lease failed! Lease is not yet it opened state!");
    };
}

fn send_blank_response(mock_app: &mut MockApp, lease_addr: &Addr) -> AppResponse {
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
                data: Default::default(),
            },
        )
        .unwrap()
}

pub fn expect_ibc_transfer(
    queue: &RefCell<VecDeque<NeutronMsg>>,
    ica_channel: &str,
    sender_addr: &str,
    ica_addr: &str,
    empty: bool,
) -> CwCoin {
    let NeutronMsg::IbcTransfer {
        source_port, source_channel, token, sender, receiver, ..
    } = queue.borrow_mut().pop_front().unwrap() else {
        unreachable!("Unexpected message type!")
    };

    assert_eq!(queue.borrow().is_empty(), empty);

    assert_eq!(&source_port, "transfer");
    assert_ne!(&source_channel, ica_channel);
    assert_eq!(&sender, sender_addr);
    assert_eq!(&receiver, ica_addr);

    token
}
