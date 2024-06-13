use std::collections::BTreeSet;

use super::container;

pub struct Pool<Container>(BTreeSet<Container>)
where
    Container: container::Container<DeduplicationContext = Self> + for<'r> From<&'r str>;

impl<Container> Pool<Container>
where
    Container: container::Container<DeduplicationContext = Self> + for<'r> From<&'r str>,
{
    pub const fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn get_or_insert(&mut self, s: &str) -> Container {
        match self.0.get(s) {
            Some(s) => s.clone(),
            None => {
                let s: Container = s.into();

                #[cfg(debug_assertions)]
                let true = self.0.insert(s.clone()) else {
                    unreachable!()
                };
                #[cfg(not(debug_assertions))]
                self.0.insert(s.clone());

                s
            }
        }
    }
}
