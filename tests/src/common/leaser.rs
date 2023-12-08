use std::collections::HashSet;

use currency::Currency;
use finance::{coin::Coin, duration::Duration, liability::Liability, percent::Percent, test};
use lease::api::{ConnectionParams, Ics20Channel, InterestPaymentSpec, LpnCoin, PositionSpecDTO};
use leaser::{
    execute, instantiate,
    msg::{InstantiateMsg, QueryMsg, QuoteResponse},
    query, reply, sudo,
};
use platform::contract::CodeId;
use sdk::cosmwasm_std::{Addr, Uint64};

use super::{
    test_case::{app::App, TestCase},
    CwContractWrapper, ADMIN,
};

pub(crate) struct Instantiator;

impl Instantiator {
    pub const INTEREST_RATE_MARGIN: Percent = Percent::from_permille(30);

    pub const REPAYMENT_PERIOD: Duration = Duration::from_days(90);

    pub const GRACE_PERIOD: Duration = Duration::from_days(10);

    pub const RECALC_TIME: Duration = Duration::from_hours(1);

    pub fn liability() -> Liability {
        Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            Percent::from_percent(2),
            Percent::from_percent(3),
            Percent::from_percent(2),
            Self::RECALC_TIME,
        )
    }

    pub fn min_asset() -> LpnCoin {
        super::lpn_coin(200)
    }

    pub fn min_transaction() -> LpnCoin {
        super::lpn_coin(50)
    }

    pub fn position_spec() -> PositionSpecDTO {
        PositionSpecDTO::new(
            Self::liability(),
            Self::min_asset(),
            Self::min_transaction(),
        )
    }

    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        lease_code_id: CodeId,
        lpp_addr: Addr,
        time_alarms: Addr,
        market_price_oracle: Addr,
        profit: Addr,
    ) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            lease_code_id: Uint64::new(lease_code_id),
            lpp_ust_addr: lpp_addr,
            lease_interest_rate_margin: Self::INTEREST_RATE_MARGIN,
            lease_position_spec: Self::position_spec(),
            lease_interest_payment: InterestPaymentSpec::new(
                Self::REPAYMENT_PERIOD,
                Self::GRACE_PERIOD,
            ),
            time_alarms,
            market_price_oracle,
            profit,
            dex: ConnectionParams {
                connection_id: TestCase::DEX_CONNECTION_ID.into(),
                transfer_channel: Ics20Channel {
                    local_endpoint: TestCase::LEASER_IBC_CHANNEL.into(),
                    remote_endpoint: "channel-422".into(),
                },
            },
        };

        app.instantiate(code_id, Addr::unchecked(ADMIN), &msg, &[], "leaser", None)
            .unwrap()
            .unwrap_response()
    }
}

pub(crate) fn query_quote<DownpaymentC, LeaseC>(
    app: &App,
    leaser: Addr,
    downpayment: Coin<DownpaymentC>,
    max_ltd: Option<Percent>,
) -> QuoteResponse
where
    DownpaymentC: Currency,
    LeaseC: Currency,
{
    app.query()
        .query_wasm_smart(
            leaser,
            &QueryMsg::Quote {
                downpayment: test::funds::<_, DownpaymentC>(downpayment.into()),
                lease_asset: LeaseC::TICKER.into(),
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
