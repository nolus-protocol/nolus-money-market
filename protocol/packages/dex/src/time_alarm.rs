use finance::instant::Instant;
use platform::batch::Batch;

use crate::DexResult;

pub trait TimeAlarm {
    fn setup_alarm(&self, r#for: Instant) -> DexResult<Batch>;
}
