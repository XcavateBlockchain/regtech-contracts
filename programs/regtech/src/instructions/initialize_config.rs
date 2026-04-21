use anchor_lang::prelude::*;

use crate::constants::{BPS_DENOMINATOR, CONFIG_SEED};
use crate::error::RegtechError;
use crate::state::Config;

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump,
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_initialize_config(
    ctx: Context<InitializeConfig>,
    default_pass_threshold_bps: u16,
    default_cooldown_seconds: i64,
) -> Result<()> {
    require!(
        default_pass_threshold_bps <= BPS_DENOMINATOR,
        RegtechError::InvalidThreshold
    );
    require!(
        default_cooldown_seconds >= 0,
        RegtechError::InvalidCooldown
    );

    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.paused = false;
    config.default_pass_threshold_bps = default_pass_threshold_bps;
    config.default_cooldown_seconds = default_cooldown_seconds;
    config.bump = ctx.bumps.config;

    emit!(ConfigInitialized {
        admin: config.admin,
        default_pass_threshold_bps,
        default_cooldown_seconds,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct ConfigInitialized {
    pub admin: Pubkey,
    pub default_pass_threshold_bps: u16,
    pub default_cooldown_seconds: i64,
    pub timestamp: i64,
}
