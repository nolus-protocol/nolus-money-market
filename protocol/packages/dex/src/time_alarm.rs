use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;

use crate::DexResult;

pub trait TimeAlarm {
    fn setup_alarm(&self, r#for: Timestamp) -> DexResult<Batch>;
}
