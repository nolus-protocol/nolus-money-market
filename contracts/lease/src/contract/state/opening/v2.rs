use dex::{SwapExactIn as SwapExactInV3, TransferOut as TransferOutV3};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use serde::Deserialize;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{DownpaymentCoin, NewLeaseContract, NewLeaseForm},
    contract::{
        cmd::OpenLoanRespResult,
        state::{
            v2::{Account as AccountV2, Migrate},
            State,
        },
    },
};

use super::{
    buy_asset::{BuyAsset as BuyAssetV3, DexState},
    open_ica::OpenIcaAccount as OpenIcaAccountV3,
};

type CoinsNb = u8;
pub(in crate::contract) type Transfer = TransferOut;
pub(in crate::contract) type Swap = SwapExactIn;

#[derive(Deserialize)]
pub(in crate::contract) struct RequestLoan();
impl Migrate for RequestLoan {
    fn into_last_version(self) -> State {
        unreachable!("This state is transient and do not last past a transaction is over")
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct OpenIcaAccount {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl From<OpenIcaAccount> for OpenIcaAccountV3 {
    fn from(value: OpenIcaAccount) -> Self {
        let timealarms = TimeAlarmsRef::unchecked(value.new_lease.form.time_alarms.clone());
        let deps = (value.deps.0, value.deps.1, timealarms);
        Self::new(value.new_lease, value.downpayment, value.loan, deps)
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct BuyAsset {
    form: NewLeaseForm,
    dex_account: AccountV2,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl From<BuyAsset> for BuyAssetV3 {
    fn from(value: BuyAsset) -> Self {
        let timealarms = TimeAlarmsRef::unchecked(value.form.time_alarms.clone());
        let deps = (value.deps.0, value.deps.1, timealarms);
        Self::migrate_to(
            value.form,
            value.dex_account.into(),
            value.downpayment,
            value.loan,
            deps,
        )
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct TransferOut {
    pub spec: BuyAsset,
    pub coin_index: CoinsNb,
    pub last_coin_index: CoinsNb,
}

impl Migrate for TransferOut {
    fn into_last_version(self) -> State {
        DexState::from(TransferOutV3::migrate_from(
            self.spec.into(),
            self.coin_index,
            self.last_coin_index,
        ))
        .into()
    }
}

#[derive(Deserialize)]
pub(in crate::contract) struct SwapExactIn {
    pub spec: BuyAsset,
}

impl Migrate for SwapExactIn {
    fn into_last_version(self) -> State {
        DexState::from(SwapExactInV3::migrate_from(self.spec.into())).into()
    }
}
