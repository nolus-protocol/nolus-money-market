use crate::{
    CurrencyDef, FindMapT,
    group::{FilterMapT, GroupMember},
    test::{SubGroup, SubGroupTestC6, SubGroupTestC10},
};

// ======== START GENERATED CODE =========
pub(super) enum Item {
    SubGroupTestC6(),
    SubGroupTestC10(),
}

impl GroupMember<SubGroup> for Item {
    fn first() -> Option<Self> {
        Some(Self::SubGroupTestC6())
    }

    fn next(&self) -> Option<Self> {
        match self {
            Item::SubGroupTestC6() => Some(Self::SubGroupTestC10()),
            Item::SubGroupTestC10() => None,
        }
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: FilterMapT<SubGroup>,
    {
        match *self {
            Item::SubGroupTestC6() => filter_map.on::<SubGroupTestC6>(SubGroupTestC6::dto()),
            Item::SubGroupTestC10() => filter_map.on::<SubGroupTestC10>(SubGroupTestC10::dto()),
        }
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<SubGroup>,
    {
        match *self {
            Item::SubGroupTestC6() => find_map.on::<SubGroupTestC6>(SubGroupTestC6::dto()),
            Item::SubGroupTestC10() => find_map.on::<SubGroupTestC10>(SubGroupTestC10::dto()),
        }
    }
}

// ======== END GENERATED CODE =========

#[cfg(test)]
mod test {

    use crate::{
        CurrencyDef, Group,
        test::{
            SubGroup, SubGroupTestC6, SubGroupTestC10, SuperGroupTestC1,
            filter::{Dto, FindByTicker},
        },
    };

    #[test]
    fn enumerate_all() {
        let filter = Dto::<SubGroup>::default();
        let mut iter = SubGroup::filter_map::<Dto<SubGroup>, _>(&filter);
        assert_eq!(Some(SubGroupTestC6::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC10::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn skip_some() {
        let filter = FindByTicker::new(SubGroupTestC10::ticker(), SuperGroupTestC1::ticker());
        let mut iter = SubGroup::filter_map(filter);
        assert_eq!(Some(SubGroupTestC10::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }
}
