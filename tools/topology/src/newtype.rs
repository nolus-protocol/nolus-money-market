macro_rules! define {
    (
        $(#[$($attributes:tt)+])*
        $visibility:vis $newtype:ident $(<$([$($generics:tt)+]),+ $(,)?>)? ($field:ty $(,)?)
        $(where $($where_clause:tt)+)?
    ) => {
        $(#[$($attributes)+])*
        $visibility struct $newtype $(<$($($generics)+),+>)? ($field)
        $(where $($where_clause)+)?;
    };
    (
        $(#[$($attributes:tt)+])*
        $visibility:vis $newtype:ident $(<$([$($generics:tt)+]),+ $(,)?>)? ($field:ty $(,)?)
        as [$borrowed:ty $(,)?]
        $(where $($where_clause:tt)+)?
    ) => {
        $crate::newtype::define!{
            $(#[$($attributes)+])*
            #[derive(PartialEq, Eq, PartialOrd, Ord)]
            $visibility $newtype $(<$([$($generics)+]),+>)? ($field)
            $(where $($where_clause)+)?
        }

        impl $(<$($($generics)+),+>)? ::std::hash::Hash for $newtype $(<$($($generics)+),+>)?
        $(where $($where_clause)+)?
        {
            #[inline]
            fn hash<H>(&self, state: &mut H)
            where
                H: ::std::hash::Hasher,
            {
                <$field as ::std::hash::Hash>::hash(&self.0, state)
            }
        }

        impl $(<$($($generics)+),+>)? ::std::borrow::Borrow<$borrowed> for $newtype $(<$($($generics)+),+>)?
        where
            $field: ::std::borrow::Borrow<$borrowed>,
            $($($where_clause)+)?
        {
            #[inline]
            fn borrow(&self) -> &$borrowed {
                <$field as ::std::borrow::Borrow<$borrowed>>::borrow(&self.0)
            }
        }
    };
    (
        $(#[$($attributes:tt)+])*
        $visibility:vis $newtype:ident $(<$([$($generics:tt)+]),+ $(,)?>)? ($field:ty $(,)?)
        as [$borrowed:ty, $($borrowed_rest:ty),+ $(,)?]
        $(where $($where_clause:tt)+)?
    ) => {
        $crate::newtype::define!{
            $(#[$($attributes)+])*
            $visibility $newtype $(<$([$($generics)+]),+>)? ($field)
            as [$($borrowed_rest),+]
            $(where $($where_clause)+)?
        }

        impl $(<$($($generics)+),+>)? ::std::borrow::Borrow<$borrowed> for $newtype $(<$($($generics)+),+>)?
        where
            $field: ::std::borrow::Borrow<$borrowed>,
            $($($where_clause)+)?
        {
            #[inline]
            fn borrow(&self) -> &$borrowed {
                <$field as ::std::borrow::Borrow<$borrowed>>::borrow(&self.0)
            }
        }
    };
}

pub(crate) use define;
