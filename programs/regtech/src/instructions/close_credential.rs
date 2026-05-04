use anchor_lang::prelude::*;

use crate::constants::{CREDENTIAL_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Credential, Partner};

#[derive(Accounts)]
pub struct CloseCredential<'info> {
    pub partner_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
    )]
    pub partner: Account<'info, Partner>,

    #[account(
        mut,
        close = partner,
        seeds = [
            CREDENTIAL_SEED,
            credential.user.as_ref(),
            &partner.partner_id,
            &credential.module_id_hash,
        ],
        bump = credential.bump,
        constraint = credential.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
        constraint = credential.revoked_at.is_some() @ RegtechError::NotRevoked,
    )]
    pub credential: Account<'info, Credential>,
}

pub(crate) fn handle_close_credential(
    ctx: Context<CloseCredential>,
    reason_code: u8,
) -> Result<()> {
    let credential = &ctx.accounts.credential;

    emit!(CredentialClosed {
        actor: ctx.accounts.partner_admin.key(),
        user: credential.user,
        partner_id: credential.partner_id,
        module_id_hash: credential.module_id_hash,
        reason_code,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct CredentialClosed {
    pub actor: Pubkey,
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub reason_code: u8,
    pub timestamp: i64,
}
