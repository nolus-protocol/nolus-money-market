use std::{borrow::Borrow, collections::BTreeMap};

use serde::de::{Deserialize, Deserializer, Error as _};

use crate::{raw::RawTopology, str::container};

mod raw;
mod str;

#[derive(Debug, PartialEq, Eq)]
pub struct NetworkAndChannel<Container>
where
    Container: container::Container,
{
    pub network: Container,
    pub channel: Container,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChannelPair<Container>
where
    Container: container::Container,
{
    pub network_a: NetworkAndChannel<Container>,
    pub network_b: NetworkAndChannel<Container>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AmmPool<Container>
where
    Container: container::Container,
{
    pub id: Container,
    pub first_token: Container,
    pub second_token: Container,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NativeCurrency<Container>
where
    Container: container::Container,
{
    pub name: Container,
    pub symbol: Container,
    pub decimal_digits: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Currency<Container>
where
    Container: container::Container,
{
    Native(NativeCurrency<Container>),
    Foreign {
        network: Container,
        currency: Container,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct CurrencyWithIcon<Container>
where
    Container: container::Container,
{
    pub currency: Currency<Container>,
    pub icon: Option<Container>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Network<Container>
where
    Container: container::Container,
{
    pub currencies: BTreeMap<Container, CurrencyWithIcon<Container>>,
    pub amm_pools: Vec<AmmPool<Container>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HostCurrency<Container>
where
    Container: container::Container,
{
    pub id: Container,
    pub native: NativeCurrency<Container>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HostNetwork<Container>
where
    Container: container::Container,
{
    pub name: Container,
    pub currency: HostCurrency<Container>,
}

macro_rules! newtype_str_container {
    ($($type: ident),+ $(,)?) => {
        $(
            #[repr(transparent)]
            #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
            pub struct $type<Container>(Container)
            where
                Container: container::Container;

            impl<Container> From<Container> for $type<Container>
            where
                Container: container::Container,
            {
                fn from(value: Container) -> Self {
                    Self(value)
                }
            }

            impl<Container> AsRef<Container> for $type<Container>
            where
                Container: container::Container,
            {
                fn as_ref(&self) -> &Container {
                    &self.0
                }
            }

            impl<Container> AsRef<str> for $type<Container>
            where
                Container: container::Container,
            {
                fn as_ref(&self) -> &str {
                    &self.0
                }
            }

            impl<Container> Borrow<str> for $type<Container>
            where
                Container: container::Container,
            {
                fn borrow(&self) -> &str {
                    &self.0
                }
            }
        )+
    };
}

newtype_str_container![SourceNetwork, DestinationNetwork, ChannelId];

impl<Container> SourceNetwork<Container>
where
    Container: container::Container,
{
    pub(crate) fn into_inner(self) -> Container {
        self.0
    }
}

impl<Container> DestinationNetwork<Container>
where
    Container: container::Container,
{
    pub(crate) fn into_inner(self) -> Container {
        self.0
    }
}

pub type NetworkChannels<Container> = BTreeMap<DestinationNetwork<Container>, ChannelId<Container>>;

pub type NetworksChannels<Container> =
    BTreeMap<SourceNetwork<Container>, NetworkChannels<Container>>;

#[derive(Debug, PartialEq, Eq)]
pub struct Topology<Container>
where
    Container: container::Container,
{
    pub host_network: HostNetwork<Container>,
    pub networks: BTreeMap<Container, Network<Container>>,
    pub channels: NetworksChannels<Container>,
}

impl<'de, Container> Deserialize<'de> for Topology<Container>
where
    Container: container::Container,
    Container::DeduplicationContext: Sized,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        RawTopology::deserialize(deserializer)
            .and_then(|raw| raw.try_into().map_err(D::Error::custom))
    }
}
