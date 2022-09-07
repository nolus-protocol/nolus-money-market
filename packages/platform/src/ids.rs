use std::{
    any::type_name,
    error::Error,
    fmt::{Debug, Display, Formatter},
};

#[macro_export]
macro_rules! generate_ids {
    ($visibility: vis $enum_name: ident as $as_type: ty { $($value: ident $(= $int_value: literal)?),+ $(,)? }) => {
        #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
        $visibility enum $enum_name {
            $($value $(= $int_value)?,)+
        }

        impl ::core::convert::From<$enum_name> for $as_type {
            fn from(value: $enum_name) -> Self {
                value as $as_type
            }
        }

        impl ::core::convert::TryFrom<$as_type> for $enum_name {
            type Error = $crate::ids::TryIdFromIntError;

            fn try_from(value: $as_type) -> ::core::result::Result<Self, Self::Error> {
                ::core::result::Result::Ok(match value {
                    $(value if value == Self::$value as $as_type => Self::$value,)+
                    _ => return ::core::result::Result::Err($crate::ids::TryIdFromIntError),
                })
            }
        }
    };
}

pub struct TryIdFromIntError;

impl Debug for TryIdFromIntError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(type_name::<Self>())
    }
}

impl Display for TryIdFromIntError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(type_name::<Self>())
    }
}

impl Error for TryIdFromIntError {}
