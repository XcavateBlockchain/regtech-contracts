mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

// Comfortably above the fixture's 7_000 bps threshold so the attempt
// passes without having to think about boundary arithmetic.
const PASSING_SCORE: u16 = 8_500;

#[test]
fn happy_path_writes_credential_from_passed_attempt() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    send_ok(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );

    let c = read_credential(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(c.user, user.pubkey());
    assert_eq!(c.partner_id, partner_id);
    assert_eq!(c.module_id_hash, module_id_hash);
    assert_eq!(c.score_bps, PASSING_SCORE);
    assert_eq!(c.issued_by, partner_admin.pubkey());
    assert!(c.issued_at > 0);
    // The module fixture sets expires_in_seconds to a year, so the
    // credential should come out with a deadline set.
    assert!(c.expires_at.is_some(), "expiry should be snapshotted from module");
    assert!(c.revoked_at.is_none(), "fresh credential is not revoked");
    assert!(c.credential_asset.is_none(), "asset link is set later, not at claim");
}

#[test]
fn claim_rejected_when_attempt_not_passed() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    // Enroll and start, but submit a failing score (below the 7_000 threshold).
    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    send_ok(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 5_000),
        &[&attestor],
    );

    let res = send(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::AttemptNotPassed);
}

#[test]
fn claim_rejected_without_attempt() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    let res = send(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );
    expect_error_code(res, 3012);
}

#[test]
fn claim_rejected_after_enrollment_revoked() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    send_ok(
        &mut svm,
        ix_revoke_enrollment(
            partner_admin.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            0,
        ),
        &[&partner_admin],
    );

    let res = send(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );
    expect_error_code(res, 3012);
}

#[test]
fn duplicate_claim_rejected() {
    // One credential per (user, partner, module). A second claim on the
    // same triple hits "account already in use" from the system program.
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    send_ok(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );

    advance_blockhash(&mut svm);

    let res = send(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );
    assert!(res.is_err(), "second claim for same triple should fail init");
}

#[test]
fn non_partner_admin_caller_rejected() {
    // Super-admin doesn't get to issue credentials. That's the
    // partner_admin's call for their own partner.
    let ModuleFixture {
        mut svm,
        admin,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    let res = send(
        &mut svm,
        ix_claim_credential(admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn imposter_rejected() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_claim_credential(imposter.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn rejects_when_paused() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    set_config_paused(&mut svm, true);

    let res = send(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::Paused);
}

#[test]
fn rejects_when_partner_inactive() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    set_partner_active(&mut svm, &partner_id, false);

    let res = send(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::PartnerInactive);
}

#[test]
fn rejects_when_module_inactive() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    set_module_active(&mut svm, &partner_id, &module_id_hash, false);

    let res = send(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::ModuleInactive);
}

#[test]
fn module_without_expiry_produces_credential_without_expiry() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        ..
    } = register_partner_fixture();

    let code = "eternal-module".to_string();
    let module_id_hash = code_hash(&code);
    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            code,
            "ipfs://eternal".to_string(),
            None,
            None,
            None, // no expiry
        ),
        &[&partner_admin],
    );

    let user = passed_user(
        &mut svm,
        &partner_admin,
        &attestor,
        partner_id,
        module_id_hash,
        PASSING_SCORE,
    );

    send_ok(
        &mut svm,
        ix_claim_credential(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
    );

    let c = read_credential(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert!(c.expires_at.is_none(), "no module expiry, no credential expiry");
}

#[test]
fn partner_b_cannot_claim_for_partner_a_user() {
    let PlatformFixture { mut svm, admin } = init_platform();

    // Partner A with a module, user enrolled and passed.
    let partner_a_id = [0xAAu8; 16];
    let partner_admin_a = Keypair::new();
    let attestor_a = Keypair::new();
    let collection_a = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin_a.pubkey(), 2_000_000_000);
    fund(&mut svm, &attestor_a.pubkey(), 2_000_000_000);
    install_collection(&mut svm, collection_a, partner_pda(&partner_a_id));
    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_a_id,
            collection_a,
            "Partner A".to_string(),
            attestor_a.pubkey(),
            partner_admin_a.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    send_ok(
        &mut svm,
        ix_fund_partner(admin.pubkey(), partner_a_id, DEFAULT_VAULT_FUNDING),
        &[&admin],
    );

    let code_a = "module-a".to_string();
    let module_a_hash = code_hash(&code_a);
    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin_a.pubkey(),
            partner_a_id,
            module_a_hash,
            code_a,
            "uri-a".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin_a],
    );

    let user = passed_user(
        &mut svm,
        &partner_admin_a,
        &attestor_a,
        partner_a_id,
        module_a_hash,
        PASSING_SCORE,
    );

    // Partner B setup.
    let partner_b_id = [0xBBu8; 16];
    let partner_admin_b = Keypair::new();
    let attestor_b = Keypair::new();
    let collection_b = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin_b.pubkey(), 2_000_000_000);
    install_collection(&mut svm, collection_b, partner_pda(&partner_b_id));
    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_b_id,
            collection_b,
            "Partner B".to_string(),
            attestor_b.pubkey(),
            partner_admin_b.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );

    // Partner B calls claim with B's Partner account in the partner
    // slot, targeting partner A's user+module. Seeds derived through B
    // don't line up with A's on-chain Enrollment/Attempt, tx fails.
    let res = send(
        &mut svm,
        ix_claim_credential(partner_admin_b.pubkey(), user.pubkey(), partner_b_id, module_a_hash),
        &[&partner_admin_b],
    );
    assert!(res.is_err(), "partner B should not be able to claim on A's data");
}
