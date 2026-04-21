use anchor_lang::prelude::*;

use crate::constants::PARTNER_SEED;
use crate::error::RegtechError;
use crate::state::Partner;

#[derive(Accounts)]
pub struct RotateAttestor<'info> {
    #[account(mut)]
    pub partner_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
    )]
    pub partner: Account<'info, Partner>,
}

pub(crate) fn handle_rotate_attestor(
    ctx: Context<RotateAttestor>,
    new_attestor: Pubkey,
) -> Result<()> {
    // Rotating to the zero pubkey would mean no one can submit attempt scores
    // for this partner ever again.
    require!(
        new_attestor != Pubkey::default(),
        RegtechError::InvalidPubkey
    );

    let partner = &mut ctx.accounts.partner;
    let previous_attestor = partner.attestor;
    partner.attestor = new_attestor;

    emit!(AttestorRotated {
        partner_id: partner.partner_id,
        previous_attestor,
        new_attestor,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct AttestorRotated {
    pub partner_id: [u8; 16],
    pub previous_attestor: Pubkey,
    pub new_attestor: Pubkey,
    pub timestamp: i64,
}
