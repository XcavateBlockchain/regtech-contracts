use anchor_lang::prelude::*;

use crate::constants::{
    BPS_DENOMINATOR, CONFIG_SEED, MAX_NAME_LEN, MPL_CORE_COLLECTION_UPDATE_AUTHORITY_OFFSET,
    MPL_CORE_KEY_COLLECTION_V1, MPL_CORE_PROGRAM_ID, PARTNER_SEED,
};
use crate::error::RegtechError;
use crate::state::{Config, Partner};

#[derive(Accounts)]
#[instruction(partner_id: [u8; 16])]
pub struct RegisterPartner<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ RegtechError::NotAuthorized,
        constraint = !config.paused @ RegtechError::Paused,
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = admin,
        space = 8 + Partner::INIT_SPACE,
        seeds = [PARTNER_SEED, &partner_id],
        bump,
    )]
    pub partner: Account<'info, Partner>,

    /// CHECK: The mpl-core Collection the partner pre-created for their
    /// credentials. We can't use a typed Anchor account here because mpl-core
    /// isn't Anchor-native, so the handler checks the owner, the account type,
    /// and the update authority by hand before trusting it.
    pub collection: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_register_partner(
    ctx: Context<RegisterPartner>,
    partner_id: [u8; 16],
    name: String,
    attestor: Pubkey,
    partner_admin: Pubkey,
    pass_threshold_bps_override: Option<u16>,
    cooldown_seconds_override: Option<i64>,
) -> Result<()> {
    require!(name.len() <= MAX_NAME_LEN, RegtechError::StringTooLong);

    let pass_threshold_bps = pass_threshold_bps_override
        .unwrap_or_else(|| ctx.accounts.config.default_pass_threshold_bps);
    let cooldown_seconds = cooldown_seconds_override
        .unwrap_or_else(|| ctx.accounts.config.default_cooldown_seconds);

    require!(
        pass_threshold_bps <= BPS_DENOMINATOR,
        RegtechError::InvalidThreshold
    );
    require!(cooldown_seconds >= 0, RegtechError::InvalidCooldown);

    let expected_partner_pda = ctx.accounts.partner.key();

    require!(
        ctx.accounts.collection.owner == &MPL_CORE_PROGRAM_ID,
        RegtechError::CollectionNotOwnedByMplCore
    );

    let update_authority = {
        let data = ctx.accounts.collection.try_borrow_data()?;
        parse_collection_update_authority(&data)?
    };

    require!(
        update_authority == expected_partner_pda,
        RegtechError::CollectionAuthorityMismatch
    );

    let clock = Clock::get()?;
    let collection_key = ctx.accounts.collection.key();
    let partner = &mut ctx.accounts.partner;
    partner.partner_id = partner_id;
    partner.name = name.clone();
    partner.credential_collection = collection_key;
    partner.attestor = attestor;
    partner.partner_admin = partner_admin;
    partner.pass_threshold_bps = pass_threshold_bps;
    partner.cooldown_seconds = cooldown_seconds;
    partner.active = true;
    partner.created_at = clock.unix_timestamp;
    partner.bump = ctx.bumps.partner;

    emit!(PartnerRegistered {
        partner_id,
        name,
        credential_collection: collection_key,
        attestor,
        partner_admin,
        pass_threshold_bps,
        cooldown_seconds,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Parses the `update_authority` pubkey out of a raw mpl-core CollectionV1
/// account. Bounds-checks before indexing so a truncated or wrong-type account
/// errors cleanly instead of panicking on a slice out of bounds.
fn parse_collection_update_authority(data: &[u8]) -> Result<Pubkey> {
    require!(
        data.len() >= MPL_CORE_COLLECTION_UPDATE_AUTHORITY_OFFSET + 32,
        RegtechError::CollectionWrongType
    );
    require!(
        data[0] == MPL_CORE_KEY_COLLECTION_V1,
        RegtechError::CollectionWrongType
    );
    Pubkey::try_from(
        &data[MPL_CORE_COLLECTION_UPDATE_AUTHORITY_OFFSET
            ..MPL_CORE_COLLECTION_UPDATE_AUTHORITY_OFFSET + 32],
    )
    .map_err(|_| error!(RegtechError::CollectionWrongType))
}

#[event]
pub struct PartnerRegistered {
    pub partner_id: [u8; 16],
    pub name: String,
    pub credential_collection: Pubkey,
    pub attestor: Pubkey,
    pub partner_admin: Pubkey,
    pub pass_threshold_bps: u16,
    pub cooldown_seconds: i64,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_collection_bytes(key_byte: u8, authority: Pubkey, extra: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(1 + 32 + extra);
        data.push(key_byte);
        data.extend_from_slice(authority.as_ref());
        data.extend(std::iter::repeat(0u8).take(extra));
        data
    }

    #[test]
    fn parses_update_authority_from_valid_collection_v1() {
        let expected = Pubkey::new_unique();
        let data = build_collection_bytes(MPL_CORE_KEY_COLLECTION_V1, expected, 128);
        let got = parse_collection_update_authority(&data).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn rejects_group_v1_key_byte() {
        // Variant 6 is GroupV1, not CollectionV1. Pinning this guards against
        // the off-by-one error where someone claims CollectionV1 == 6.
        let data = build_collection_bytes(6, Pubkey::new_unique(), 128);
        assert!(parse_collection_update_authority(&data).is_err());
    }

    #[test]
    fn rejects_asset_v1_key_byte() {
        let data = build_collection_bytes(1, Pubkey::new_unique(), 128);
        assert!(parse_collection_update_authority(&data).is_err());
    }

    #[test]
    fn rejects_truncated_account_data() {
        let data = vec![MPL_CORE_KEY_COLLECTION_V1, 0, 0, 0];
        assert!(parse_collection_update_authority(&data).is_err());
    }

    #[test]
    fn rejects_empty_account_data() {
        assert!(parse_collection_update_authority(&[]).is_err());
    }
}
