use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::error::RegtechError;
use crate::state::Config;

#[derive(Accounts)]
pub struct ProposeAdminUpdate<'info> {
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

pub(crate) fn handle_propose_admin_update(
    ctx: Context<ProposeAdminUpdate>,
    candidate: Pubkey,
) -> Result<()> {
    require!(candidate != Pubkey::default(), RegtechError::InvalidPubkey);

    let config = &mut ctx.accounts.config;
    let previous = config.pending_admin;
    config.pending_admin = Some(candidate);

    emit!(AdminProposalUpdated {
        previous_candidate: previous,
        new_candidate: candidate,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct AdminProposalUpdated {
    pub previous_candidate: Option<Pubkey>,
    pub new_candidate: Pubkey,
    pub timestamp: i64,
}
