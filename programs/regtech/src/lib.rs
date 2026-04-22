#![allow(clippy::diverging_sub_expression)]

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::accept_admin_update::*;
pub use instructions::enroll_user::*;
pub use instructions::initialize_config::*;
pub use instructions::propose_admin_update::*;
pub use instructions::register_module::*;
pub use instructions::register_partner::*;
pub use instructions::revoke_enrollment::*;
pub use instructions::rotate_attestor::*;
pub use instructions::set_module_active::*;
pub use instructions::set_partner_active::*;
pub use instructions::set_paused::*;
pub use instructions::start_attempt::*;
pub use instructions::submit_attempt::*;

declare_id!("24tGu1PF1WwazTDeW4VWaJnqnGK2S422ntkmX7vfA2aq");

#[program]
pub mod regtech {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        default_pass_threshold_bps: u16,
        default_cooldown_seconds: i64,
    ) -> Result<()> {
        instructions::initialize_config::handle_initialize_config(
            ctx,
            default_pass_threshold_bps,
            default_cooldown_seconds,
        )
    }

    pub fn register_partner(
        ctx: Context<RegisterPartner>,
        partner_id: [u8; 16],
        name: String,
        attestor: Pubkey,
        partner_admin: Pubkey,
        pass_threshold_bps_override: Option<u16>,
        cooldown_seconds_override: Option<i64>,
    ) -> Result<()> {
        instructions::register_partner::handle_register_partner(
            ctx,
            partner_id,
            name,
            attestor,
            partner_admin,
            pass_threshold_bps_override,
            cooldown_seconds_override,
        )
    }

    pub fn register_module(
        ctx: Context<RegisterModule>,
        module_id_hash: [u8; 32],
        module_code: String,
        metadata_uri: String,
        pass_threshold_bps_override: Option<u16>,
        cooldown_seconds_override: Option<i64>,
        expires_in_seconds: Option<i64>,
    ) -> Result<()> {
        instructions::register_module::handle_register_module(
            ctx,
            module_id_hash,
            module_code,
            metadata_uri,
            pass_threshold_bps_override,
            cooldown_seconds_override,
            expires_in_seconds,
        )
    }

    pub fn start_attempt(ctx: Context<StartAttempt>) -> Result<()> {
        instructions::start_attempt::handle_start_attempt(ctx)
    }

    pub fn submit_attempt(ctx: Context<SubmitAttempt>, score_bps: u16) -> Result<()> {
        instructions::submit_attempt::handle_submit_attempt(ctx, score_bps)
    }

    pub fn set_paused(ctx: Context<SetPaused>, paused: bool, reason_code: u8) -> Result<()> {
        instructions::set_paused::handle_set_paused(ctx, paused, reason_code)
    }

    pub fn set_partner_active(
        ctx: Context<SetPartnerActive>,
        active: bool,
        reason_code: u8,
    ) -> Result<()> {
        instructions::set_partner_active::handle_set_partner_active(ctx, active, reason_code)
    }

    pub fn set_module_active(
        ctx: Context<SetModuleActive>,
        active: bool,
        reason_code: u8,
    ) -> Result<()> {
        instructions::set_module_active::handle_set_module_active(ctx, active, reason_code)
    }

    pub fn propose_admin_update(
        ctx: Context<ProposeAdminUpdate>,
        candidate: Pubkey,
    ) -> Result<()> {
        instructions::propose_admin_update::handle_propose_admin_update(ctx, candidate)
    }

    pub fn accept_admin_update(ctx: Context<AcceptAdminUpdate>) -> Result<()> {
        instructions::accept_admin_update::handle_accept_admin_update(ctx)
    }

    pub fn rotate_attestor(ctx: Context<RotateAttestor>, new_attestor: Pubkey) -> Result<()> {
        instructions::rotate_attestor::handle_rotate_attestor(ctx, new_attestor)
    }

    pub fn enroll_user(ctx: Context<EnrollUser>, reason_code: u8) -> Result<()> {
        instructions::enroll_user::handle_enroll_user(ctx, reason_code)
    }

    pub fn revoke_enrollment(ctx: Context<RevokeEnrollment>, reason_code: u8) -> Result<()> {
        instructions::revoke_enrollment::handle_revoke_enrollment(ctx, reason_code)
    }
}
