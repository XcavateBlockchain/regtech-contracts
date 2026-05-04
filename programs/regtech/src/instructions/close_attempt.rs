use anchor_lang::prelude::*;

use crate::constants::{ATTEMPT_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Attempt, Partner};

#[derive(Accounts)]
pub struct CloseAttempt<'info> {
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
            ATTEMPT_SEED,
            attempt.user.as_ref(),
            &partner.partner_id,
            &attempt.module_id_hash,
        ],
        bump = attempt.bump,
        constraint = attempt.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
    )]
    pub attempt: Account<'info, Attempt>,
}

pub(crate) fn handle_close_attempt(
    ctx: Context<CloseAttempt>,
    reason_code: u8,
) -> Result<()> {
    let attempt = &ctx.accounts.attempt;

    emit!(AttemptClosed {
        actor: ctx.accounts.partner_admin.key(),
        user: attempt.user,
        partner_id: attempt.partner_id,
        module_id_hash: attempt.module_id_hash,
        reason_code,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct AttemptClosed {
    pub actor: Pubkey,
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub reason_code: u8,
    pub timestamp: i64,
}
