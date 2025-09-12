use std::{any::TypeId, borrow::Borrow, marker::PhantomData};

use crate::{
    CurrencyDef,
    group::FilterMapT,
    test::{
        SuperGroup, SuperGroupTestC1, SuperGroupTestC2, SuperGroupTestC3, SuperGroupTestC4,
        SuperGroupTestC5,
    },
};

/// Iterator over [`SuperGroup`] currency types mapped to some values
pub(super) struct Currencies<FilterMap, FilterMapRef> {
    f: FilterMapRef,
    _f_type: PhantomData<FilterMap>,
    next: Option<TypeId>,
}

impl<FilterMap, FilterMapRef> Currencies<FilterMap, FilterMapRef>
where
    FilterMap: FilterMapT<SuperGroup>,
    FilterMapRef: Borrow<FilterMap>,
{
    pub fn with_filter(f: FilterMapRef) -> Self {
        Self {
            f,
            _f_type: PhantomData,
            next: Some(TypeId::of::<SuperGroupTestC1>()),
        }
    }

    fn next_map(&mut self) -> Option<FilterMap::Outcome> {
        debug_assert!(self.next.is_some());

        // TODO define `const` for each of the currencies
        // once `const fn TypeId::of` gets stabilized
        // and switch from `if-else` to `match`
        let c1_type = TypeId::of::<SuperGroupTestC1>();
        let c2_type = TypeId::of::<SuperGroupTestC2>();
        let c3_type = TypeId::of::<SuperGroupTestC3>();
        let c4_type = TypeId::of::<SuperGroupTestC4>();
        let c5_type = TypeId::of::<SuperGroupTestC5>();

        self.next.and_then(|next_type| {
            let filter = self.f.borrow();
            if next_type == c1_type {
                self.next = Some(c2_type);
                filter.on::<SuperGroupTestC1>(SuperGroupTestC1::dto())
            } else if next_type == c2_type {
                self.next = Some(c3_type);
                filter.on::<SuperGroupTestC2>(SuperGroupTestC2::dto())
            } else if next_type == c3_type {
                self.next = Some(c4_type);
                filter.on::<SuperGroupTestC3>(SuperGroupTestC3::dto())
            } else if next_type == c4_type {
                self.next = Some(c5_type);
                filter.on::<SuperGroupTestC4>(SuperGroupTestC4::dto())
            } else if next_type == c5_type {
                self.next = None;
                filter.on::<SuperGroupTestC5>(SuperGroupTestC5::dto())
            } else {
                unimplemented!("Unknown type found!")
            }
        })
    }
}

impl<FilterMap, FilterMapRef> Iterator for Currencies<FilterMap, FilterMapRef>
where
    FilterMap: FilterMapT<SuperGroup>,
    FilterMapRef: Borrow<FilterMap>,
{
    type Item = FilterMap::Outcome;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result = None;
        while result.is_none() && self.next.is_some() {
            result = self.next_map();
        }
        result
    }
}

#[cfg(test)]
mod test {

    use crate::{
        CurrencyDef, Group,
        test::{
            SubGroupTestC6, SubGroupTestC10, SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
            SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
            filter::{Dto, FindByTicker},
        },
    };

    #[test]
    fn enumerate_all() {
        let filter = Dto::default();
        let mut iter = SuperGroup::filter_map(filter);

        assert_eq!(Some(SuperGroupTestC1::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC2::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC3::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC4::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC5::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC6::dto().into_super_group()), iter.next());
        assert_eq!(Some(SubGroupTestC10::dto().into_super_group()), iter.next());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn skip_some() {
        let filter = FindByTicker::new(SuperGroupTestC3::ticker(), SubGroupTestC10::ticker());
        let mut iter = SuperGroup::filter_map(filter);
        assert_eq!(Some(SuperGroupTestC3::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC10::dto().into_super_group()), iter.next());
        assert_eq!(None, iter.next().as_ref());
    }
}
