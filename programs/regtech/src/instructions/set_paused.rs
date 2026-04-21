use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::error::RegtechError;
use crate::state::Config;

#[derive(Accounts)]
pub struct SetPaused<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ RegtechError::NotAuthorized,
    )]
    pub config: Account<'info, Config>,
}

pub(crate) fn handle_set_paused(
    ctx: Context<SetPaused>,
    paused: bool,
    reason_code: u8,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let was_paused = config.paused;
    config.paused = paused;

    emit!(ConfigPauseChanged {
        admin: config.admin,
        was_paused,
        now_paused: paused,
        reason_code,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct ConfigPauseChanged {
    pub admin: Pubkey,
    pub was_paused: bool,
    pub now_paused: bool,
    pub reason_code: u8,
    pub timestamp: i64,
}
