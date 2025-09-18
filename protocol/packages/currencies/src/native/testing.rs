pub(super) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{CurrencyDTO, CurrencyDef, Definition, InPoolWith, PairsFindMapT, PairsGroup};

    use crate::{lease::LeaseC5, payment::Group as PaymentGroup};

    use super::super::Group as NativeGroup;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct Nls(CurrencyDTO<NativeGroup>);

    impl CurrencyDef for Nls {
        type Group = NativeGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const { CurrencyDTO::new(const { &Definition::new("NLS", "unls", "ibc/dex_NLS", 6) }) }
        }
    }

    impl PairsGroup for Nls {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(_f: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            todo!("(Lpn)")
        }
    }

    impl InPoolWith<LeaseC5> for Nls {}
}
