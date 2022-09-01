#[macro_export]
macro_rules! generate_ids {
    ($enum_name: ident as $as_type: ty { $($value: ident),+ $(,)? }) => {
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

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    pub fn either_into<T>(self) -> T
    where
        L: Into<T>,
        R: Into<T>,
    {
        match self {
            Self::Left(left) => left.into(),
            Self::Right(right) => right.into(),
        }
    }

    pub fn either_try_into<T>(self) -> Result<T, <L as TryInto<T>>::Error>
    where
        L: TryInto<T>,
        R: TryInto<T, Error = <L as TryInto<T>>::Error>,
    {
        match self {
            Self::Left(left) => left.try_into(),
            Self::Right(right) => right.try_into(),
        }
    }
}
