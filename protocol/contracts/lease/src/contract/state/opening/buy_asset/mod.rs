use calculator::Factory as CalculatorFactory;
use currency::{
    CurrencyDef, Group, MemberOf,
    never::{self},
};
use finish::BuyAssetFinish;
use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use dex::{
    Account, ContractInSwap, Stage, StartLocalRemoteState, SwapOutputTask, SwapTask,
    WithCalculator, WithOutputTask,
};
use finance::{coin::CoinDTO, duration::Duration};
use platform::ica::HostAccount;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        DownpaymentCoin, LeaseAssetCurrencies, LeasePaymentCurrencies,
        open::{NewLeaseContract, NewLeaseForm},
        query::{StateResponse as QueryStateResponse, opening::OngoingTrx},
    },
    contract::{
        cmd::OpenLoanRespResult,
        finalize::LeasesRef,
        state::{
            SwapClient, SwapResult,
            out_task::{OutTaskFactory, WithOutCurrency},
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
        },
    },
    error::ContractResult,
    event::Type,
    finance::{LppRef, OracleRef},
};

use super::open_ica::OpenIcaAccount;

mod calculator;
mod finish;

type AssetGroup = LeaseAssetCurrencies;
pub(super) type StartState = StartLocalRemoteState<OpenIcaAccount, BuyAsset>;
pub(in super::super) type DexState = dex::StateRemoteOut<
    OpenIcaAccount,
    BuyAsset,
    SwapClient,
    ForwardToDexEntry,
    ForwardToDexEntryContinue,
>;

pub(super) fn start(
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Timestamp,
) -> StartState {
    dex::start_local_remote::<_, BuyAsset>(OpenIcaAccount::new(
        new_lease,
        downpayment,
        loan,
        deps,
        start_opening_at,
    ))
}

type BuyAssetStateResponse = <BuyAsset as SwapTask>::StateResponse;

#[derive(Serialize, Deserialize)]
pub struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Timestamp,
}

impl BuyAsset {
    pub(super) fn new(
        form: NewLeaseForm,
        dex_account: Account,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
        start_opening_at: Timestamp,
    ) -> Self {
        Self {
            form,
            dex_account,
            downpayment,
            loan,
            deps,
            start_opening_at,
        }
    }

    fn state<InP>(self, in_progress_fn: InP) -> BuyAssetStateResponse
    where
        InP: FnOnce(String) -> OngoingTrx,
    {
        Ok(QueryStateResponse::Opening {
            currency: self.form.currency,
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: in_progress_fn(HostAccount::from(self.dex_account).into()),
        })
    }
}

impl SwapTask for BuyAsset {
    type InG = LeasePaymentCurrencies;
    type OutG = AssetGroup;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::OpeningSwap
    }

    fn dex_account(&self) -> &Account {
        &self.dex_account
    }

    fn oracle(&self) -> &impl SwapPath<<Self::InG as Group>::TopG> {
        &self.deps.1
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.deps.2
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
        [self.downpayment, self.loan.principal.into_super_group()].into_iter()
    }

    fn with_slippage_calc<WithCalc>(&self, with_calc: WithCalc) -> WithCalc::Output
    where
        WithCalc: WithCalculator<Self>,
    {
        never::safe_unwrap(
            self.form
                .currency
                .into_super_group()
                .into_currency_type(CalculatorFactory::from(with_calc)),
        )
    }

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>,
    {
        struct OutputTaskFactory {}
        impl OutTaskFactory<BuyAsset> for OutputTaskFactory {
            fn new_task<OutC>(swap_task: BuyAsset) -> impl SwapOutputTask<BuyAsset, OutC = OutC>
            where
                OutC: CurrencyDef,
                OutC::Group: MemberOf<<BuyAsset as SwapTask>::OutG>
                    + MemberOf<<<BuyAsset as SwapTask>::InG as Group>::TopG>,
            {
                BuyAssetFinish::<_, OutC>::from(swap_task)
            }
        }
        never::safe_unwrap(
            self.form
                .currency
                .into_super_group()
                .into_currency_type(WithOutCurrency::<_, OutputTaskFactory, _>::from(self, cmd)),
        )
    }
}

impl ContractInSwap for BuyAsset {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        in_progress: Stage,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        match in_progress {
            Stage::TransferOut => self.state(|ica_account| OngoingTrx::TransferOut { ica_account }),
            Stage::Swap => self.state(|ica_account| OngoingTrx::BuyAsset { ica_account }),
            Stage::TransferInInit => unimplemented!(),
            Stage::TransferInFinish => unimplemented!(),
        }
    }
}

#[cfg(all(feature = "internal.test.contract", test))]
mod test {
    use sdk::cosmwasm_std;

    use crate::contract::state::State;

    #[test]
    fn parse_old() {
        const STATE: &str = r#"{"BuyAsset":{"SwapExactInRespDelivery":{"handler":{"spec":{"form":{"customer":"nolus17rjgmry3w2xcc8yer4h4m8vuypkhkh8he3u8xv","currency":"LC1","max_ltd":250,"position_spec":{"liability":{"initial":600,"healthy":830,"first_liq_warn":850,"second_liq_warn":865,"third_liq_warn":880,"max":900,"recalc_time":7200000000000},"min_asset":{"amount":"15000000","ticker":"LPN"},"min_transaction":{"amount":"10000","ticker":"LPN"}},"loan":{"lpp":"nolus1qqcr7exupnymvg6m63eqwu8pd4n5x6r5t3pyyxdy7r97rcgajmhqy3gn94","profit":"nolus1udkxyfeh7kxjnzm0exfaq9hncqzm3rj59gut4qnll0gq2z4yff0sda5aw2","annual_margin_interest":40,"due_period":1209600000000000},"reserve":"nolus10hzky830fafe5ffzt6vqprmpxjsy0fk8gcq5wvnvgr6lt4s6he3s045c4n","time_alarms":"nolus1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqmx7chl","market_price_oracle":"nolus1jew4l5nq7m3xhkqzy8j7cc99083m5j8d9w004ayyv8xl3yv4h0dql2dd4e"},"dex_account":{"owner":"nolus1yhcph5r2x9rss6tluptttma736rknasjwn3659620ysu5fhmx2wq47gmch","host":"neutron1kdfwfa2pxf7jfth0pej3ds8v4fqa5nhc4nxdm6lr3ctqzvqjfg0shuxdcy","dex":{"connection_id":"connection-11","transfer_channel":{"local_endpoint":"channel-3839","remote_endpoint":"channel-44"}}},"downpayment":{"amount":"40000000","ticker":"LPN"},"loan":{"principal":{"amount":"10000000","ticker":"LPN"},"annual_interest_rate":87},"deps":[{"addr":"nolus1qqcr7exupnymvg6m63eqwu8pd4n5x6r5t3pyyxdy7r97rcgajmhqy3gn94"},{"addr":"nolus1jew4l5nq7m3xhkqzy8j7cc99083m5j8d9w004ayyv8xl3yv4h0dql2dd4e","base_currency":"LPN"},{"addr":"nolus1zwv6feuzhy6a9wekh96cd57lsarmqlwxdypdsplw6zhfncqw6ftqmx7chl"},{"addr":"nolus1et45v5gepxs44jxewfxah0hk4wqmw34m8pm4alf44ucxvj895kas5yrxd8"}],"start_opening_at":"1705072797559458289"}},"response":"Ek4KLC9jb3Ntd2FzbS53YXNtLnYxLk1zZ0V4ZWN1dGVDb250cmFjdFJlc3BvbnNlEh4KHHsicmV0dXJuX2Ftb3VudCI6IjI4ODgzNTQyIn0STQosL2Nvc213YXNtLndhc20udjEuTXNnRXhlY3V0ZUNvbnRyYWN0UmVzcG9uc2USHQobeyJyZXR1cm5fYW1vdW50IjoiNzIyMDgwOCJ9","_forward_to_inner_msg":null,"_delivery_adapter":null}}}"#;
        let state: State = cosmwasm_std::from_json(STATE).unwrap();
        assert!(matches!(state, State::BuyAsset(_)));
    }
}
