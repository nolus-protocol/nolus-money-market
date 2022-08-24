#![no_std]

mod sealed {
    pub trait SealedId<T>
    where
        Self: Sized,
        T: TryInto<Self>,
    {
        fn sealed_downcast(self) -> T;
    }
}

pub type Error = ();

pub trait IdType<T>
where
    Self: sealed::SealedId<T>,
    T: TryInto<Self>,
{
    fn downcast(self) -> T;
}

impl<T, U> IdType<U> for T
where
    Self: sealed::SealedId<U>,
    U: TryInto<Self>,
{
    fn downcast(self) -> U {
        <Self as sealed::SealedId<U>>::sealed_downcast(self)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct RawId<T>(T);

impl<T> From<T> for RawId<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> sealed::SealedId<T> for RawId<T>
where
    T: TryInto<Self>,
{
    fn sealed_downcast(self) -> T {
        self.0
    }
}

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

        impl ::core::convert::From<$enum_name> for $crate::RawId<$as_type> {
            fn from(value: $enum_name) -> Self {
                RawId(value as $as_type)
            }
        }

        impl ::core::convert::TryFrom<$as_type> for $enum_name {
            type Error = crate::Error;

            fn try_from(value: $as_type) -> ::core::result::Result<Self, Self::Error> {
                ::core::result::Result::Ok(match value {
                    $(value if value == Self::$value as $as_type => Self::$value,)+
                    _ => return ::core::result::Result::Err(()),
                })
            }
        }

        impl $crate::sealed::SealedId<$as_type> for $enum_name {
            fn sealed_downcast(self) -> $as_type {
                self as $as_type
            }
        }
    };
}

generate_ids! {
    LeaseReplyId as u64 {
        LppLoan,
        LeaseTimeAlarm,
        LiquidationExcess,
    }
}
