use anchor_lang::prelude::*;

use crate::constants::{ENROLLMENT_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Enrollment, Partner};

// Revoke works even when the program is paused. Same logic as the
// other admin ops.
#[derive(Accounts)]
pub struct RevokeEnrollment<'info> {
    #[account(mut)]
    pub partner_admin: Signer<'info>,

    #[account(
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
    )]
    pub partner: Account<'info, Partner>,

    // `close = partner_admin` sends the Enrollment's lamports back to
    // the partner_admin who paid for it, and zeros the account. After
    // that the PDA reads back as AccountNotInitialized, which is what
    // start_attempt's loader trips on.
    #[account(
        mut,
        close = partner_admin,
        seeds = [
            ENROLLMENT_SEED,
            enrollment.user.as_ref(),
            &partner.partner_id,
            &enrollment.module_id_hash,
        ],
        bump = enrollment.bump,
        constraint = enrollment.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
    )]
    pub enrollment: Account<'info, Enrollment>,
}

pub(crate) fn handle_revoke_enrollment(
    ctx: Context<RevokeEnrollment>,
    reason_code: u8,
) -> Result<()> {
    let actor = ctx.accounts.partner_admin.key();
    let enrollment = &ctx.accounts.enrollment;

    emit!(EnrollmentRevoked {
        actor,
        user: enrollment.user,
        partner_id: enrollment.partner_id,
        module_id_hash: enrollment.module_id_hash,
        reason_code,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct EnrollmentRevoked {
    pub actor: Pubkey,
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub reason_code: u8,
    pub timestamp: i64,
}
