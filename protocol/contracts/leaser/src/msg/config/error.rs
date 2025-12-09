use std::{any::type_name, marker::PhantomData};

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
#[error("[Leaser] Programming error or invalid serialized object of '{0}' type, cause '{msg}'", type_name::<T>().to_string())]
pub struct BrokenInvariant<T> {
    msg: String,
    _type: PhantomData<T>,
}

impl<T> BrokenInvariant<T> {
    pub fn r#if(check: bool, msg: &str) -> Result<(), Self> {
        if check {
            Err(Self {
                msg: msg.into(),
                _type: PhantomData,
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use finance::coin::Amount;

    use super::BrokenInvariant;

    #[test]
    fn err() {
        assert!(BrokenInvariant::<Amount>::r#if(true, "").is_err());
    }

    #[test]
    fn ok() {
        assert!(BrokenInvariant::<Amount>::r#if(false, "").is_ok());
    }
}
