pub(super) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{CurrencyDTO, CurrencyDef, Definition, InPoolWith};
    use sdk::schemars::{self, JsonSchema};

    use crate::{
        lease::{LeaseC2, LeaseC7},
        native::Nls,
    };

    use super::super::Group as LpnGroup;

    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
    )]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct Lpn(CurrencyDTO<LpnGroup>);

    impl CurrencyDef for Lpn {
        type Group = LpnGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            const DTO: CurrencyDTO<LpnGroup> =
                CurrencyDTO::new(&Definition::new("LPN", "ibc/bank_LPN", "ibc/dex_LPN", 6));
            &DTO
        }
    }

    impl InPoolWith<LeaseC2> for Lpn {}

    impl InPoolWith<LeaseC7> for Lpn {}

    impl InPoolWith<Nls> for Lpn {}
}
