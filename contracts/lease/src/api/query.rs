#[cfg(any(test, feature = "testing"))]
use serde::Deserialize;
use serde::Serialize;

use finance::percent::Percent;
use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

use super::{DownpaymentCoin, LeaseCoin, LpnCoin};

#[derive(Serialize, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug, Deserialize))]
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
        downpayment: DownpaymentCoin,
        loan: LpnCoin,
        loan_interest_rate: Percent,
        in_progress: opening::OngoingTrx,
    },
    Opened {
        amount: LeaseCoin,
        loan_interest_rate: Percent,
        margin_interest_rate: Percent,
        principal_due: LpnCoin,
        previous_margin_due: LpnCoin,
        previous_interest_due: LpnCoin,
        current_margin_due: LpnCoin,
        current_interest_due: LpnCoin,
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

pub mod opening {
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

pub mod opened {
    #[cfg(any(test, feature = "testing"))]
    use serde::Deserialize;
    use serde::Serialize;

    use crate::api::{LeaseCoin, PaymentCoin};

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
}

pub mod paid {
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
