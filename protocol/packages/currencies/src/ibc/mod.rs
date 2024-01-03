use compile_time_sha2::{consts::SHA2_256_OUTPUT_LENGTH, Sha256};

use currency::SymbolSlice;

pub(crate) mod macros;

type ChannelId = u32;

type ChannelIdsSlice = [ChannelId];

/// "ibc/" + hexadecimal representation of the SHA2-256 digest,
/// which requires two alphanumerical bytes for each raw byte.
pub(crate) type IbcSymbolArray = [u8; OUTPUT_PREFIX.len() + (SHA2_256_OUTPUT_LENGTH * 2)];

const OUTPUT_PREFIX: [u8; 4] = *b"ibc/";

#[cfg(not(test))]
const LOCAL_TO_DEX_CHANNEL: ChannelId = resolve_channel_from_feature_matrix(&[
    (cfg!(feature = "net_dev"), cfg!(feature = "astroport"), 116),
    (cfg!(feature = "net_dev"), cfg!(feature = "osmosis"), 0),
    (
        cfg!(feature = "net_test"),
        cfg!(feature = "astroport"),
        1990,
    ),
    (cfg!(feature = "net_test"), cfg!(feature = "osmosis"), 1993),
    (
        cfg!(feature = "net_main"),
        cfg!(feature = "astroport"),
        3839,
    ),
    (cfg!(feature = "net_main"), cfg!(feature = "osmosis"), 0),
]);
#[cfg(test)]
const LOCAL_TO_DEX_CHANNEL: ChannelId = 0;

#[cfg(not(test))]
const DEX_TO_LOCAL_CHANNEL: ChannelId = resolve_channel_from_feature_matrix(&[
    (cfg!(feature = "net_dev"), cfg!(feature = "astroport"), 209),
    (cfg!(feature = "net_dev"), cfg!(feature = "osmosis"), 109),
    (cfg!(feature = "net_test"), cfg!(feature = "astroport"), 208),
    (cfg!(feature = "net_test"), cfg!(feature = "osmosis"), 4508),
    (cfg!(feature = "net_main"), cfg!(feature = "astroport"), 44),
    (cfg!(feature = "net_main"), cfg!(feature = "osmosis"), 783),
]);
#[cfg(test)]
const DEX_TO_LOCAL_CHANNEL: ChannelId = 0;

#[inline]
pub(crate) const fn bank_symbol(channels: &[u32], symbol: &SymbolSlice) -> IbcSymbolArray {
    ibc_symbol(Some(LOCAL_TO_DEX_CHANNEL), channels, symbol)
}

#[inline]
pub(crate) const fn local_native_on_dex_symbol(symbol: &SymbolSlice) -> IbcSymbolArray {
    ibc_symbol(Some(DEX_TO_LOCAL_CHANNEL), &[], symbol)
}

#[inline]
pub(crate) const fn dex_symbol(channels: &[u32], symbol: &SymbolSlice) -> IbcSymbolArray {
    ibc_symbol(None, channels, symbol)
}

const fn ibc_symbol(
    prefixing_channel: Option<ChannelId>,
    channels: &ChannelIdsSlice,
    symbol: &SymbolSlice,
) -> IbcSymbolArray {
    let mut digest = Sha256::new();

    if let Some(prefixing_channel) = prefixing_channel {
        digest = digest_channel(digest, prefixing_channel);
    }

    let mut channel_index = 0;

    while channel_index < channels.len() {
        digest = digest_channel(digest, channels[channel_index]);

        channel_index += 1;
    }

    let digest = handle_result(digest.update(symbol.as_bytes())).finalize();

    let mut output = [0; 4 + 64];

    let mut output_index = 0;

    while output_index < OUTPUT_PREFIX.len() {
        output[output_index] = OUTPUT_PREFIX[output_index];

        output_index += 1;
    }

    let mut digest_index = 0;

    while digest_index < SHA2_256_OUTPUT_LENGTH {
        output[output_index] = to_hex(digest[digest_index] >> 4);
        output[output_index + 1] = to_hex(digest[digest_index] & 0b1111);

        digest_index += 1;
        output_index += 2;
    }

    assert!(output_index == output.len());

    output
}

#[cfg(not(test))]
const fn resolve_channel_from_feature_matrix(
    network_proocol_list: &[(bool, bool, ChannelId)],
) -> ChannelId {
    let mut index = 0;

    let channel_id = loop {
        if index == network_proocol_list.len() {
            panic!("Constant not defined for selected network and protocol!")
        }

        if network_proocol_list[index].0 && network_proocol_list[index].1 {
            break network_proocol_list[index].2;
        }

        index += 1;
    };

    index += 1;

    while index < network_proocol_list.len() {
        if network_proocol_list[index].0 && network_proocol_list[index].1 {
            panic!("Constant defined more than once!");
        }

        index += 1;
    }

    channel_id
}

const fn handle_result(result: Result<Sha256, compile_time_sha2::error::MessageTooLong>) -> Sha256 {
    if let Ok(digest) = result {
        digest
    } else {
        // Shouldn't be able to reach 2^61 bytes.
        unreachable!();
    }
}

const fn digest_u32_as_decimal(mut digest: Sha256, mut x: u32) -> Sha256 {
    const MAX_U32_DECIMAL_DIGITS: usize = 10;

    let mut bytes = [0; MAX_U32_DECIMAL_DIGITS];

    let mut index = MAX_U32_DECIMAL_DIGITS - 1;

    loop {
        bytes[index] = b'0' + ((x % 10) as u8);

        x /= 10;

        if x == 0 {
            break;
        }

        index -= 1;
    }

    while index < MAX_U32_DECIMAL_DIGITS {
        digest = handle_result(digest.update(&[bytes[index]]));

        index += 1;
    }

    digest
}

const fn digest_channel(digest: Sha256, channel: ChannelId) -> Sha256 {
    handle_result(
        digest_u32_as_decimal(handle_result(digest.update(b"transfer/channel-")), channel)
            .update(b"/"),
    )
}

const fn to_hex(half_byte: u8) -> u8 {
    match half_byte {
        ..=9 => b'0' + half_byte,
        10..=16 => b'A' + half_byte - 10,
        _ => panic!("Invariant broken!"),
    }
}

#[cfg(test)]
#[test]
fn test_ibc_symbols() {
    use self::macros::{bank_symbol, dex_symbol};

    const LOCAL_SYMBOL: &str =
        "ibc/ECFDE61B64BB920E087E7448C4C3FE356B7BD13A1C2153119E98816C964FE196";
    const DEX_SYMBOL: &str = "ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477";

    const fn local_symbol(channels: &ChannelIdsSlice, symbol: &SymbolSlice) -> IbcSymbolArray {
        ibc_symbol(Some(0), channels, symbol)
    }

    /* Local IBC symbol */
    assert_eq!(ibc_symbol(Some(0), &[12], "uatom"), LOCAL_SYMBOL.as_bytes());
    assert_eq!(ibc_symbol(None, &[0, 12], "uatom"), LOCAL_SYMBOL.as_bytes());
    assert_eq!(local_symbol(&[12], "uatom"), LOCAL_SYMBOL.as_bytes());
    assert_eq!(bank_symbol!([12], "uatom").0, LOCAL_SYMBOL);

    /* Dex IBC symbol */
    assert_eq!(ibc_symbol(None, &[12], "uatom"), DEX_SYMBOL.as_bytes());
    assert_eq!(dex_symbol(&[12], "uatom"), DEX_SYMBOL.as_bytes());
    assert_eq!(dex_symbol!([12], "uatom").0, DEX_SYMBOL);
}
