use anchor_lang::prelude::*;

use crate::constants::{CONFIG_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Config, Partner};

// Symmetric to allocate_quizzes. Super-admin signs off on a refund against
// the contractual quota. The quizzes_refunded counter is monotonic, so the
// chain keeps an honest record: you can always see what was purchased,
// what was used, and what was refunded without ever rewriting history.
//
// Rejects if (consumed + refunded + count) would exceed purchased, since
// you can't refund more than the partner actually has outstanding.
#[derive(Accounts)]
pub struct RefundQuizzes<'info> {
    #[account(mut)]
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

pub(crate) fn handle_refund_quizzes(
    ctx: Context<RefundQuizzes>,
    count: u64,
    reason_code: u8,
) -> Result<()> {
    require!(count > 0, RegtechError::InvalidAmount);

    let partner = &mut ctx.accounts.partner;

    let used = partner
        .quizzes_consumed
        .checked_add(partner.quizzes_refunded)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    let new_used = used
        .checked_add(count)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    // Cap violation rather than "quota exhausted": the partner still has
    // outstanding quizzes, the caller just asked to refund more than is
    // actually available.
    require!(
        new_used <= partner.quizzes_purchased,
        RegtechError::InvalidAmount
    );

    partner.quizzes_refunded = partner
        .quizzes_refunded
        .checked_add(count)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;

    emit!(QuizzesRefunded {
        actor: ctx.accounts.admin.key(),
        partner_id: partner.partner_id,
        count,
        reason_code,
        new_refunded: partner.quizzes_refunded,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct QuizzesRefunded {
    pub actor: Pubkey,
    pub partner_id: [u8; 16],
    pub count: u64,
    pub reason_code: u8,
    pub new_refunded: u64,
    pub timestamp: i64,
}
