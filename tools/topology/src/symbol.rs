use sha2::{Digest as _, Sha256};

use crate::{channel, currency};

#[derive(Debug, PartialEq, Eq)]
pub struct Symbol {
    path: String,
    symbol: String,
}

impl Symbol {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

#[derive(Debug)]
pub(crate) struct Builder(Inner);

impl Builder {
    // TODO [1.79]
    //  Replace with following:
    //  ```
    //  pub const fn new() -> Self {
    //      const { Self(Inner::Native) }
    //  }
    //  ```
    pub const NEW: Self = Self(Inner::Native);

    pub fn add_channel(&mut self, channel: &channel::Id) {
        match &mut self.0 {
            Inner::Native => self.0 = Inner::Ibc(Ibc::new(channel)),
            Inner::Ibc(ibc_symbol) => ibc_symbol.add_channel(channel),
        }
    }

    pub fn add_symbol(self, symbol: &currency::Id) -> Symbol {
        match self.0 {
            Inner::Native => Symbol {
                path: symbol.as_ref().into(),
                symbol: symbol.as_ref().into(),
            },
            Inner::Ibc(ibc_symbol) => ibc_symbol.add_symbol(symbol),
        }
    }
}

#[derive(Debug)]
enum Inner {
    Native,
    Ibc(Ibc),
}

#[derive(Debug)]
struct Ibc {
    path: String,
}

impl Ibc {
    const SHA2_256_OUTPUT_SIZE: usize = 32;

    const BYTE_HALVES_PER_CHAR: usize = 2;

    const SYMBOL_PREFIX: &'static str = "ibc/";

    const OUTPUT_SYMBOL_LENGTH: usize =
        Self::SYMBOL_PREFIX.len() + (Self::SHA2_256_OUTPUT_SIZE * Self::BYTE_HALVES_PER_CHAR);

    fn new(outmost_channel: &channel::Id) -> Self {
        let mut instance = Self {
            path: String::new(),
        };

        instance.add_channel(outmost_channel);

        instance
    }

    fn add_channel(&mut self, channel: &channel::Id) {
        self.path.push_str("transfer/");

        self.path.push_str(channel.as_ref());

        self.path.push('/');
    }

    fn add_symbol(mut self, symbol: &currency::Id) -> Symbol {
        self.path.push_str(symbol.as_ref());

        let symbol = Self::digest_symbol(&self.path);

        Symbol {
            path: self.path,
            symbol,
        }
    }

    fn digest_symbol(path: &str) -> String {
        let mut symbol = String::new();

        symbol.reserve_exact(Self::OUTPUT_SYMBOL_LENGTH);

        symbol.push_str(Self::SYMBOL_PREFIX);

        let digest: [u8; Self::SHA2_256_OUTPUT_SIZE] = Sha256::digest(path).into();

        Self::map_into_hex_iter(digest).for_each(|ch| symbol.push(ch));

        symbol
    }

    fn map_into_hex_iter<const N: usize>(digest: [u8; N]) -> impl Iterator<Item = char> {
        Self::map_into_byte_halves(digest)
            .into_iter()
            .flat_map(|ByteHalves { low, high }| [high, low])
            .map(|half_byte| {
                if half_byte < 10 {
                    b'0' + half_byte
                } else {
                    b'A' + (half_byte - 10)
                }
                .into()
            })
    }

    fn map_into_byte_halves<const N: usize>(digest: [u8; N]) -> [ByteHalves; N] {
        digest.map(|byte| ByteHalves {
            high: byte >> 4,
            low: byte & 0xF,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteHalves {
    low: u8,
    high: u8,
}

const _: () = {
    let 68 = Ibc::OUTPUT_SYMBOL_LENGTH else {
        panic!("Output symbol length didn't match well-known value!")
    };
};

#[test]
fn test_into_byte_halves() {
    const VALUES_ARRAY_LENGTH: usize = 256;

    const fn values() -> [u8; VALUES_ARRAY_LENGTH] {
        let mut values = [0; VALUES_ARRAY_LENGTH];

        let mut index = 0;

        let mut value: u8 = 0;

        while index != values.len() {
            values[index] = value;

            index += 1;

            value = value.wrapping_add(1);
        }

        values
    }

    const INPUT_VALUES: [u8; VALUES_ARRAY_LENGTH] = values();

    const OUTPUT_VALUES: [ByteHalves; VALUES_ARRAY_LENGTH] = {
        let mut values = [ByteHalves { low: 0, high: 0 }; VALUES_ARRAY_LENGTH];

        let mut index = 0;

        while index != values.len() {
            values[index] = ByteHalves {
                low: INPUT_VALUES[index] % 16,
                high: INPUT_VALUES[index] / 16,
            };

            index += 1;
        }

        values
    };

    let byte_halves = Ibc::map_into_byte_halves(INPUT_VALUES);

    assert_eq!(
        byte_halves[0x01],
        ByteHalves {
            low: 0x1,
            high: 0x0
        }
    );

    assert_eq!(
        byte_halves[0x0F],
        ByteHalves {
            low: 0xF,
            high: 0x0
        }
    );

    assert_eq!(
        byte_halves[0x10],
        ByteHalves {
            low: 0x0,
            high: 0x1
        }
    );

    assert_eq!(
        byte_halves[0x11],
        ByteHalves {
            low: 0x1,
            high: 0x1
        }
    );

    assert_eq!(
        byte_halves[0x1F],
        ByteHalves {
            low: 0xF,
            high: 0x1
        }
    );

    assert_eq!(
        byte_halves[0xF0],
        ByteHalves {
            low: 0x0,
            high: 0xF
        }
    );

    assert_eq!(
        byte_halves[0xF1],
        ByteHalves {
            low: 0x1,
            high: 0xF
        }
    );

    assert_eq!(
        byte_halves[0xFF],
        ByteHalves {
            low: 0xF,
            high: 0xF
        }
    );

    assert_eq!(byte_halves, OUTPUT_VALUES);
}

#[test]
fn test_into_hex_iter() {
    assert_eq!(
        &*Ibc::map_into_hex_iter([
            0x0, 0x1, 0xA, 0xB, 0x10, 0x11, 0x1A, 0x1B, 0xA0, 0xA1, 0xAA, 0xAB
        ])
        .collect::<Vec<_>>(),
        &b"00010A0B10111A1BA0A1AAAB".map(char::from),
    );
}

#[test]
fn test_digest_symbol() {
    assert_eq!(
        &*Ibc::digest_symbol(
            "transfer/channel-10001/transfer/channel-1001/transfer/\
                channel-101/transfer/channel-11/chostc"
        ),
        "ibc/127DE8C2179188419C34E69BFF735D4D2D443C31F39272DF5970DAFFEF5CCBC0",
    );
}
