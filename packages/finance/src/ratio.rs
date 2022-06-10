use cosmwasm_std::Fraction;

pub(crate) struct Ratio<U> {
    nominator: U,
    denominator: U,
}

impl<U> Ratio<U> {
    pub fn new(nominator: U, denominator: U) -> Self {
        Self {
            nominator,
            denominator,
        }
    }
}
impl<U> Fraction<U> for Ratio<U>
where
    U: Default + PartialEq + Copy,
{
    fn numerator(&self) -> U {
        self.nominator
    }

    fn denominator(&self) -> U {
        self.denominator
    }

    fn inv(&self) -> Option<Self> {
        if self.nominator == U::default() {
            None
        } else {
            Some(Ratio {
                nominator: self.denominator,
                denominator: self.nominator,
            })
        }
    }
}
