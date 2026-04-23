use anchor_lang::prelude::*;

use crate::constants::{CONFIG_SEED, PARTNER_SEED};
use crate::error::RegtechError;
use crate::state::{Config, Partner};

// Pulls lamports back out of a partner's vault to the super-admin. The
// usual trigger is a partner who overfunded for quizzes they never ran and
// wants their money back. Super-admin drains whatever's left on-chain and
// settles the fiat side off-chain.
#[derive(Accounts)]
pub struct RefundPartner<'info> {
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

pub(crate) fn handle_refund_partner(ctx: Context<RefundPartner>, amount: u64) -> Result<()> {
    require!(amount > 0, RegtechError::VaultInsufficient);

    let partner_info = ctx.accounts.partner.to_account_info();
    let admin_info = ctx.accounts.admin.to_account_info();

    // The vault balance is whatever sits on top of the Partner PDA's own
    // rent-exempt minimum. Anything below that floor belongs to the
    // account itself, not to the vault.
    let partner_own_rent = Rent::get()?.minimum_balance(8 + Partner::INIT_SPACE);
    let current = partner_info.lamports();
    let available = current
        .checked_sub(partner_own_rent)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    require!(available >= amount, RegtechError::VaultInsufficient);

    // Partner is program-owned so the runtime lets us debit its lamports
    // directly.
    **partner_info.try_borrow_mut_lamports()? = current
        .checked_sub(amount)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;
    **admin_info.try_borrow_mut_lamports()? = admin_info
        .lamports()
        .checked_add(amount)
        .ok_or(error!(RegtechError::ArithmeticOverflow))?;

    emit!(PartnerRefunded {
        actor: ctx.accounts.admin.key(),
        partner_id: ctx.accounts.partner.partner_id,
        amount,
        new_balance: partner_info.lamports(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PartnerRefunded {
    pub actor: Pubkey,
    pub partner_id: [u8; 16],
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}
