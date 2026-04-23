use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::constants::{CONFIG_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Config, Partner};

// Tops up a partner's on-chain vault. The Partner PDA pulls double duty:
// its data holds partner metadata, and any lamports above its own rent-
// exempt minimum are the quiz budget the program draws from when creating
// Attempt PDAs. The partner themselves have no path to pull SOL out. Only
// start_attempt (rent for Attempt) and refund_partner (super-admin) ever
// move lamports out of here.
#[derive(Accounts)]
pub struct FundPartner<'info> {
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

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_fund_partner(ctx: Context<FundPartner>, amount: u64) -> Result<()> {
    require!(amount > 0, RegtechError::VaultInsufficient);

    // Admin is System-owned, so we route the debit through the system
    // program. The receiver is our Partner PDA.
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.admin.to_account_info(),
                to: ctx.accounts.partner.to_account_info(),
            },
        ),
        amount,
    )?;

    emit!(PartnerFunded {
        actor: ctx.accounts.admin.key(),
        partner_id: ctx.accounts.partner.partner_id,
        amount,
        new_balance: ctx.accounts.partner.to_account_info().lamports(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PartnerFunded {
    pub actor: Pubkey,
    pub partner_id: [u8; 16],
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}
