#[macro_export]
macro_rules! generate_ids {
    ($enum_name: ident as $as_type: ident { $($value: ident),+ $(,)? }) => {
        #[repr($as_type)]
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub enum $enum_name {
            $($value,)+
        }

        impl ::core::convert::From<$enum_name> for $as_type {
            fn from(value: $enum_name) -> Self {
                value as $as_type
            }
        }

        impl ::core::convert::TryFrom<$as_type> for $enum_name {
            type Error = ();

            fn try_from(value: $as_type) -> ::core::result::Result<Self, Self::Error> {
                ::core::result::Result::Ok(match value {
                    $(value if value == Self::$value as $as_type => Self::$value,)+
                    _ => return ::core::result::Result::Err(()),
                })
            }
        }
    };
}
