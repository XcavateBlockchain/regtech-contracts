use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, system_instruction};

use crate::constants::{ATTEMPT_SEED, CONFIG_SEED, ENROLLMENT_SEED, MODULE_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Attempt, Config, Enrollment, Module, Partner};

#[derive(Accounts)]
pub struct StartAttempt<'info> {
    pub attestor: Signer<'info>,

    /// CHECK: Subject of the attempt, used for PDA derivation and the
    /// audit trail. Doesn't sign. Authorization comes from the attestor
    /// (via has_one on Partner) and the Enrollment PDA that partner_admin
    /// had to create earlier.
    pub user: UncheckedAccount<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ RegtechError::Paused,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
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

    /// CHECK: Created via CPI in the handler. PDA seeds verified by Anchor.
    #[account(
        mut,
        seeds = [ATTEMPT_SEED, user.key().as_ref(), &partner.partner_id, &module.module_id_hash],
        bump,
    )]
    pub attempt: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_start_attempt(ctx: Context<StartAttempt>) -> Result<()> {
    let clock = Clock::get()?;
    let partner_id = ctx.accounts.partner.partner_id;
    let module_id_hash = ctx.accounts.module.module_id_hash;
    let user_key = ctx.accounts.user.key();
    let attempt_bump = ctx.bumps.attempt;

    // Quiz quota check runs before vault math on purpose. If both would
    // fail, the partner should hear "you're out of quizzes, buy more"
    // rather than "our rent accounting is off".
    {
        let partner = &mut ctx.accounts.partner;
        let available = partner
            .quizzes_purchased
            .checked_sub(partner.quizzes_consumed)
            .and_then(|v| v.checked_sub(partner.quizzes_refunded))
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
        require!(available > 0, RegtechError::QuizQuotaExhausted);
        partner.quizzes_consumed = partner
            .quizzes_consumed
            .checked_add(1)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    }

    let rent = Rent::get()?;
    let space = 8 + Attempt::INIT_SPACE;
    let lamports = rent.minimum_balance(space);
    let partner_own_rent = rent.minimum_balance(8 + Partner::INIT_SPACE);

    let partner_info = ctx.accounts.partner.to_account_info();
    let attempt_info = ctx.accounts.attempt.to_account_info();
    let deficit = lamports.saturating_sub(attempt_info.lamports());

    let vault_available = partner_info
        .lamports()
        .checked_sub(partner_own_rent)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    require!(vault_available >= deficit, RegtechError::VaultInsufficient);

    let attempt_seeds: &[&[u8]] = &[
        ATTEMPT_SEED,
        user_key.as_ref(),
        &partner_id,
        &module_id_hash,
        &[attempt_bump],
    ];

    invoke_signed(
        &system_instruction::allocate(&attempt_info.key(), space as u64),
        std::slice::from_ref(&attempt_info),
        &[attempt_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(&attempt_info.key(), &crate::ID),
        std::slice::from_ref(&attempt_info),
        &[attempt_seeds],
    )?;

    if deficit > 0 {
        **partner_info.try_borrow_mut_lamports()? = partner_info
            .lamports()
            .checked_sub(deficit)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
        **attempt_info.try_borrow_mut_lamports()? = attempt_info
            .lamports()
            .checked_add(deficit)
            .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    }

    let state = Attempt {
        user: user_key,
        partner_id,
        module_id_hash,
        last_attempt_at: 0,
        last_score_bps: 0,
        attempt_count: 0,
        passed: false,
        passed_at: None,
        bump: attempt_bump,
    };
    let mut buf = Vec::with_capacity(space);
    state.try_serialize(&mut buf)?;
    let mut data = attempt_info.try_borrow_mut_data()?;
    data[..buf.len()].copy_from_slice(&buf);
    drop(data);

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
