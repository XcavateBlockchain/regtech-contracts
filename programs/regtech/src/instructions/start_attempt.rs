use anchor_lang::prelude::*;

use crate::constants::{ATTEMPT_SEED, CONFIG_SEED, ENROLLMENT_SEED, MODULE_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Attempt, Config, Enrollment, Module, Partner};

#[derive(Accounts)]
pub struct StartAttempt<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ RegtechError::Paused,
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
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

    // The enrollment gate. Anchor's typed Account<'info, Enrollment> loader
    // fails with AccountNotInitialized if this PDA doesn't exist (either the
    // partner never enrolled the user, or they revoked the enrollment and it
    // was closed). This is the maker-checker separation: partner_admin's
    // enrollment decision gates user participation, distinct from the
    // attestor's scoring decision in submit_attempt.
    #[account(
        seeds = [
            ENROLLMENT_SEED,
            user.key().as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump = enrollment.bump,
        constraint = enrollment.user == user.key() @ RegtechError::NotAuthorized,
        constraint = enrollment.partner_id == partner.partner_id @ RegtechError::NotAuthorized,
    )]
    pub enrollment: Account<'info, Enrollment>,

    #[account(
        init,
        payer = user,
        space = 8 + Attempt::INIT_SPACE,
        seeds = [ATTEMPT_SEED, user.key().as_ref(), &partner.partner_id, &module.module_id_hash],
        bump,
    )]
    pub attempt: Account<'info, Attempt>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_start_attempt(ctx: Context<StartAttempt>) -> Result<()> {
    let clock = Clock::get()?;
    let partner_id = ctx.accounts.partner.partner_id;
    let module_id_hash = ctx.accounts.module.module_id_hash;
    let user_key = ctx.accounts.user.key();

    let attempt = &mut ctx.accounts.attempt;
    attempt.user = user_key;
    attempt.partner_id = partner_id;
    attempt.module_id_hash = module_id_hash;
    attempt.last_attempt_at = 0;
    attempt.last_score_bps = 0;
    attempt.attempt_count = 0;
    attempt.passed = false;
    attempt.passed_at = None;
    attempt.bump = ctx.bumps.attempt;

    emit!(AttemptStarted {
        user: user_key,
        partner_id,
        module_id_hash,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct AttemptStarted {
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub timestamp: i64,
}
