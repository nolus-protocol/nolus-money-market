use std::marker::PhantomData;

use currencies::Lpns;
use currency::{Currency, CurrencyDef, MemberOf};
use finance::percent::{bound::BoundToHundredPercent, Percent};
use lpp::borrow::InterestRate;
use platform::ica::OpenAckVersion;
use profit::{
    msg::{ConfigResponse as ProfitConfigResponse, QueryMsg as ProfitQueryMsg},
    typedefs::CadenceHours,
};
use sdk::{
    cosmwasm_std::{self, Addr, Coin as CwCoin},
    cw_multi_test::next_block,
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};

use crate::common::{
    cwcoin, cwcoin_dex,
    leaser::{Alarms, Instantiator as LeaserInstantiator},
    lpp::Instantiator as LppInstantiator,
    oracle::Instantiator as OracleInstantiator,
    profit::Instantiator as ProfitInstantiator,
    protocols::{Instantiator as ProtocolsInstantiator, Registry},
    reserve::Instantiator as ReserveInstantiator,
    test_case::{OptionalLppEndpoints, OptionalOracleWrapper, TestCase},
    timealarms::Instantiator as TimeAlarmsInstantiator,
    treasury::Instantiator as TreasuryInstantiator,
};

pub(crate) type BlankBuilder<Lpn> = Builder<Lpn, (), (), (), (), (), (), (), ()>;

pub(crate) struct Builder<
    Lpn,
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
> {
    test_case:
        TestCase<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>,
    _lpn: PhantomData<Lpn>,
}

impl<Lpn> BlankBuilder<Lpn>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
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

impl<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn into_generic(
        self,
    ) -> TestCase<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    {
        self.test_case
    }
}

impl<Lpn, Dispatcher, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Lpn, (), Dispatcher, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_protocols_registry(
        self,
        registry: Registry,
    ) -> Builder<Lpn, Addr, Dispatcher, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms> {
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

impl<Lpn, Profit, Reserve, Leaser, Lpp, Oracle>
    Builder<Lpn, Addr, (), Profit, Reserve, Leaser, Lpp, Oracle, Addr>
where
    Lpn: Currency,
{
    pub fn init_treasury(
        self,
    ) -> Builder<Lpn, Addr, Addr, Profit, Reserve, Leaser, Lpp, Oracle, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let treasury_addr: Addr = TreasuryInstantiator::instantiate(
            &mut test_case.app,
            test_case.address_book.protocols_registry().clone(),
            test_case.address_book.time_alarms().clone(),
        );

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

impl<Lpn, ProtocolsRegistry, Reserve, Leaser, Lpp>
    Builder<Lpn, ProtocolsRegistry, Addr, (), Reserve, Leaser, Lpp, Addr, Addr>
where
    Lpn: Currency,
{
    pub fn init_profit(
        self,
        cadence_hours: CadenceHours,
    ) -> Builder<Lpn, ProtocolsRegistry, Addr, Addr, Reserve, Leaser, Lpp, Addr, Addr> {
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

        let profit_ica_addr: Addr = TestCase::ica_addr(&profit_addr, TestCase::PROFIT_ICA_ID);

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
        test_case: &mut TestCase<ProtocolsRegistry, Addr, (), Reserve, Leaser, Lpp, Addr, Addr>,
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
        test_case: &mut TestCase<ProtocolsRegistry, Addr, (), Reserve, Leaser, Lpp, Addr, Addr>,
        profit_addr: Addr,
    ) {
        let ica_addr: Addr = TestCase::ica_addr(&profit_addr, TestCase::PROFIT_ICA_ID);

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

impl<Lpn, ProtocolsRegistry, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Lpn, ProtocolsRegistry, Treasury, Profit, (), Leaser, Lpp, Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_reserve(
        self,
    ) -> Builder<Lpn, ProtocolsRegistry, Treasury, Profit, Addr, Leaser, Lpp, Oracle, TimeAlarms>
    {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let reserve_addr = ReserveInstantiator::instantiate(
            &mut test_case.app,
            test_case.address_book.lease_code(),
        );

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_reserve(reserve_addr),
            },
            _lpn,
        }
    }
}

impl<Lpn, Treasury> Builder<Lpn, Addr, Treasury, Addr, Addr, (), Addr, Addr, Addr>
where
    Lpn: Currency,
{
    pub fn init_leaser(self) -> Builder<Lpn, Addr, Treasury, Addr, Addr, Addr, Addr, Addr, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let leaser_addr = LeaserInstantiator::instantiate(
            &mut test_case.app,
            test_case.address_book.lease_code(),
            test_case.address_book.lpp().clone(),
            Alarms {
                time_alarm: test_case.address_book.time_alarms().clone(),
                market_price_oracle: test_case.address_book.oracle().clone(),
            },
            test_case.address_book.profit().clone(),
            test_case.address_book.reserve().clone(),
            test_case.address_book.protocols_registry().clone(),
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

impl<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Oracle, TimeAlarms>
    Builder<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, (), Oracle, TimeAlarms>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    pub fn init_lpp(
        self,
        custom_wrapper: OptionalLppEndpoints,
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
        min_utilization: BoundToHundredPercent,
    ) -> Builder<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Addr, Oracle, TimeAlarms>
    {
        self.init_lpp_with_funds(
            custom_wrapper,
            &[CwCoin::new(2500u128, Lpn::bank())],
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
    ) -> Builder<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Addr, Oracle, TimeAlarms>
    {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let lease_code = test_case.address_book.lease_code();

        let borrow_rate = InterestRate::new(
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )
        .expect("Couldn't construct interest rate value!");

        #[expect(if_let_rescope)]
        // TODO remove once stop linting with the 'rust-2024-compatibility' group
        let lpp: Addr = if let Some(endpoints) = endpoints {
            LppInstantiator::instantiate::<Lpn>(
                &mut test_case.app,
                Box::new(endpoints),
                lease_code,
                init_balance,
                borrow_rate,
                min_utilization,
            )
        } else {
            LppInstantiator::instantiate_default::<Lpn>(
                &mut test_case.app,
                lease_code,
                init_balance,
                borrow_rate,
                min_utilization,
            )
        };

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_lpp(lpp),
            },
            _lpn,
        }
    }
}

impl<Lpn, Treasury, Profit, Reserve, Leaser, Lpp, TimeAlarms>
    Builder<Lpn, Addr, Treasury, Profit, Reserve, Leaser, Lpp, (), TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_oracle(
        self,
        custom_wrapper: OptionalOracleWrapper,
    ) -> Builder<Lpn, Addr, Treasury, Profit, Reserve, Leaser, Lpp, Addr, TimeAlarms> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        #[expect(if_let_rescope)]
        // TODO remove once stop linting with the 'rust-2024-compatibility' group
        let oracle_addr: Addr = if let Some(contract) = custom_wrapper {
            OracleInstantiator::instantiate(
                &mut test_case.app,
                Box::new(contract),
                Some(test_case.address_book.protocols_registry().clone()),
            )
        } else {
            OracleInstantiator::instantiate_default(&mut test_case.app)
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

impl<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle>
    Builder<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, ()>
where
    Lpn: Currency,
{
    pub fn init_time_alarms(
        self,
    ) -> Builder<Lpn, ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, Addr> {
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
