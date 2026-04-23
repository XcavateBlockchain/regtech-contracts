use anchor_lang::prelude::*;

use crate::constants::{ATTEMPT_SEED, CONFIG_SEED, ENROLLMENT_SEED, MODULE_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Attempt, Config, Enrollment, Module, Partner};

// Attestor signs (same key that later scores the submission), Partner PDA
// picks up the rent bill. The user is the subject, not a signer, which
// matches how B2B works in practice: the partner's backend drives the
// flow and the end user never touches a wallet.
//
// Rent flow is a two-step dance. Anchor's `init` pays the Attempt rent
// out of the attestor's account, then the handler swaps lamports back:
// vault down by rent, attestor up by rent, net zero for the attestor.
// The attestor still needs a working SOL balance to cover that window,
// but it stays roughly flat across calls.
#[derive(Accounts)]
pub struct StartAttempt<'info> {
    #[account(mut)]
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

    // Enrollment gate. The loader trips with AccountNotInitialized if the
    // user was never enrolled or got revoked. Seeds use partner.partner_id
    // so an attempt to start through the wrong partner doesn't resolve.
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
        payer = attestor,
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

    // Anchor's init just created the Attempt PDA out of the attestor's
    // pocket. Refund the attestor from the vault so the partner ends up
    // bearing the cost.
    let rent = Rent::get()?;
    let attempt_rent = rent.minimum_balance(8 + Attempt::INIT_SPACE);
    let partner_own_rent = rent.minimum_balance(8 + Partner::INIT_SPACE);

    let partner_info = ctx.accounts.partner.to_account_info();
    let attestor_info = ctx.accounts.attestor.to_account_info();

    let partner_balance = partner_info.lamports();
    let vault_available = partner_balance
        .checked_sub(partner_own_rent)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    require!(vault_available >= attempt_rent, RegtechError::VaultInsufficient);

    **partner_info.try_borrow_mut_lamports()? = partner_balance
        .checked_sub(attempt_rent)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    **attestor_info.try_borrow_mut_lamports()? = attestor_info
        .lamports()
        .checked_add(attempt_rent)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;

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
