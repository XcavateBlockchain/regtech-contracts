use anchor_lang::prelude::*;

use crate::constants::{CONFIG_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Config, Partner};

// Bumps a partner's contractual quiz quota. Super-admin signs, the counter
// only goes up. Kept separate from fund_partner so the legal/accounting
// side (what was contracted for) is independent of the economic side
// (what SOL is on-chain to cover rent). In the common case ops bundles
// allocate_quizzes and fund_partner into one transaction for atomicity.
#[derive(Accounts)]
pub struct AllocateQuizzes<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ RegtechError::NotAuthorized,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
    )]
    pub partner: Account<'info, Partner>,
}

pub(crate) fn handle_allocate_quizzes(
    ctx: Context<AllocateQuizzes>,
    count: u64,
) -> Result<()> {
    require!(count > 0, RegtechError::InvalidAmount);

    let partner = &mut ctx.accounts.partner;
    partner.quizzes_purchased = partner
        .quizzes_purchased
        .checked_add(count)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;

    emit!(QuizzesAllocated {
        actor: ctx.accounts.admin.key(),
        partner_id: partner.partner_id,
        count,
        new_purchased: partner.quizzes_purchased,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct QuizzesAllocated {
    pub actor: Pubkey,
    pub partner_id: [u8; 16],
    pub count: u64,
    pub new_purchased: u64,
    pub timestamp: i64,
}
