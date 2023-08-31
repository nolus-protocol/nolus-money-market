use serde::{de::DeserializeOwned, Serialize};

use crate::SingleVisitor;

use super::{Currency, MaybeVisitResult, Symbol};

pub type Functor<'r, C, V> = fn(Symbol<'r>, V) -> MaybeVisitResult<C, V>;

pub trait SpecificFunctor<'r, C, V>
where
    C: Currency + Serialize + DeserializeOwned,
    V: SingleVisitor<C>,
{
    const FUNCTOR: Functor<'r, C, V>;
}

pub trait GenericFunctor {
    type Specific<'r, C, V>: SpecificFunctor<'r, C, V>
    where
        C: Currency + Serialize + DeserializeOwned,
        V: SingleVisitor<C>;
}

pub trait GeneralizedVisitor {
    type VisitorFunctor: GenericFunctor;

    fn identifier(&self) -> Symbol<'_>;
}

pub trait GeneralizedVisitorExt
where
    Self: GeneralizedVisitor,
{
    fn maybe_visit<C, V>(&self, visitor: V) -> MaybeVisitResult<C, V>
    where
        C: Currency + Serialize + DeserializeOwned,
        V: SingleVisitor<C>,
    {
        <<Self::VisitorFunctor as GenericFunctor>::Specific<'_, C, V> as SpecificFunctor<C, V>>::FUNCTOR(
            self.identifier(),
            visitor,
        )
    }
}

impl<T> GeneralizedVisitorExt for T where T: GeneralizedVisitor {}

macro_rules! impl_visitor {
    ($(($module:ident, $functor:ident, $export:ident)),+ $(,)?) => {
        pub use self::{$($module::Visitor as $export),+};

        $(
            mod $module {
                use std::marker::PhantomData;

                use serde::{de::DeserializeOwned, Serialize};

                use crate::{Currency, SingleVisitor, Symbol};

                use super::{
                    GeneralizedVisitor, GenericFunctor, SpecificFunctor,
                    Functor,
                };

                enum Never {}

                pub struct SpecificFunctorImpl<'r, C, V>(Never, PhantomData<&'r ()>, PhantomData<C>, PhantomData<V>)
                where
                    C: Currency + Serialize + DeserializeOwned,
                    V: SingleVisitor<C>;

                impl<'r, C, V> SpecificFunctor<'r, C, V> for SpecificFunctorImpl<'r, C, V>
                where
                    C: Currency + Serialize + DeserializeOwned,
                    V: SingleVisitor<C>,
                {
                    const FUNCTOR: Functor<'r, C, V> = crate::$functor::<C, V>;
                }

                pub struct GenericFunctorImpl(Never);

                impl GenericFunctor for GenericFunctorImpl {
                    type Specific<'r, C, V> = SpecificFunctorImpl<'r, C, V>
                    where
                        C: Currency + Serialize + DeserializeOwned,
                        V: SingleVisitor<C>;
                }

                pub struct Visitor<'r>(Symbol<'r>);

                impl<'r> Visitor<'r> {
                    pub const fn new(ticker: Symbol<'r>) -> Self {
                        Self(ticker)
                    }
                }

                impl<'r> GeneralizedVisitor for Visitor<'r> {
                    type VisitorFunctor = GenericFunctorImpl;

                    fn identifier(&self) -> Symbol<'_> {
                        self.0
                    }
                }
            }
        )+
    };
}

impl_visitor![
    (bank, maybe_visit_on_bank_symbol, BankSymbolVisitor),
    (dex, maybe_visit_on_dex_symbol, DexSymbolVisitor),
    (ticker, maybe_visit_on_ticker, TickerVisitor),
];
