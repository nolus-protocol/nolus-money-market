use serde::Deserialize;

use currency::{dex::LeaseGroup, SymbolOwned, SymbolSlice};
use dex::{Account, CoinVisitor, ConnectionParams, IterNext, IterState, MigrateSpec, SwapTask};
use finance::{coin::CoinDTO, liability::Liability, percent::Percent};
use lpp::stub::LppRef;
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        self, DownpaymentCoin, LoanForm, NewLeaseContract as NewLeaseContract_v6,
        NewLeaseForm as NewLeaseForm_v6, PositionSpecDTO,
    },
    contract::{
        cmd::OpenLoanRespResult,
        finalize::FinalizerRef,
        state::{
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            SwapResult,
        },
    },
    error::ContractResult,
    event::Type,
    lease::v5::{MIN_ASSET, MIN_TRANSACTION},
};

use super::{buy_asset::BuyAsset as BuyAsset_v6, open_ica::OpenIcaAccount as OpenIcaAccount_v6};

pub(in crate::contract::state) type DexState =
    dex::StateRemoteOut<OpenIcaAccount, BuyAsset, ForwardToDexEntry, ForwardToDexEntryContinue>;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseContract {
    /// An application form for opening a new lease
    pub form: NewLeaseForm,
    /// Connection parameters of a Dex capable to perform currency swaps
    pub dex: ConnectionParams,
}

impl NewLeaseContract {
    pub(crate) fn migrate(self) -> NewLeaseContract_v6 {
        NewLeaseContract_v6 {
            form: self.form.migrate(),
            dex: self.dex,
            finalizer: Addr::unchecked("0xDEADCODE"), //use a dummy one since it is used only at lease instance creation time
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NewLeaseForm {
    /// The customer who wants to open a lease.
    pub customer: Addr,
    /// Ticker of the currency this lease will be about.
    pub currency: SymbolOwned,
    /// Maximum Loan-to-Downpayment percentage of the new lease, optional.
    pub max_ltd: Option<Percent>,
    /// Liability parameters
    pub liability: Liability,
    /// Loan parameters
    pub loan: LoanForm,
    /// The time alarms contract the lease uses to get time notifications
    pub time_alarms: Addr,
    /// The oracle contract that sends market price alerts to the lease
    pub market_price_oracle: Addr,
}

impl NewLeaseForm {
    fn migrate(self) -> NewLeaseForm_v6 {
        NewLeaseForm_v6 {
            customer: self.customer,
            currency: self.currency,
            max_ltd: self.max_ltd,
            position_spec: PositionSpecDTO::new_internal(
                self.liability,
                MIN_ASSET.into(),
                MIN_TRANSACTION.into(),
            ),
            loan: self.loan,
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
    deps: (LppRef, OracleRef, TimeAlarmsRef),
}

impl<SEnumNew> MigrateSpec<Self, OpenIcaAccount_v6, SEnumNew> for OpenIcaAccount {
    type Out = OpenIcaAccount_v6;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(Self) -> Self::Out,
    {
        migrate_fn(self)
    }
}

impl OpenIcaAccount {
    pub(crate) fn migrate(self, finalizer: FinalizerRef, now: Timestamp) -> OpenIcaAccount_v6 {
        OpenIcaAccount_v6::new(
            self.new_lease.migrate(),
            self.downpayment,
            self.loan,
            (self.deps.0, self.deps.1, self.deps.2, finalizer),
            now,
        )
    }
}

#[derive(Deserialize)]
pub(crate) struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef),
}

impl BuyAsset {
    pub(crate) fn migrate(self, finalizer: FinalizerRef, now: Timestamp) -> BuyAsset_v6 {
        BuyAsset_v6::new(
            self.form.migrate(),
            self.dex_account,
            self.downpayment,
            self.loan,
            (self.deps.0, self.deps.1, self.deps.2, finalizer),
            now,
        )
    }
}
impl SwapTask for BuyAsset {
    type OutG = LeaseGroup;
    type Label = Type;
    type StateResponse = ContractResult<api::StateResponse>;
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
        _querier: &QuerierWrapper<'_>,
    ) -> Self::Result {
        unreachable!()
    }
}
