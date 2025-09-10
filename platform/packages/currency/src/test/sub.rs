use std::any::TypeId;

use crate::{
    CurrencyDef,
    group::FilterMapT,
    test::{SubGroup, SubGroupTestC6, SubGroupTestC10},
};

/// Iterator over [`SubGroup`] currency types mapped to some values
pub(super) struct Currencies<FilterMap> {
    f: FilterMap,
    next: Option<TypeId>,
}

impl<FilterMap> Currencies<FilterMap>
where
    FilterMap: FilterMapT<SubGroup>,
{
    pub fn with_filter(f: FilterMap) -> Self {
        Self {
            f,
            next: Some(TypeId::of::<SubGroupTestC6>()),
        }
    }

    fn next_map(&mut self) -> Option<FilterMap::Outcome> {
        debug_assert!(self.next.is_some());

        // TODO define `const` for each of the currencies
        // once `const fn TypeId::of` gets stabilized
        // and switch from `if-else` to `match`
        let c6_type = TypeId::of::<SubGroupTestC6>();
        let c10_type = TypeId::of::<SubGroupTestC10>();

        self.next.and_then(|next_type| {
            if next_type == c6_type {
                self.next = Some(c10_type);
                self.f.on::<SubGroupTestC6>(SubGroupTestC6::dto())
            } else if next_type == c10_type {
                self.next = None;
                self.f.on::<SubGroupTestC10>(SubGroupTestC10::dto())
            } else {
                unimplemented!("Unknown type found!")
            }
        })
    }
}

impl<FilterMap> Iterator for Currencies<FilterMap>
where
    FilterMap: FilterMapT<SubGroup>,
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
        CurrencyDef,
        test::{
            SubGroupTestC6, SubGroupTestC10, SuperGroupTestC1,
            filter::{Dto, FindByTicker},
        },
    };

    use super::Currencies;

    #[test]
    fn enumerate_all() {
        let mut iter = Currencies::with_filter(Dto::default());
        assert_eq!(Some(SubGroupTestC6::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC10::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn skip_some() {
        let mut iter = Currencies::with_filter(FindByTicker::new(
            SubGroupTestC10::ticker(),
            SuperGroupTestC1::ticker(),
        ));
        assert_eq!(Some(SubGroupTestC10::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }
}
