pub mod coin;
pub mod currency;
pub mod duration;
pub mod error;
pub mod fraction;
pub mod fractionable;
pub mod interest;
pub mod liability;
pub mod percent;
pub mod price;
pub mod ratio;

#[macro_export]
macro_rules! check {
    ($cond:expr $(,)?) => {
        if !$cond {
            Err(Error::broken_invariant_err::<Liability>(""))
        }
    };

    ($cond:expr, $($arg:tt)+) => {
        if !$cond {
            let mut msg = String::from("");
            $(msg.push_str($arg);)*
            Err(Error::broken_invariant_err::<Liability>(&msg))
        } else {
            Ok(())
        }
    };
}
