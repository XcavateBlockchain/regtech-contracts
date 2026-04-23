use anchor_lang::prelude::*;

#[constant]
pub const CONFIG_SEED: &[u8] = b"config";

#[constant]
pub const PARTNER_SEED: &[u8] = b"partner";

#[constant]
pub const MODULE_SEED: &[u8] = b"module";

#[constant]
pub const ATTEMPT_SEED: &[u8] = b"attempt";

#[constant]
pub const ENROLLMENT_SEED: &[u8] = b"enrollment";

#[constant]
pub const CREDENTIAL_SEED: &[u8] = b"credential";

pub const BPS_DENOMINATOR: u16 = 10_000;

pub const MAX_NAME_LEN: usize = 64;
pub const MAX_MODULE_CODE_LEN: usize = 64;
pub const MAX_URI_LEN: usize = 256;

pub const MPL_CORE_PROGRAM_ID: Pubkey = pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

/// Discriminator for `mpl_core::types::Key::CollectionV1` (variant index 5,
/// Borsh-encoded as a single u8). Checked against mpl-core v0.12.0 source.
/// Re-verify if Metaplex cuts a major version.
pub const MPL_CORE_KEY_COLLECTION_V1: u8 = 5;

/// Byte offset of `update_authority: Pubkey` inside a `CollectionV1`
/// account. Byte 0 is the Key discriminator, bytes 1..33 are the pubkey.
/// Checked against mpl-core v0.12.0 source.
pub const MPL_CORE_COLLECTION_UPDATE_AUTHORITY_OFFSET: usize = 1;
