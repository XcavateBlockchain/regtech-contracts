use anchor_lang::prelude::*;

use crate::constants::{
    ATTEMPT_SEED, CONFIG_SEED, CREDENTIAL_SEED, ENROLLMENT_SEED, MODULE_SEED, PARTNER_SEED,
};
use crate::error::RegtechError;
use crate::state::{Attempt, Config, Credential, Enrollment, Module, Partner};

// Issuance op, so a global pause stops it. Same treatment as enroll_user.
// Only the deactivation paths (revoke_credential when we get to it) should
// keep working through a pause.
#[derive(Accounts)]
pub struct ClaimCredential<'info> {
    #[account(mut)]
    pub partner_admin: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ RegtechError::Paused,
    )]
    pub config: Account<'info, Config>,

    #[account(
        seeds = [PARTNER_SEED, &partner.partner_id],
        bump = partner.bump,
        has_one = partner_admin @ RegtechError::NotAuthorized,
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

    // Enrollment has to still be live when the credential is claimed. If
    // partner_admin revoked, the PDA is closed and the loader bounces with
    // AccountNotInitialized. Passing the quiz on its own isn't enough, the
    // grant of access has to still be in force.
    #[account(
        seeds = [
            ENROLLMENT_SEED,
            enrollment.user.as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump = enrollment.bump,
    )]
    pub enrollment: Account<'info, Enrollment>,

    // Attempt PDA for the same user+partner+module. The explicit
    // attempt.user == enrollment.user check stops a partner from claiming
    // for user A while pointing at user B's passing attempt.
    #[account(
        seeds = [
            ATTEMPT_SEED,
            attempt.user.as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump = attempt.bump,
        constraint = attempt.user == enrollment.user @ RegtechError::NotAuthorized,
        constraint = attempt.passed @ RegtechError::AttemptNotPassed,
    )]
    pub attempt: Account<'info, Attempt>,

    #[account(
        init,
        payer = partner_admin,
        space = 8 + Credential::INIT_SPACE,
        seeds = [
            CREDENTIAL_SEED,
            enrollment.user.as_ref(),
            &partner.partner_id,
            &module.module_id_hash,
        ],
        bump,
    )]
    pub credential: Account<'info, Credential>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handle_claim_credential(ctx: Context<ClaimCredential>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let partner_id = ctx.accounts.partner.partner_id;
    let module = &ctx.accounts.module;
    let enrollment = &ctx.accounts.enrollment;
    let attempt = &ctx.accounts.attempt;
    let issued_by = ctx.accounts.partner_admin.key();

    // Snapshot the expiry from the module as it stands right now. If the
    // partner changes the module's expiry policy later, already-issued
    // credentials keep the deadline they were stamped with.
    let expires_at = match module.expires_in_seconds {
        Some(secs) => Some(
            now.checked_add(secs)
                .ok_or(error!(RegtechError::ArithmeticOverflow))?,
        ),
        None => None,
    };

    let credential = &mut ctx.accounts.credential;
    credential.user = enrollment.user;
    credential.partner_id = partner_id;
    credential.module_id_hash = module.module_id_hash;
    credential.score_bps = attempt.last_score_bps;
    credential.issued_at = now;
    credential.issued_by = issued_by;
    credential.expires_at = expires_at;
    credential.revoked_at = None;
    credential.credential_asset = None;
    credential.bump = ctx.bumps.credential;

    emit!(CredentialIssued {
        actor: issued_by,
        user: credential.user,
        partner_id,
        module_id_hash: credential.module_id_hash,
        score_bps: credential.score_bps,
        issued_at: credential.issued_at,
        expires_at: credential.expires_at,
    });

    Ok(())
}

#[event]
pub struct CredentialIssued {
    pub actor: Pubkey,
    pub user: Pubkey,
    pub partner_id: [u8; 16],
    pub module_id_hash: [u8; 32],
    pub score_bps: u16,
    pub issued_at: i64,
    pub expires_at: Option<i64>,
}
