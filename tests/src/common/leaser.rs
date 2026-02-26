use std::collections::HashSet;

use currencies::{LeaseGroup, Lpn, PaymentGroup};
use currency::{CurrencyDef, MemberOf};
use dex::{ConnectionParams, Ics20Channel, MaxSlippage};
use finance::{
    coin::Coin, duration::Duration, fraction::Unit, liability::Liability, percent::{Percent, Percent100}, test
};
use lease::api::{LpnCoinDTO, limits::MaxSlippages, open::PositionSpecDTO};
use leaser::msg::{InstantiateMsg, NewConfig, QueryMsg, QuoteResponse};
use platform::contract::{Code, CodeId};
use sdk::{cosmwasm_std::Addr, testing};

use super::{
    ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, CwContractWrapper, LEASE_ADMIN,
    UTILIZATION_OPTIMAL,
    protocols::Registry,
    test_case::{TestCase, app::App, builder::BlankBuilder as TestCaseBuilder},
};

pub(crate) struct Instantiator;

impl Instantiator {
    pub const INTEREST_RATE_MARGIN: Percent100 = Percent100::from_permille(30);

    pub const REPAYMENT_PERIOD: Duration = Duration::from_days(90);

    pub const INITIAL_LTV: Percent100 = Percent100::from_permille(650);
    pub const FIRST_LIQ_WARN: Percent100 = Percent100::from_permille(730);
    pub const SECOND_LIQ_WARN: Percent100 = Percent100::from_permille(750);
    pub const THIRD_LIQ_WARN: Percent100 = Percent100::from_permille(780);
    pub const MAX_LTV: Percent100 = Percent100::from_permille(800);
    pub const RECALC_TIME: Duration = Duration::from_hours(1);

    pub const MAX_SLIPPAGE: Percent100 = Percent100::from_permille(150);

    pub fn liability() -> Liability {
        Liability::new(
            Self::INITIAL_LTV,
            Percent100::from_percent(70),
            Self::FIRST_LIQ_WARN,
            Self::SECOND_LIQ_WARN,
            Self::THIRD_LIQ_WARN,
            Self::MAX_LTV,
            Self::RECALC_TIME,
        )
    }

    pub fn min_asset() -> LpnCoinDTO {
        super::lpn_coin_dto(200)
    }

    pub fn min_transaction() -> LpnCoinDTO {
        super::lpn_coin_dto(50)
    }

    pub fn position_spec() -> PositionSpecDTO {
        PositionSpecDTO::new(
            Self::liability(),
            Self::min_asset(),
            Self::min_transaction(),
        )
    }

    pub fn new_config() -> NewConfig {
        NewConfig {
            lease_position_spec: Instantiator::position_spec(),
            lease_interest_rate_margin: Instantiator::INTEREST_RATE_MARGIN,
            lease_due_period: Instantiator::REPAYMENT_PERIOD,
            lease_max_slippages: MaxSlippages {
                liquidation: MaxSlippage::unchecked(Instantiator::MAX_SLIPPAGE),
            },
        }
    }

    /// The expected address of the leaser contract
    ///
    /// It may not be necessarily the leaser's one, nor contract at all.
    /// Used as `lease_code_admin` or `protocol_admin`.
    /// Due to the protocol contracts instantiation order, the production code cannot and does not
    /// check for existence.
    pub fn expected_addr() -> Addr {
        testing::contract(9, 0)
    }

    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        lease_code: Code,
        lpp: Addr,
        alarms: Alarms,
        profit: Addr,
        reserve: Addr,
        protocols_registry: Addr,
    ) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(leaser::execute, leaser::instantiate, leaser::query)
            .with_reply(leaser::reply)
            .with_sudo(leaser::sudo);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            lease_code: CodeId::from(lease_code).into(),
            lpp,
            profit,
            reserve,
            protocols_registry,
            lease_interest_rate_margin: Self::INTEREST_RATE_MARGIN,
            lease_position_spec: Self::position_spec(),
            lease_due_period: Self::REPAYMENT_PERIOD,
            lease_max_slippages: MaxSlippages {
                liquidation: MaxSlippage::unchecked(Self::MAX_SLIPPAGE),
            },
            lease_admin: testing::user(LEASE_ADMIN),
            time_alarms: alarms.time_alarm,
            market_price_oracle: alarms.market_price_oracle,
            dex: ConnectionParams {
                connection_id: TestCase::DEX_CONNECTION_ID.into(),
                transfer_channel: Ics20Channel {
                    local_endpoint: TestCase::LEASER_IBC_CHANNEL.into(),
                    remote_endpoint: "channel-422".into(),
                },
            },
        };

        app.instantiate(code_id, testing::user(ADMIN), &msg, &[], "leaser", None)
            .unwrap()
            .unwrap_response()
    }
}

pub(super) type LeaserTestCase = TestCase<Addr, Addr, Addr, Addr, Addr, Addr, Addr, Addr>;

pub(crate) struct Alarms {
    pub time_alarm: Addr,
    pub market_price_oracle: Addr,
}

pub fn test_case() -> LeaserTestCase {
    TestCaseBuilder::<Lpn>::new()
        .init_lpp(
            None,
            BASE_INTEREST_RATE,
            UTILIZATION_OPTIMAL,
            ADDON_OPTIMAL_INTEREST_RATE,
            TestCase::DEFAULT_LPP_MIN_UTILIZATION,
        )
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(24)
        .init_reserve()
        .init_leaser()
        .into_generic()
}

pub(crate) fn query_quote<DownpaymentC, LeaseC>(
    app: &App,
    leaser: Addr,
    downpayment: Coin<DownpaymentC>,
    max_ltd: Option<Percent>,
) -> QuoteResponse
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
    LeaseC: CurrencyDef,
    LeaseC::Group: MemberOf<LeaseGroup>,
{
    app.query()
        .query_wasm_smart(
            leaser,
            &QueryMsg::Quote {
                downpayment: test::funds::<_, DownpaymentC>(downpayment.to_primitive()),
                lease_asset: currency::dto::<LeaseC, _>(),
                max_ltd,
            },
        )
        .unwrap()
}

pub(crate) fn expect_a_lease(app: &App, leaser: Addr, customer: Addr) -> Addr {
    let leases = leases(app, leaser, customer);
    assert_eq!(1, leases.len());

    leases.into_iter().next().unwrap()
}

pub(crate) fn assert_no_leases(app: &App, leaser: Addr, customer: Addr) {
    assert!(leases(app, leaser, customer).is_empty());
}

pub(crate) fn assert_lease(app: &App, leaser: Addr, customer: Addr, lease: &Addr) {
    assert!(leases(app, leaser, customer).contains(lease));
}

fn leases(app: &App, leaser: Addr, customer: Addr) -> HashSet<Addr> {
    app.query()
        .query_wasm_smart(leaser, &QueryMsg::Leases { owner: customer })
        .unwrap()
}
