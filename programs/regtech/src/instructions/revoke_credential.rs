use anchor_lang::prelude::*;

use crate::constants::{CREDENTIAL_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Credential, Partner};

#[derive(Accounts)]
pub struct RevokeCredential<'info> {
    pub partner_admin: Signer<'info>,

    #[account(
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
    )]
    pub partner: Account<'info, Partner>,

    #[account(
        mut,
        seeds = [
            CREDENTIAL_SEED,
            credential.user.as_ref(),
            &partner.partner_id,
            &credential.module_id_hash,
        ],
        bump = credential.bump,
        constraint = credential.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
        constraint = credential.revoked_at.is_none() @ RegtechError::AlreadyRevoked,
    )]
    pub credential: Account<'info, Credential>,
}

pub(crate) fn handle_revoke_credential(
    ctx: Context<RevokeCredential>,
    reason_code: u8,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let credential = &mut ctx.accounts.credential;
    credential.revoked_at = Some(now);

    emit!(CredentialRevoked {
        actor: ctx.accounts.partner_admin.key(),
        user: credential.user,
        partner_id: credential.partner_id,
        module_id_hash: credential.module_id_hash,
        reason_code,
        revoked_at: now,
    });

    Ok(())
}

#[event]
pub struct CredentialRevoked {
    pub actor: Pubkey,
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub reason_code: u8,
    pub revoked_at: i64,
}
