use std::str;

const PACKAGE_NAME: &[u8] = env!("CARGO_PKG_NAME").as_bytes();

const CARGO_SUBCOMMAND_PREFIX: &[u8] = b"cargo-";

const SUBCOMMAND_NAME_LENGTH: usize = if CARGO_SUBCOMMAND_PREFIX.len() < PACKAGE_NAME.len() {
    PACKAGE_NAME.len() - CARGO_SUBCOMMAND_PREFIX.len()
} else {
    unimplemented!()
};

const _: () = {
    let mut index = 0;

    while index < CARGO_SUBCOMMAND_PREFIX.len() {
        if PACKAGE_NAME[index] != CARGO_SUBCOMMAND_PREFIX[index] {
            unimplemented!()
        }

        index += 1;
    }
};

const SUBCOMMAND_NAME_ARRAY: [u8; SUBCOMMAND_NAME_LENGTH] = {
    let mut array = [0; SUBCOMMAND_NAME_LENGTH];

    let mut index = 0;

    while index < SUBCOMMAND_NAME_LENGTH {
        array[index] = PACKAGE_NAME[CARGO_SUBCOMMAND_PREFIX.len() + index];

        index += 1;
    }

    array
};

pub(crate) const CARGO_SUBCOMMAND_NAME: &str = {
    if let Ok(s) = str::from_utf8(&SUBCOMMAND_NAME_ARRAY) {
        s
    } else {
        unreachable!()
    }
};
