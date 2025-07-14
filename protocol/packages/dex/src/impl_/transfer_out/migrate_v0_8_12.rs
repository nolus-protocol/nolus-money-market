use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::{CoinsNb, SwapTask as SwapTaskT};

use super::TransferOut as LastVersionTransferOut;

/// Migrate v0.8.12 TransferOut to the next v0.8.14 format
// TODO clean-up the v0.8.12 support once all leases have gone through this migration
#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[serde(
    bound(deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"),
    // deliberately not using `deny_unknown_fields` following the serde doc
    rename_all = "snake_case",
    untagged
)]
pub(super) enum TransferOut<SwapTask, SEnum, SwapClient> {
    V0_8_14 {
        // this is the first variant since the untagged enum representation tries to deserialize to variants as per their order in the definition
        spec: SwapTask,
        acks_left: CoinsNb,
        // cannot use the actual 0.8.14 type due to overflow during deserialization
        // cause: the type tries to deserialize from this enum and so on
        // unit tests guaranee correctness
        //new: LastVersionTransferOut<SwapTask, SEnum, SwapClient>,
    },
    V0_8_12 {
        #[serde(flatten)]
        prev: PrevVersionTransferOut<SwapTask, SEnum, SwapClient>,
    },
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(test, derive(Clone, Debug, PartialEq, Eq))]
#[serde(rename_all = "snake_case")] // deliberately not using `deny_unknown_fields` following the serde doc
pub struct PrevVersionTransferOut<SwapTask, SEnum, SwapClient> {
    spec: SwapTask,
    coin_index: CoinsNb,
    last_coin_index: CoinsNb,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
    #[serde(skip)]
    _swap_client: PhantomData<SwapClient>,
}

impl<SwapTask, SEnum, SwapClient> From<TransferOut<SwapTask, SEnum, SwapClient>>
    for LastVersionTransferOut<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn from(value: TransferOut<SwapTask, SEnum, SwapClient>) -> Self {
        match value {
            TransferOut::V0_8_14 { spec, acks_left } => {
                LastVersionTransferOut::nth(spec, acks_left)
            }
            TransferOut::V0_8_12 { prev } => LastVersionTransferOut::migrate_from(
                prev.spec,
                prev.coin_index,
                prev.last_coin_index,
            ),
        }
    }
}

#[cfg(test)]
mod test_two_versions {

    use std::marker::PhantomData;

    use oracle::stub::SwapPath;
    use oracle_platform::OracleRef;
    use serde::{Deserialize, Serialize};

    use currency::{
        Group,
        never::Never,
        test::{SubGroup, SuperGroup, SuperGroupTestC1},
    };
    use finance::coin::{Coin, CoinDTO};
    use platform::tests;
    use sdk::cosmwasm_std::{self, Addr};
    use timealarms::stub::TimeAlarmsRef;

    use super::{LastVersionTransferOut, TransferOut};
    use crate::{
        Account, CoinsNb, SwapTask, impl_::transfer_out::migrate_v0_8_12::PrevVersionTransferOut,
    };

    const ORACLE_ADDR: &str = "my_nice_oracle";

    #[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
    struct DummyTask(OracleRef<SuperGroupTestC1, SuperGroup>);
    impl SwapTask for DummyTask {
        type InG = SuperGroup;

        type OutG = SubGroup;

        type Label = String;

        type StateResponse = Never;

        type Result = Never;

        fn label(&self) -> Self::Label {
            unimplemented!()
        }

        fn dex_account(&self) -> &Account {
            unimplemented!()
        }

        fn oracle(&self) -> &impl SwapPath<<Self::InG as Group>::TopG> {
            &self.0
        }

        fn time_alarm(&self) -> &TimeAlarmsRef {
            unimplemented!()
        }

        fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
            [
                Coin::<SuperGroupTestC1>::new(10).into(),
                Coin::<SuperGroupTestC1>::new(20).into(),
            ]
        }

        fn with_slippage_calc<WithCalc>(&self, _with_calc: WithCalc) -> WithCalc::Output
        where
            WithCalc: crate::WithCalculator<Self>,
        {
            unimplemented!()
        }

        fn into_output_task<Cmd>(self, _cmd: Cmd) -> Cmd::Output
        where
            Cmd: crate::WithOutputTask<Self>,
        {
            unimplemented!()
        }
    }

    #[test]
    fn read_8_12_into_8_14() {
        const RAW_8_12: &str = r#"{
            "spec":{"addr": "my_nice_oracle"},
            "coin_index":0,
            "last_coin_index":1
        }"#;

        let spec = DummyTask(OracleRef::unchecked(Addr::unchecked(ORACLE_ADDR)));
        let coin_index = 0;
        let last_coin_index = 1;

        let transfer_v8_12 = PrevVersionTransferOut {
            spec: spec.clone(),
            coin_index,
            last_coin_index,
            _state_enum: PhantomData::<Never>,
            _swap_client: PhantomData::<Never>,
        };
        let transfer_variant_v8_12 = TransferOut::V0_8_12 {
            prev: transfer_v8_12.clone(),
        };

        assert_eq!(transfer_v8_12, cosmwasm_std::from_json(RAW_8_12).unwrap());

        assert_eq!(
            transfer_variant_v8_12,
            cosmwasm_std::from_json(RAW_8_12).unwrap()
        );

        let exp_out = LastVersionTransferOut::<DummyTask, Never, Never>::migrate_from(
            spec.clone(),
            coin_index,
            last_coin_index,
        );
        assert_eq!(
            LastVersionTransferOut::internal_new(
                spec,
                last_coin_index - coin_index + 1,
                coin_index + 1
            ),
            exp_out
        );
        assert_eq!(
            exp_out,
            cosmwasm_std::from_json::<TransferOut::<DummyTask, Never, Never>>(RAW_8_12)
                .unwrap()
                .into()
        );
        assert_eq!(exp_out, cosmwasm_std::from_json(RAW_8_12).unwrap());
        assert_eq!(exp_out, tests::ser_de(&transfer_v8_12).unwrap());
    }

    #[test]
    fn read_8_14() {
        const RAW_8_14: &str = r#"{
            "spec":{"addr": "my_nice_oracle"},
            "acks_left":2
        }"#;

        let spec = DummyTask(OracleRef::unchecked(Addr::unchecked(ORACLE_ADDR)));
        let coins_nb: CoinsNb = spec.coins().into_iter().count().try_into().unwrap();
        let acks_left = coins_nb;

        let transfer_v8_14 = LastVersionTransferOut {
            spec: spec.clone(),
            acks_left,
            requests_sent: coins_nb,
            _state_enum: PhantomData::<Never>,
            _swap_client: PhantomData::<Never>,
        };
        let transfer_variant_v8_14 = TransferOut::<_, Never, Never>::V0_8_14 {
            spec: spec.clone(),
            acks_left,
        };

        assert_eq!(
            transfer_variant_v8_14,
            cosmwasm_std::from_json(RAW_8_14).unwrap()
        );
        assert_eq!(transfer_v8_14, cosmwasm_std::from_json(RAW_8_14).unwrap());

        assert_eq!(
            LastVersionTransferOut::nth(spec.clone(), acks_left),
            cosmwasm_std::from_json::<TransferOut::<DummyTask, Never, Never>>(RAW_8_14)
                .unwrap()
                .into()
        );

        assert_eq!(transfer_v8_14, tests::ser_de(&transfer_v8_14).unwrap());
    }
}
