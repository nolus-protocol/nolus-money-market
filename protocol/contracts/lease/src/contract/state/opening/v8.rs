use serde::Deserialize;

use currencies::LeaseGroup;
use currency::{SymbolOwned, SymbolSlice};
use dex::{Account, CoinVisitor, ConnectionParams, IterNext, IterState, MigrateSpec, SwapTask};
use finance::{coin::CoinDTO, percent::Percent};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        open::{
            LoanForm, NewLeaseContract as NewLeaseContract_v9, NewLeaseForm as NewLeaseForm_v9,
            PositionSpecDTO,
        },
        query::StateResponse,
        DownpaymentCoin, LeasePaymentCurrencies,
    },
    contract::{
        cmd::OpenLoanRespResult,
        finalize::FinalizerRef,
        state::{
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            SwapClient, SwapResult,
        },
    },
    error::ContractResult,
    event::Type,
    finance::{LppRef, OracleRef, ReserveRef},
};

use super::{buy_asset::BuyAsset as BuyAsset_v9, open_ica::OpenIcaAccount as OpenIcaAccount_v9};

pub(in super::super) type DexState = dex::StateRemoteOut<
    OpenIcaAccount,
    BuyAsset,
    LeasePaymentCurrencies,
    SwapClient,
    ForwardToDexEntry,
    ForwardToDexEntryContinue,
>;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseContract {
    form: NewLeaseForm,
    dex: ConnectionParams,
    finalizer: Addr,
}

impl NewLeaseContract {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> NewLeaseContract_v9 {
        NewLeaseContract_v9 {
            form: self.form.migrate(reserve),
            dex: self.dex,
            finalizer: self.finalizer,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseForm {
    customer: Addr,
    currency: SymbolOwned,
    max_ltd: Option<Percent>,
    position_spec: PositionSpecDTO,
    loan: LoanForm,
    time_alarms: Addr,
    market_price_oracle: Addr,
}

impl NewLeaseForm {
    fn migrate(self, reserve: ReserveRef) -> NewLeaseForm_v9 {
        NewLeaseForm_v9 {
            customer: self.customer,
            currency: self.currency,
            max_ltd: self.max_ltd,
            position_spec: self.position_spec,
            loan: self.loan,
            reserve: reserve.into(),
            time_alarms: self.time_alarms,
            market_price_oracle: self.market_price_oracle,
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct OpenIcaAccount {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, FinalizerRef),
    start_opening_at: Timestamp,
}

impl<SEnumNew> MigrateSpec<Self, OpenIcaAccount_v9, SEnumNew> for OpenIcaAccount {
    type Out = OpenIcaAccount_v9;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(Self) -> Self::Out,
    {
        migrate_fn(self)
    }
}

impl OpenIcaAccount {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> OpenIcaAccount_v9 {
        OpenIcaAccount_v9::new(
            self.new_lease.migrate(reserve),
            self.downpayment,
            self.loan,
            self.deps,
            self.start_opening_at,
        )
    }
}

#[derive(Deserialize)]
pub(crate) struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, FinalizerRef),
    start_opening_at: Timestamp,
}

impl BuyAsset {
    pub(crate) fn migrate(self, reserve: ReserveRef) -> BuyAsset_v9 {
        BuyAsset_v9::new(
            self.form.migrate(reserve),
            self.dex_account,
            self.downpayment,
            self.loan,
            self.deps,
            self.start_opening_at,
        )
    }
}
impl SwapTask for BuyAsset {
    type OutG = LeaseGroup;
    type Label = Type;
    type StateResponse = ContractResult<StateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        unreachable!()
    }

    fn dex_account(&self) -> &Account {
        unreachable!()
    }

    fn oracle(&self) -> &OracleRef {
        unreachable!()
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        unreachable!()
    }

    fn out_currency(&self) -> &SymbolSlice {
        unreachable!()
    }

    fn on_coins<Visitor>(&self, _visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        unreachable!()
    }

    fn finish(
        self,
        _amount_out: CoinDTO<Self::OutG>,
        _env: &Env,
        _querier: QuerierWrapper<'_>,
    ) -> Self::Result {
        unreachable!()
    }
}
