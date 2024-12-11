use currency::CurrencyDTO;
use serde::{Deserialize, Serialize};

use finance::{duration::Duration, percent::Percent};
use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

use crate::finance::LpnCoinDTO;

use super::{DownpaymentCoin, LeaseAssetCurrencies, LeaseCoin};

pub use opened::ClosePolicy;

#[derive(Deserialize, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug, Serialize))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StateQuery {}

#[derive(Serialize)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(Clone, PartialEq, Eq, Debug, Deserialize)
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum StateResponse {
    Opening {
        currency: CurrencyDTO<LeaseAssetCurrencies>,
        downpayment: DownpaymentCoin,
        loan: LpnCoinDTO,
        loan_interest_rate: Percent,
        in_progress: opening::OngoingTrx,
    },
    Opened {
        amount: LeaseCoin,
        loan_interest_rate: Percent,
        margin_interest_rate: Percent,
        principal_due: LpnCoinDTO,
        overdue_margin: LpnCoinDTO,
        overdue_interest: LpnCoinDTO,
        overdue_collect_in: Duration,
        due_margin: LpnCoinDTO,
        due_interest: LpnCoinDTO,
        close_policy: ClosePolicy,
        validity: Timestamp,
        in_progress: Option<opened::OngoingTrx>,
    },
    Paid {
        amount: LeaseCoin,
        in_progress: Option<paid::ClosingTrx>,
    },
    Closed(),
    Liquidated(),
}

pub(crate) mod opening {
    #[cfg(any(test, feature = "testing"))]
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize)]
    #[cfg_attr(
        any(test, feature = "testing"),
        derive(Clone, PartialEq, Eq, Deserialize, Debug)
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum OngoingTrx {
        OpenIcaAccount,
        TransferOut { ica_account: String },
        BuyAsset { ica_account: String },
    }
}

pub(crate) mod opened {
    use finance::percent::Percent;
    #[cfg(any(test, feature = "testing"))]
    use serde::Deserialize;
    use serde::Serialize;

    use crate::api::{LeaseCoin, PaymentCoin};

    /// The data transport type of the configured Lease close policy
    ///
    /// Designed for use in query responses only!
    #[derive(Serialize)]
    #[cfg_attr(
        any(test, feature = "testing"),
        derive(Clone, Default, PartialEq, Eq, Debug, Deserialize)
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct ClosePolicy {
        take_profit: Option<Percent>,
        stop_loss: Option<Percent>,
    }

    #[derive(Serialize)]
    #[cfg_attr(
        any(test, feature = "testing"),
        derive(Clone, PartialEq, Eq, Debug, Deserialize)
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum OngoingTrx {
        Repayment {
            payment: PaymentCoin,
            in_progress: RepayTrx,
        },
        Liquidation {
            liquidation: LeaseCoin,
            in_progress: PositionCloseTrx,
        },
        Close {
            close: LeaseCoin,
            in_progress: PositionCloseTrx,
        },
    }

    #[derive(Serialize)]
    #[cfg_attr(
        any(test, feature = "testing"),
        derive(Clone, PartialEq, Eq, Debug, Deserialize)
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum RepayTrx {
        TransferOut,
        Swap,
        TransferInInit,
        TransferInFinish,
    }

    #[derive(Serialize)]
    #[cfg_attr(
        any(test, feature = "testing"),
        derive(Clone, PartialEq, Eq, Debug, Deserialize)
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum PositionCloseTrx {
        Swap,
        TransferInInit,
        TransferInFinish,
    }

    #[cfg(feature = "contract")]
    impl ClosePolicy {
        pub fn new(tp: Option<Percent>, sl: Option<Percent>) -> Self {
            Self {
                take_profit: tp,
                stop_loss: sl,
            }
        }
    }
}

pub(crate) mod paid {
    #[cfg(any(test, feature = "testing"))]
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Serialize)]
    #[cfg_attr(
        any(test, feature = "testing"),
        derive(Clone, PartialEq, Eq, Debug, Deserialize)
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum ClosingTrx {
        TransferInInit,
        TransferInFinish,
    }
}
