use std::marker::PhantomData;

use currency::Currency;
use finance::percent::Percent;
use lease::api::{ConnectionParams, Ics20Channel};
use platform::ica::OpenAckVersion;
use profit::msg::{ConfigResponse as ProfitConfigResponse, QueryMsg as ProfitQueryMsg};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, Uint64},
    cw_multi_test::{next_block, AppResponse},
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};

use crate::common::{
    cwcoin, dispatcher::Instantiator as DispatcherInstantiator,
    leaser::Instantiator as LeaserInstantiator, lpp::Instantiator as LppInstantiator,
    oracle::Instantiator as OracleInstantiator, profit::Instantiator as ProfitInstantiator,
    timealarms::Instantiator as TimeAlarmsInstantiator,
    treasury::Instantiator as TreasuryInstantiator,
};

use super::{
    app::{default_wasm, DefaultWasm, Wasm as WasmTrait},
    response::{RemoteChain, ResponseWithInterChainMsgs},
    OptionalLppEndpoints, OptionalOracleWrapper, TestCase,
};

pub(crate) type BlankBuilder<Wasm, Lpn> = Builder<Wasm, Lpn, (), (), (), (), (), (), ()>;

pub(crate) struct Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Wasm: WasmTrait,
{
    test_case: TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    _lpn: PhantomData<Lpn>,
}

impl<Wasm, Lpn> Builder<Wasm, Lpn, (), (), (), (), (), (), ()>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    pub fn new<WasmF>(wasm_f: WasmF) -> Self
    where
        WasmF: FnOnce() -> (Wasm, Wasm::CounterPart),
    {
        Self::with_reserve(&[cwcoin::<Lpn, _>(10_000)], wasm_f)
    }

    pub fn with_reserve<WasmF>(reserve: &[CwCoin], wasm_f: WasmF) -> Self
    where
        WasmF: FnOnce() -> (Wasm, Wasm::CounterPart),
    {
        Self {
            test_case: TestCase::with_reserve(reserve, wasm_f),
            _lpn: PhantomData,
        }
    }
}

impl<Lpn> Builder<DefaultWasm, Lpn, (), (), (), (), (), (), ()>
where
    Lpn: Currency,
{
    pub fn with_default_wasm() -> Self {
        Self::new(default_wasm)
    }

    pub fn with_reserve_and_default_wasm(reserve: &[CwCoin]) -> Self {
        Self::with_reserve(reserve, default_wasm)
    }
}

impl<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    pub fn into_generic(
        self,
    ) -> TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        self.test_case
    }
}

impl<Wasm, Lpn, Profit, Leaser> Builder<Wasm, Lpn, (), Addr, Profit, Leaser, Addr, Addr, Addr>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    pub fn init_dispatcher(
        self,
    ) -> Builder<Wasm, Lpn, Addr, Addr, Profit, Leaser, Addr, Addr, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        // Instantiate Dispatcher contract
        let dispatcher_addr: Addr = DispatcherInstantiator::instantiate(
            &mut test_case.app,
            test_case.address_book.lpp().clone(),
            test_case.address_book.oracle().clone(),
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.treasury().clone(),
        );

        test_case.app.update_block(next_block);

        let _: AppResponse = test_case
            .app
            .sudo(
                test_case.address_book.treasury().clone(),
                &treasury::msg::SudoMsg::ConfigureRewardTransfer {
                    rewards_dispatcher: dispatcher_addr.clone(),
                },
            )
            .unwrap()
            .unwrap_response();

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_dispatcher(dispatcher_addr),
            },
            _lpn,
        }
    }
}

impl<Wasm, Lpn, Dispatcher, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Wasm, Lpn, Dispatcher, (), Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    pub fn init_treasury_without_dispatcher(
        self,
    ) -> Builder<Wasm, Lpn, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        self.init_treasury(TreasuryInstantiator::new_with_no_dispatcher())
    }

    pub fn init_treasury_with_dispatcher(
        self,
        rewards_dispatcher: Addr,
    ) -> Builder<Wasm, Lpn, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        self.init_treasury(TreasuryInstantiator::new(rewards_dispatcher))
    }

    fn init_treasury(
        self,
        treasury: TreasuryInstantiator,
    ) -> Builder<Wasm, Lpn, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let treasury_addr: Addr = treasury.instantiate::<Wasm, Lpn>(&mut test_case.app);

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_treasury(treasury_addr),
            },
            _lpn,
        }
    }
}

impl<Wasm, Lpn, Dispatcher, Leaser, Lpp>
    Builder<Wasm, Lpn, Dispatcher, Addr, (), Leaser, Lpp, Addr, Addr>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    const PROFIT_CONNECTION_ID: &str = "dex-connection";

    pub fn init_profit(
        self,
        cadence_hours: u16,
    ) -> Builder<Wasm, Lpn, Dispatcher, Addr, Addr, Leaser, Lpp, Addr, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let profit_addr: Addr = ProfitInstantiator::instantiate(
            &mut test_case.app,
            cadence_hours,
            test_case.address_book.treasury().clone(),
            test_case.address_book.oracle().clone(),
            test_case.address_book.time_alarms().clone(),
        );

        test_case.app.update_block(next_block);

        Self::initialize(&mut test_case, &profit_addr, cadence_hours);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_profit(profit_addr),
            },
            _lpn,
        }
    }

    fn initialize(
        test_case: &mut TestCase<Wasm, Dispatcher, Addr, (), Leaser, Lpp, Addr, Addr>,
        profit_addr: &Addr,
        cadence_hours: u16,
    ) {
        Self::send_open_channel_response(test_case, profit_addr);

        test_case.app.update_block(next_block);

        Self::send_open_ica_response(test_case, profit_addr);

        let ProfitConfigResponse {
            cadence_hours: reported_cadence_hours,
        } = test_case
            .app
            .query()
            .query_wasm_smart(profit_addr.clone(), &ProfitQueryMsg::Config {})
            .unwrap();

        assert_eq!(reported_cadence_hours, cadence_hours);
    }

    fn send_open_channel_response(
        test_case: &mut TestCase<Wasm, Dispatcher, Addr, (), Leaser, Lpp, Addr, Addr>,
        profit_addr: &Addr,
    ) {
        let mut response: ResponseWithInterChainMsgs<'_, AppResponse> = test_case
            .app
            .sudo(
                profit_addr.clone(),
                &NeutronSudoMsg::OpenAck {
                    port_id: Self::PROFIT_CONNECTION_ID.into(),
                    channel_id: TestCase::PROFIT_ICA_CHANNEL.into(),
                    counterparty_channel_id: TestCase::PROFIT_ICA_CHANNEL.into(),
                    counterparty_version: String::new(),
                },
            )
            .unwrap();

        response.expect_register_ica(Self::PROFIT_CONNECTION_ID, "0");

        response.ignore_response().unwrap_response()
    }

    fn send_open_ica_response(
        test_case: &mut TestCase<Wasm, Dispatcher, Addr, (), Leaser, Lpp, Addr, Addr>,
        profit_addr: &Addr,
    ) {
        test_case
            .app
            .sudo(
                profit_addr.clone(),
                &NeutronSudoMsg::OpenAck {
                    port_id: "ica-port".into(),
                    channel_id: TestCase::PROFIT_ICA_CHANNEL.into(),
                    counterparty_channel_id: TestCase::PROFIT_ICA_CHANNEL.into(),
                    counterparty_version: serde_json_wasm::to_string(&OpenAckVersion {
                        version: "1".into(),
                        controller_connection_id: Self::PROFIT_CONNECTION_ID.into(),
                        host_connection_id: "DEADCODE".into(),
                        address: TestCase::PROFIT_ICA_ADDR.into(),
                        encoding: "DEADCODE".into(),
                        tx_type: "DEADCODE".into(),
                    })
                    .unwrap(),
                },
            )
            .unwrap()
            .ignore_response()
            .unwrap_response()
    }
}

impl<Wasm, Lpn, Dispatcher, Treasury>
    Builder<Wasm, Lpn, Dispatcher, Treasury, Addr, (), Addr, Addr, Addr>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    pub fn init_leaser(
        self,
    ) -> Builder<Wasm, Lpn, Dispatcher, Treasury, Addr, Addr, Addr, Addr, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let leaser_addr = LeaserInstantiator::instantiate(
            &mut test_case.app,
            test_case.address_book.lease_code_id(),
            test_case.address_book.lpp().clone(),
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.oracle().clone(),
            test_case.address_book.profit().clone(),
        );

        () = test_case
            .app
            .sudo(
                leaser_addr.clone(),
                &leaser::msg::SudoMsg::SetupDex(ConnectionParams {
                    connection_id: TestCase::LEASER_CONNECTION_ID.into(),
                    transfer_channel: Ics20Channel {
                        local_endpoint: TestCase::LEASER_IBC_CHANNEL.into(),
                        remote_endpoint: "channel-422".into(),
                    },
                }),
            )
            .unwrap()
            .ignore_response()
            .unwrap_response();

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_leaser(leaser_addr),
            },
            _lpn,
        }
    }
}

impl<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Oracle, TimeAlarms>
    Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, (), Oracle, TimeAlarms>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    pub fn init_lpp(
        self,
        custom_wrapper: OptionalLppEndpoints,
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Addr, Oracle, TimeAlarms> {
        self.init_lpp_with_funds(
            custom_wrapper,
            &[CwCoin::new(400, Lpn::BANK_SYMBOL)],
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )
    }

    pub fn init_lpp_with_funds(
        self,
        endpoints: OptionalLppEndpoints,
        init_balance: &[CwCoin],
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Addr, Oracle, TimeAlarms> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let lease_code_id: Uint64 = Uint64::new(test_case.address_book.lease_code_id());

        let lpp_addr: Addr = if let Some(endpoints) = endpoints {
            LppInstantiator::instantiate::<Wasm, Lpn>(
                &mut test_case.app,
                Box::new(endpoints),
                lease_code_id,
                init_balance,
                base_interest_rate,
                utilization_optimal,
                addon_optimal_interest_rate,
            )
        } else {
            LppInstantiator::instantiate_default::<Wasm, Lpn>(
                &mut test_case.app,
                lease_code_id,
                init_balance,
                base_interest_rate,
                utilization_optimal,
                addon_optimal_interest_rate,
            )
        }
        .0;

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_lpp(lpp_addr),
            },
            _lpn,
        }
    }
}

impl<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>
    Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, (), TimeAlarms>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    pub fn init_oracle(
        self,
        custom_wrapper: OptionalOracleWrapper,
    ) -> Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let oracle_addr: Addr = if let Some(contract) = custom_wrapper {
            OracleInstantiator::instantiate::<Wasm, Lpn>(&mut test_case.app, Box::new(contract))
        } else {
            OracleInstantiator::instantiate_default::<Wasm, Lpn>(&mut test_case.app)
        };

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_oracle(oracle_addr),
            },
            _lpn,
        }
    }
}

impl<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>
    Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, ()>
where
    Wasm: WasmTrait,
    Lpn: Currency,
{
    pub fn init_time_alarms(
        self,
    ) -> Builder<Wasm, Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let time_alarms_addr: Addr = TimeAlarmsInstantiator::instantiate(&mut test_case.app);

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_time_alarms(time_alarms_addr),
            },
            _lpn,
        }
    }
}
