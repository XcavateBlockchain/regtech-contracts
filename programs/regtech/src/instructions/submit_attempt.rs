use anchor_lang::prelude::*;

use crate::constants::{ATTEMPT_SEED, BPS_DENOMINATOR, CONFIG_SEED, MODULE_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Attempt, Config, Module, Partner};

#[derive(Accounts)]
pub struct SubmitAttempt<'info> {
    pub attestor: Signer<'info>,

    /// CHECK: We only use this pubkey to re-derive the Attempt PDA below.
    /// The attestor is the one actually authorizing the submission. This is
    /// safe because `start_attempt` required this user as a Signer, so
    /// whoever controls this key has already opted in to taking the quiz.
    /// If `start_attempt` ever stops requiring that signature, the attestor
    /// could write scores against pubkeys nobody authorized, so if you
    /// change that, come back and rethink this.
    pub user: UncheckedAccount<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ RegtechError::Paused,
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = attestor @ RegtechError::NotAuthorized,
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
        mut,
        seeds = [ATTEMPT_SEED, user.key().as_ref(), &partner.partner_id, &module.module_id_hash],
        bump = attempt.bump,
    )]
    pub attempt: Account<'info, Attempt>,
}

pub(crate) fn handle_submit_attempt(ctx: Context<SubmitAttempt>, score_bps: u16) -> Result<()> {
    require!(score_bps <= BPS_DENOMINATOR, RegtechError::InvalidScore);

    let now = Clock::get()?.unix_timestamp;
    let module_cooldown = ctx.accounts.module.cooldown_seconds;
    let module_threshold = ctx.accounts.module.pass_threshold_bps;

    let attempt = &mut ctx.accounts.attempt;
    require!(!attempt.passed, RegtechError::AlreadyPassed);

    if attempt.last_attempt_at > 0 {
        let next_allowed = attempt
            .last_attempt_at
            .checked_add(module_cooldown)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
        require!(now >= next_allowed, RegtechError::CooldownNotElapsed);
    }

    let new_passed = score_bps >= module_threshold;
    let was_passed = attempt.passed;

    attempt.last_attempt_at = now;
    attempt.last_score_bps = score_bps;
    attempt.attempt_count = attempt
        .attempt_count
        .checked_add(1)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    attempt.passed = was_passed || new_passed;
    if !was_passed && new_passed {
        attempt.passed_at = Some(now);
    }

    emit!(AttemptSubmitted {
        user: attempt.user,
        partner_id: attempt.partner_id,
        module_id_hash: attempt.module_id_hash,
        attempt_count: attempt.attempt_count,
        score_bps,
        passed: attempt.passed,
        timestamp: now,
    });

    Ok(())
}

#[event]
pub struct AttemptSubmitted {
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub attempt_count: u32,
    pub score_bps: u16,
    pub passed: bool,
    pub timestamp: i64,
}
