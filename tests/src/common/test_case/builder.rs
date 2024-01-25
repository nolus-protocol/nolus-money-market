use std::marker::PhantomData;

use currency::Currency;
use finance::percent::{bound::BoundToHundredPercent, Percent};
use lpp::borrow::InterestRate;
use platform::ica::OpenAckVersion;
use profit::{
    msg::{ConfigResponse as ProfitConfigResponse, QueryMsg as ProfitQueryMsg},
    typedefs::CadenceHours,
};
use sdk::{
    cosmwasm_std::{self, Addr, Coin as CwCoin, Uint64},
    cw_multi_test::{next_block, AppResponse},
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};

use crate::common::{
    cwcoin, cwcoin_dex,
    dispatcher::Instantiator as DispatcherInstantiator,
    leaser::Instantiator as LeaserInstantiator,
    lpp::Instantiator as LppInstantiator,
    oracle::Instantiator as OracleInstantiator,
    profit::Instantiator as ProfitInstantiator,
    protocols::{Instantiator as ProtocolsInstantiator, Registry},
    test_case::{OptionalLppEndpoints, OptionalOracleWrapper, TestCase},
    timealarms::Instantiator as TimeAlarmsInstantiator,
    treasury::Instantiator as TreasuryInstantiator,
};

pub(crate) type BlankBuilder<Lpn> = Builder<Lpn, (), (), (), (), (), (), (), ()>;

pub(crate) struct Builder<
    Lpn,
    ProtocolsRegistry,
    Dispatcher,
    Treasury,
    Profit,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
> {
    test_case:
        TestCase<ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    _lpn: PhantomData<Lpn>,
}

impl<Lpn> BlankBuilder<Lpn>
where
    Lpn: Currency,
{
    pub fn new() -> Self {
        Self::with_reserve(&[cwcoin::<Lpn, _>(10_000), cwcoin_dex::<Lpn, _>(10_000)])
    }

    pub fn with_reserve(reserve: &[CwCoin]) -> Self {
        Self {
            test_case: TestCase::with_reserve(reserve),
            _lpn: PhantomData,
        }
    }
}

impl<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn into_generic(
        self,
    ) -> TestCase<ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    {
        self.test_case
    }
}

impl<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Lpn, (), Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_protocols_registry(
        self,
        registry: Registry,
    ) -> Builder<Lpn, Addr, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let protocols_registry: Addr =
            ProtocolsInstantiator().instantiate(&mut test_case.app, registry);

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case
                    .address_book
                    .with_protocols_registry(protocols_registry),
            },
            _lpn,
        }
    }
}

impl<Lpn, Profit, Leaser, Lpp, Oracle>
    Builder<Lpn, Addr, (), Addr, Profit, Leaser, Lpp, Oracle, Addr>
where
    Lpn: Currency,
{
    pub fn init_dispatcher(
        self,
    ) -> Builder<Lpn, Addr, Addr, Addr, Profit, Leaser, Lpp, Oracle, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        // Instantiate Dispatcher contract
        let dispatcher_addr: Addr = DispatcherInstantiator::instantiate(
            &mut test_case.app,
            test_case.address_book.protocols_registry().clone(),
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

impl<Lpn, ProtocolsRegistry, Dispatcher, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Lpn, ProtocolsRegistry, Dispatcher, (), Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_treasury_without_dispatcher(
        self,
    ) -> Builder<Lpn, ProtocolsRegistry, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    {
        self.init_treasury(TreasuryInstantiator::new_with_no_dispatcher())
    }

    pub fn init_treasury_with_dispatcher(
        self,
        rewards_dispatcher: Addr,
    ) -> Builder<Lpn, ProtocolsRegistry, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    {
        self.init_treasury(TreasuryInstantiator::new(rewards_dispatcher))
    }

    fn init_treasury(
        self,
        treasury: TreasuryInstantiator,
    ) -> Builder<Lpn, ProtocolsRegistry, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let treasury_addr: Addr = treasury.instantiate::<Lpn>(&mut test_case.app);

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

impl<Lpn, ProtocolsRegistry, Dispatcher, Leaser, Lpp>
    Builder<Lpn, ProtocolsRegistry, Dispatcher, Addr, (), Leaser, Lpp, Addr, Addr>
where
    Lpn: Currency,
{
    pub fn init_profit(
        self,
        cadence_hours: CadenceHours,
    ) -> Builder<Lpn, ProtocolsRegistry, Dispatcher, Addr, Addr, Leaser, Lpp, Addr, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let profit_addr = ProfitInstantiator::instantiate(
            &mut test_case.app,
            cadence_hours,
            test_case.address_book.treasury().clone(),
            test_case.address_book.oracle().clone(),
            test_case.address_book.time_alarms().clone(),
        );

        Self::send_open_ica_response(&mut test_case, profit_addr.clone());
        Self::test_config(&mut test_case, profit_addr.clone(), cadence_hours);

        test_case.app.update_block(next_block);

        let profit_ica_addr: Addr =
            TestCase::ica_addr(profit_addr.as_str(), TestCase::PROFIT_ICA_ID);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case
                    .address_book
                    .with_profit(profit_addr, profit_ica_addr),
            },
            _lpn,
        }
    }

    fn test_config(
        test_case: &mut TestCase<ProtocolsRegistry, Dispatcher, Addr, (), Leaser, Lpp, Addr, Addr>,
        profit_addr: Addr,
        cadence_hours: CadenceHours,
    ) {
        let ProfitConfigResponse {
            cadence_hours: reported_cadence_hours,
        } = test_case
            .app
            .query()
            .query_wasm_smart(profit_addr, &ProfitQueryMsg::Config {})
            .unwrap();

        assert_eq!(reported_cadence_hours, cadence_hours);
    }

    fn send_open_ica_response(
        test_case: &mut TestCase<ProtocolsRegistry, Dispatcher, Addr, (), Leaser, Lpp, Addr, Addr>,
        profit_addr: Addr,
    ) {
        let ica_addr: Addr = TestCase::ica_addr(profit_addr.as_str(), TestCase::PROFIT_ICA_ID);

        test_case
            .app
            .sudo(
                profit_addr.clone(),
                &NeutronSudoMsg::OpenAck {
                    port_id: "ica-port".into(),
                    channel_id: TestCase::PROFIT_IBC_CHANNEL.into(),
                    counterparty_channel_id: TestCase::PROFIT_IBC_CHANNEL.into(),
                    counterparty_version: cosmwasm_std::to_json_string(&OpenAckVersion {
                        version: "1".into(),
                        controller_connection_id: TestCase::DEX_CONNECTION_ID.into(),
                        host_connection_id: "DEADCODE".into(),
                        address: ica_addr.into_string(),
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

impl<Lpn, ProtocolsRegistry, Dispatcher, Treasury>
    Builder<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Addr, (), Addr, Addr, Addr>
where
    Lpn: Currency,
{
    pub fn init_leaser(
        self,
    ) -> Builder<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Addr, Addr, Addr, Addr, Addr> {
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

impl<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Oracle, TimeAlarms>
    Builder<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, (), Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_lpp(
        self,
        custom_wrapper: OptionalLppEndpoints,
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
        min_utilization: BoundToHundredPercent,
    ) -> Builder<
        Lpn,
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Leaser,
        Addr,
        Oracle,
        TimeAlarms,
    > {
        self.init_lpp_with_funds(
            custom_wrapper,
            &[CwCoin::new(2500, Lpn::BANK_SYMBOL)],
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
            min_utilization,
        )
    }

    pub fn init_lpp_with_funds(
        self,
        endpoints: OptionalLppEndpoints,
        init_balance: &[CwCoin],
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
        min_utilization: BoundToHundredPercent,
    ) -> Builder<
        Lpn,
        ProtocolsRegistry,
        Dispatcher,
        Treasury,
        Profit,
        Leaser,
        Addr,
        Oracle,
        TimeAlarms,
    > {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let lease_code_id: Uint64 = Uint64::new(test_case.address_book.lease_code_id());

        let borrow_rate = InterestRate::new(
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )
        .expect("Couldn't construct interest rate value!");

        let lpp_addr: Addr = if let Some(endpoints) = endpoints {
            LppInstantiator::instantiate::<Lpn>(
                &mut test_case.app,
                Box::new(endpoints),
                lease_code_id,
                init_balance,
                borrow_rate,
                min_utilization,
            )
        } else {
            LppInstantiator::instantiate_default::<Lpn>(
                &mut test_case.app,
                lease_code_id,
                init_balance,
                borrow_rate,
                min_utilization,
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

impl<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>
    Builder<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, (), TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_oracle(
        self,
        custom_wrapper: OptionalOracleWrapper,
    ) -> Builder<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms>
    {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let oracle_addr: Addr = if let Some(contract) = custom_wrapper {
            OracleInstantiator::instantiate::<Lpn>(&mut test_case.app, Box::new(contract))
        } else {
            OracleInstantiator::instantiate_default::<Lpn>(&mut test_case.app)
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

impl<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>
    Builder<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, ()>
where
    Lpn: Currency,
{
    pub fn init_time_alarms(
        self,
    ) -> Builder<Lpn, ProtocolsRegistry, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>
    {
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
