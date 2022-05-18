use cosmwasm_std::{Uint128, Decimal, Timestamp, Env};

pub const NANOSECS_IN_YEAR: Uint128 = Uint128::new(365 * 24 * 60 * 60 * 1000 * 1000 * 1000);

/// Time difference in nanosecs between current block time and timestamp.
pub fn dt(env: &Env, time: Timestamp) -> Uint128 {
    let ct = env.block.time.nanos();
    let t = time.nanos();
    assert!(ct > t);
    Uint128::new((ct - t).into())
}

/// Calculate interest
pub fn interest(due: Uint128, rate: Decimal, dt_nanos: Uint128) -> Uint128 {
    due*rate*dt_nanos/NANOSECS_IN_YEAR
}
