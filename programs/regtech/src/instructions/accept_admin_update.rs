use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::error::RegtechError;
use crate::state::Config;

#[derive(Accounts)]
pub struct AcceptAdminUpdate<'info> {
    // The candidate signs here to prove they actually control the pubkey that
    // was proposed.
    #[account(mut)]
    pub new_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
}

pub(crate) fn handle_accept_admin_update(ctx: Context<AcceptAdminUpdate>) -> Result<()> {
    let new_admin_key = ctx.accounts.new_admin.key();
    let config = &mut ctx.accounts.config;

    let pending = config
        .pending_admin
        .ok_or(error!(RegtechError::NoPendingAdmin))?;
    require!(pending == new_admin_key, RegtechError::PendingAdminMismatch);

    let previous_admin = config.admin;
    config.admin = new_admin_key;
    config.pending_admin = None;

    emit!(AdminUpdated {
        previous_admin,
        new_admin: new_admin_key,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct AdminUpdated {
    pub previous_admin: Pubkey,
    pub new_admin: Pubkey,
    pub timestamp: i64,
}
