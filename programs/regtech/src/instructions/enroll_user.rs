use anchor_lang::prelude::*;

use crate::constants::{ENROLLMENT_SEED, MODULE_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Enrollment, Module, Partner};

#[derive(Accounts)]
pub struct EnrollUser<'info> {
    #[account(mut)]
    pub partner_admin: Signer<'info>,

    /// CHECK: only used as a seed for the Enrollment PDA and recorded on it.
    /// Not required to sign. The partner_admin is the authorizing party here.
    pub user: UncheckedAccount<'info>,

    #[account(
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
        constraint = partner.active @ RegtechError::PartnerInactive,
    )]
    pub partner: Account<'info, Partner>,

    #[account(
        seeds = [MODULE_SEED, &partner.partner_id, &module.module_id_hash],
        bump = module.bump,
        constraint = module.active @ RegtechError::ModuleInactive,
        constraint = module.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
    )]
    pub module: Account<'info, Module>,

    #[account(
        init,
        payer = partner_admin,
        space = 8 + Enrollment::INIT_SPACE,
        seeds = [
            ENROLLMENT_SEED,
            user.key().as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump,
    )]
    pub enrollment: Account<'info, Enrollment>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_enroll_user(
    ctx: Context<EnrollUser>,
    reason_code: u8,
) -> Result<()> {
    let clock = Clock::get()?;
    let partner_id = ctx.accounts.partner.partner_id;
    let module_id_hash = ctx.accounts.module.module_id_hash;
    let user_key = ctx.accounts.user.key();
    let enrolled_by = ctx.accounts.partner_admin.key();

    let enrollment = &mut ctx.accounts.enrollment;
    enrollment.user = user_key;
    enrollment.partner_id = partner_id;
    enrollment.module_id_hash = module_id_hash;
    enrollment.enrolled_at = clock.unix_timestamp;
    enrollment.enrolled_by = enrolled_by;
    enrollment.reason_code = reason_code;
    enrollment.bump = ctx.bumps.enrollment;

    emit!(UserEnrolled {
        actor: enrolled_by,
        user: user_key,
        partner_id,
        module_id_hash,
        reason_code,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct UserEnrolled {
    pub actor: Pubkey,
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub reason_code: u8,
    pub timestamp: i64,
}
