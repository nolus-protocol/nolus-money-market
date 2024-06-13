use std::{collections::BTreeMap, num::ParseIntError};
use std::borrow::Cow;

use serde::{
    de::{Deserializer, Error as _},
    Deserialize,
};

use crate::{
    str::container, AmmPool, ChannelId, ChannelPair, Currency, CurrencyWithIcon,
    DestinationNetwork, HostCurrency, HostNetwork, NativeCurrency, Network, NetworkAndChannel,
    NetworkChannels, NetworksChannels, SourceNetwork, Topology,
};

use self::transform::Transform;

mod transform;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct RawChannel<'r> {
    network: Cow<'r, str>,
    ch: Cow<'r, str>,
}

impl<'r, Container> Transform<Container> for RawChannel<'r>
where
    Container: container::Container,
{
    type Output = NetworkAndChannel<Container>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        NetworkAndChannel {
            network: Container::new_deduplicated(&self.network, deduplication_ctx),
            channel: Container::new_deduplicated(&self.ch, deduplication_ctx),
        }
    }
}

struct RawChannelPair<'r> {
    a: RawChannel<'r>,
    b: RawChannel<'r>,
}

impl<'r, 'de> Deserialize<'de> for RawChannelPair<'r>
where
    'de: 'r,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        struct Unchecked<'r> {
            #[serde(borrow)]
            a: RawChannel<'r>,
            b: RawChannel<'r>,
        }

        Unchecked::deserialize(deserializer).and_then(|unchecked| {
            if unchecked.a.network != unchecked.b.network {
                Ok(Self {
                    a: unchecked.a,
                    b: unchecked.b,
                })
            } else {
                cold();

                Err(D::Error::custom(
                    "Channel pair can not consist of two channels on the same network!",
                ))
            }
        })
    }
}

impl<'r, Container> Transform<Container> for RawChannelPair<'r>
where
    Container: container::Container,
{
    type Output = ChannelPair<Container>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        ChannelPair {
            network_a: self.a.transform(deduplication_ctx),
            network_b: self.b.transform(deduplication_ctx),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct RawAmmPool<'r> {
    id: Cow<'r, str>,
    token_0: Cow<'r, str>,
    token_1: Cow<'r, str>,
}

impl<'r, Container> Transform<Container> for RawAmmPool<'r>
where
    Container: container::Container,
{
    type Output = AmmPool<Container>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        AmmPool {
            id: Container::new_deduplicated(&self.id, deduplication_ctx),
            first_token: Container::new_deduplicated(&self.token_0, deduplication_ctx),
            second_token: Container::new_deduplicated(&self.token_1, deduplication_ctx),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct RawNativeCurrency<'r> {
    name: Cow<'r, str>,
    symbol: Cow<'r, str>,
    decimal_digits: Cow<'r, str>,
}

impl<'r, Container> Transform<Container> for RawNativeCurrency<'r>
where
    Container: container::Container,
{
    type Output = Result<NativeCurrency<Container>, ParseIntError>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        self.decimal_digits
            .parse()
            .map(|decimal_digits| NativeCurrency {
                name: Container::new_deduplicated(&self.name, deduplication_ctx),
                symbol: Container::new_deduplicated(&self.symbol, deduplication_ctx),
                decimal_digits,
            })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct RawForeignCurrency<'r> {
    network: Cow<'r, str>,
    currency: Cow<'r, str>,
}

impl<'r, Container> Transform<Container> for RawForeignCurrency<'r>
where
    Container: container::Container,
{
    type Output = Currency<Container>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        Currency::Foreign {
            network: Container::new_deduplicated(&self.network, deduplication_ctx),
            currency: Container::new_deduplicated(&self.currency, deduplication_ctx),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", untagged, deny_unknown_fields)]
enum RawCurrencyWithIcon<'r> {
    Native {
        native: RawNativeCurrency<'r>,
        icon: Option<Cow<'r, str>>,
    },
    Foreign {
        ibc: RawForeignCurrency<'r>,
        icon: Option<Cow<'r, str>>,
    },
}

impl<'r, Container> Transform<Container> for RawCurrencyWithIcon<'r>
where
    Container: container::Container,
{
    type Output = Result<CurrencyWithIcon<Container>, ParseIntError>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        match self {
            RawCurrencyWithIcon::Native { native, icon } => native
                .transform(deduplication_ctx)
                .map(|currency| (Currency::Native(currency), icon)),
            RawCurrencyWithIcon::Foreign { ibc, icon } => {
                Ok((ibc.transform(deduplication_ctx), icon))
            }
        }
        .map(|(currency, icon)| CurrencyWithIcon {
            currency,
            icon: icon.map(|icon| Container::new_deduplicated(&icon, deduplication_ctx)),
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct RawNetwork<'r> {
    #[serde(borrow)]
    currencies: BTreeMap<Cow<'r, str>, RawCurrencyWithIcon<'r>>,
    #[serde(default)]
    amm_pools: Vec<RawAmmPool<'r>>,
}

impl<'r, Container> Transform<Container> for RawNetwork<'r>
where
    Container: container::Container,
{
    type Output = Result<Network<Container>, Error<Container>>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        self.amm_pools
            .iter()
            .try_for_each(|amm_pool| {
                if !self.currencies.contains_key(&amm_pool.token_0) {
                    Err(Error::amm_pool_referring_to_undefined_currency(
                        &amm_pool.id,
                        &amm_pool.token_0,
                        deduplication_ctx,
                    ))
                } else if !self.currencies.contains_key(&amm_pool.token_1) {
                    Err(Error::amm_pool_referring_to_undefined_currency(
                        &amm_pool.id,
                        &amm_pool.token_1,
                        deduplication_ctx,
                    ))
                } else {
                    Ok(())
                }
            })
            .and_then(|()| {
                self.currencies
                    .into_iter()
                    .map(|(name, currency_with_icon)| {
                        currency_with_icon
                            .transform(deduplication_ctx)
                            .map(|currency_with_icon| {
                                (
                                    Container::new_deduplicated(&name, deduplication_ctx),
                                    currency_with_icon,
                                )
                            })
                    })
                    .collect::<Result<_, _>>()
                    .map(|currencies| Network {
                        currencies,
                        amm_pools: self
                            .amm_pools
                            .into_iter()
                            .map(|amm_pool| amm_pool.transform(deduplication_ctx))
                            .collect(),
                    })
                    .map_err(Error::parse_decimal_places)
            })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct RawHostCurrency<'r> {
    #[serde(borrow)]
    id: Cow<'r, str>,
    native: RawNativeCurrency<'r>,
}

impl<'r, Container> Transform<Container> for RawHostCurrency<'r>
where
    Container: container::Container,
{
    type Output = Result<HostCurrency<Container>, ParseIntError>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        self.native
            .transform(deduplication_ctx)
            .map(|native| HostCurrency {
                id: Container::new_deduplicated(&self.id, deduplication_ctx),
                native,
            })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct RawHostNetwork<'r> {
    #[serde(borrow)]
    name: Cow<'r, str>,
    currency: RawHostCurrency<'r>,
}

impl<'r, Container> Transform<Container> for RawHostNetwork<'r>
where
    Container: container::Container,
{
    type Output = Result<HostNetwork<Container>, ParseIntError>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        self.currency
            .transform(deduplication_ctx)
            .map(|currency| HostNetwork {
                name: Container::new_deduplicated(&self.name, deduplication_ctx),
                currency,
            })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct RawTopology<'r> {
    #[serde(borrow)]
    host_network: RawHostNetwork<'r>,
    networks: BTreeMap<Cow<'r, str>, RawNetwork<'r>>,
    channels: Vec<RawChannelPair<'r>>,
    #[serde(
        rename = "definitions",
        deserialize_with = "deserialize_definitions",
        default,
        skip_serializing
    )]
    _definitions: (),
}

impl<'r> RawTopology<'r> {
    fn networks_exists<Container>(
        host_network: &HostNetwork<Container>,
        networks: &BTreeMap<Cow<'r, str>, RawNetwork<'r>>,
        channel_pair: &RawChannelPair<'r>,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Result<(), Error<Container>>
    where
        Container: container::Container,
    {
        if (channel_pair.a.network == host_network.name.borrow()
            || networks.contains_key(&channel_pair.a.network))
            && (channel_pair.b.network == host_network.name.borrow()
                || networks.contains_key(&channel_pair.b.network))
        {
            Ok(())
        } else {
            Err(Error::channel_referring_to_undefined_network(
                &channel_pair.a.network,
                &channel_pair.b.network,
                deduplication_ctx,
            ))
        }
    }

    fn currencies_and_connecting_channels_exist<Container>(
        host_network: &HostNetwork<Container>,
        networks: &BTreeMap<Cow<'r, str>, RawNetwork<'r>>,
        channels: &NetworksChannels<Container>,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Result<(), Error<Container>>
    where
        Container: container::Container,
    {
        Self::all_foreign_currencies(networks, channels).try_for_each(|(channels, currency)| {
            Self::validate_channel_exists(channels, currency, deduplication_ctx).and_then(|()| {
                if currency.network == host_network.name.borrow() {
                    if currency.currency == host_network.currency.id.borrow() {
                        Ok(())
                    } else {
                        cold();

                        Err(Error::foreign_currency_referring_to_undefined_currency(
                            &currency.currency,
                            &currency.network,
                            deduplication_ctx,
                        ))
                    }
                } else {
                    Self::validate_non_host_foreign_currency(networks, currency, deduplication_ctx)
                }
            })
        })
    }

    fn all_foreign_currencies<'t, Container>(
        networks: &'t BTreeMap<Cow<'_, str>, RawNetwork<'r>>,
        channels: &'t NetworksChannels<Container>,
    ) -> impl Iterator<
        Item = (
            Option<&'t NetworkChannels<Container>>,
            &'t RawForeignCurrency<'r>,
        ),
    >
    where
        Container: container::Container,
    {
        networks.iter().flat_map(|(name, network)| {
            network.currencies.values().filter_map(|currency| {
                if let RawCurrencyWithIcon::Foreign { ibc, .. } = currency {
                    Some((channels.get(name.as_ref()), ibc))
                } else {
                    cold();

                    None
                }
            })
        })
    }

    fn validate_channel_exists<Container>(
        channels: Option<&NetworkChannels<Container>>,
        currency: &RawForeignCurrency<'r>,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Result<(), Error<Container>>
    where
        Container: container::Container,
    {
        channels
            .and_then(|channels| {
                if channels.contains_key(currency.network.as_ref()) {
                    Some(())
                } else {
                    cold();

                    None
                }
            })
            .ok_or_else(
                #[cold]
                || {
                    Error::foreign_currency_referring_to_disconnected_network(
                        &currency.currency,
                        &currency.network,
                        deduplication_ctx,
                    )
                },
            )
    }

    fn validate_non_host_foreign_currency<Container>(
        networks: &BTreeMap<Cow<'_, str>, RawNetwork<'r>>,
        currency: &RawForeignCurrency<'r>,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Result<(), Error<Container>>
    where
        Container: container::Container,
    {
        networks
            .get(&currency.network)
            .ok_or_else(
                #[cold]
                || {
                    Error::foreign_currency_referring_to_undefined_network(
                        &currency.currency,
                        &currency.network,
                        deduplication_ctx,
                    )
                },
            )
            .and_then(|network| {
                if network.currencies.contains_key(&currency.currency) {
                    Ok(())
                } else {
                    cold();

                    Err(Error::foreign_currency_referring_to_undefined_currency(
                        &currency.currency,
                        &currency.network,
                        deduplication_ctx,
                    ))
                }
            })
    }

    fn transform_networks<Container>(
        raw: BTreeMap<Cow<'r, str>, RawNetwork<'r>>,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Result<BTreeMap<Container, Network<Container>>, Error<Container>>
    where
        Container: container::Container,
    {
        raw.into_iter()
            .map(|(name, network)| {
                network.transform(deduplication_ctx).map(|network| {
                    (
                        Container::new_deduplicated(&name, deduplication_ctx),
                        network,
                    )
                })
            })
            .collect()
    }

    fn transform_channels<Container>(
        channels: Vec<RawChannelPair<'r>>,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Result<NetworksChannels<Container>, Error<Container>>
    where
        Container: container::Container,
    {
        let mut transformed_channels: NetworksChannels<Container> = NetworksChannels::new();

        channels
            .into_iter()
            .map(|raw_channel_pair| raw_channel_pair.transform(deduplication_ctx))
            .flat_map(
                |ChannelPair::<Container> {
                     network_a: a,
                     network_b: b,
                 }| {
                    [
                        (
                            SourceNetwork(a.network.clone()),
                            DestinationNetwork(b.network.clone()),
                            ChannelId(a.channel),
                        ),
                        (
                            SourceNetwork(b.network),
                            DestinationNetwork(a.network),
                            ChannelId(b.channel),
                        ),
                    ]
                },
            )
            .try_for_each(|(source, destination, channel)| {
                if transformed_channels
                    .entry(source.clone())
                    .or_default()
                    .insert(destination.clone(), channel)
                    .is_none()
                {
                    Ok(())
                } else {
                    Err(Error::channel_definition_duplication(
                        source.into_inner(),
                        destination.into_inner(),
                    ))
                }
            })
            .map(|()| transformed_channels)
    }
}

impl<'r, Container> Transform<Container> for RawTopology<'r>
where
    Container: container::Container,
{
    type Output = Result<Topology<Container>, Error<Container>>;

    fn transform(self, deduplication_ctx: &mut Container::DeduplicationContext) -> Self::Output {
        self.host_network
            .transform(deduplication_ctx)
            .map_err(Error::ParseDecimalPlaces)
            .and_then(|host_network| {
                self.channels
                    .iter()
                    .try_for_each(|raw_channel_pair| {
                        Self::networks_exists(
                            &host_network,
                            &self.networks,
                            raw_channel_pair,
                            deduplication_ctx,
                        )
                    })
                    .and_then(|()| Self::transform_channels(self.channels, deduplication_ctx))
                    .and_then(|channels| {
                        Self::currencies_and_connecting_channels_exist(
                            &host_network,
                            &self.networks,
                            &channels,
                            deduplication_ctx,
                        )
                        .and_then(|()| {
                            Self::transform_networks(self.networks, deduplication_ctx).map(
                                |networks| Topology {
                                    host_network,
                                    networks,
                                    channels,
                                },
                            )
                        })
                    })
            })
    }
}

impl<'r, Container> TryFrom<RawTopology<'r>> for Topology<Container>
where
    Container: container::Container,
    Container::DeduplicationContext: Sized,
{
    type Error = Error<Container>;

    fn try_from(raw_topology: RawTopology<'r>) -> Result<Self, Self::Error> {
        raw_topology.transform(&mut Container::new_deduplication_context())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<Container>
where
    Container: container::Container,
{
    #[error(r#"Currency, "{currency}", described as foreign is referring to an undefined network, "{remote_network}"!"#)]
    ForeignCurrencyReferringToUndefinedNetwork {
        currency: Container,
        remote_network: Container,
    },
    #[error(r#"Currency, "{currency}", described as foreign is referring to a network, "{remote_network}", which does not define such currency!"#)]
    ForeignCurrencyReferringToUndefinedCurrency {
        currency: Container,
        remote_network: Container,
    },
    #[error(r#"Currency, "{currency}", described as foreign is referring to a network, "{remote_network}", which is not directly connected!"#)]
    ForeignCurrencyReferringToDisconnectedNetwork {
        currency: Container,
        remote_network: Container,
    },
    #[error("AmmPoolReferringToUndefinedCurrency")]
    AmmPoolReferringToUndefinedCurrency {
        amm_pool: Container,
        currency: Container,
    },
    #[error(r#"Encountered channel which is referring to an undefined network! Network A: "{network_a}"; Network B: "{network_b}""#)]
    ChannelReferringToUndefinedNetwork {
        network_a: Container,
        network_b: Container,
    },
    #[error("Failed to parse decimal places of currency! Cause: {_0}")]
    ParseDecimalPlaces(ParseIntError),
    #[error(r#"Encountered channel definition which has a duplicate! Network A: "{network_a}"; Network B: "{network_b}""#)]
    ChannelDefinitionDuplication {
        network_a: Container,
        network_b: Container,
    },
}

impl<Container> Error<Container>
where
    Container: container::Container,
{
    #[cold]
    #[inline]
    fn foreign_currency_referring_to_undefined_network(
        currency: &str,
        remote_network: &str,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Self {
        Self::ForeignCurrencyReferringToUndefinedNetwork {
            currency: Container::new_deduplicated(currency, deduplication_ctx),
            remote_network: Container::new_deduplicated(remote_network, deduplication_ctx),
        }
    }

    #[cold]
    #[inline]
    fn foreign_currency_referring_to_undefined_currency(
        currency: &str,
        remote_network: &str,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Self {
        Self::ForeignCurrencyReferringToUndefinedCurrency {
            currency: Container::new_deduplicated(currency, deduplication_ctx),
            remote_network: Container::new_deduplicated(remote_network, deduplication_ctx),
        }
    }

    #[cold]
    #[inline]
    fn foreign_currency_referring_to_disconnected_network(
        currency: &str,
        remote_network: &str,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Self {
        Self::ForeignCurrencyReferringToDisconnectedNetwork {
            currency: Container::new_deduplicated(currency, deduplication_ctx),
            remote_network: Container::new_deduplicated(remote_network, deduplication_ctx),
        }
    }

    #[cold]
    #[inline]
    fn amm_pool_referring_to_undefined_currency(
        amm_pool: &str,
        currency: &str,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Self {
        Self::AmmPoolReferringToUndefinedCurrency {
            amm_pool: Container::new_deduplicated(amm_pool, deduplication_ctx),
            currency: Container::new_deduplicated(currency, deduplication_ctx),
        }
    }

    #[cold]
    #[inline]
    fn channel_referring_to_undefined_network(
        network_a: &str,
        network_b: &str,
        deduplication_ctx: &mut Container::DeduplicationContext,
    ) -> Self {
        Self::ChannelReferringToUndefinedNetwork {
            network_a: Container::new_deduplicated(network_a, deduplication_ctx),
            network_b: Container::new_deduplicated(network_b, deduplication_ctx),
        }
    }

    #[cold]
    #[inline]
    fn parse_decimal_places(error: ParseIntError) -> Self {
        Self::ParseDecimalPlaces(error)
    }

    #[cold]
    #[inline]
    fn channel_definition_duplication(network_a: Container, network_b: Container) -> Self {
        Self::ChannelDefinitionDuplication {
            network_a,
            network_b,
        }
    }
}

#[inline]
#[cold]
fn cold() {}

fn deserialize_definitions<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    serde_json::Value::deserialize(deserializer).map(|_| ())
}
