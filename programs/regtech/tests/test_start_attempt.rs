mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

#[test]
fn happy_path_writes_fresh_attempt() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );

    let a = read_attempt(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(a.user, user.pubkey());
    assert_eq!(a.partner_id, partner_id);
    assert_eq!(a.module_id_hash, module_id_hash);
    assert_eq!(a.last_attempt_at, 0, "no submissions yet");
    assert_eq!(a.last_score_bps, 0);
    assert_eq!(a.attempt_count, 0);
    assert!(!a.passed);
    assert_eq!(a.passed_at, None);
}

#[test]
fn two_users_can_start_on_same_module_independently() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user_a = Keypair::new();
    let user_b = Keypair::new();
    fund(&mut svm, &user_a.pubkey(), 1_000_000_000);
    fund(&mut svm, &user_b.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_start_attempt(user_a.pubkey(), partner_id, module_id_hash),
        &[&user_a],
    );
    send_ok(
        &mut svm,
        ix_start_attempt(user_b.pubkey(), partner_id, module_id_hash),
        &[&user_b],
    );

    let attempt_a = read_attempt(&svm, &user_a.pubkey(), &partner_id, &module_id_hash);
    let attempt_b = read_attempt(&svm, &user_b.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(attempt_a.user, user_a.pubkey());
    assert_eq!(attempt_b.user, user_b.pubkey());
}

#[test]
fn duplicate_start_by_same_user_rejected() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    // Duplicate init hits "account already in use" from the system program,
    // not one of our error codes.
    assert!(res.is_err(), "second start_attempt for same (user, module) should fail init");
}

#[test]
fn start_attempt_with_unknown_module_rejected() {
    let PartnerFixture {
        mut svm,
        partner_id,
        ..
    } = register_partner_fixture();

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    // We never registered this module, so nothing lives at its PDA.
    let fake_hash = code_hash("never-registered");

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, fake_hash),
        &[&user],
    );
    expect_error_code(res, 3012); // Anchor AccountNotInitialized
}

#[test]
fn rejects_when_paused() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();
    set_config_paused(&mut svm, true);

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    expect_regtech_error(res, RegtechError::Paused);
}

#[test]
fn rejects_when_partner_inactive() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();
    set_partner_active(&mut svm, &partner_id, false);

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    expect_regtech_error(res, RegtechError::PartnerInactive);
}

#[test]
fn rejects_when_module_inactive() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();
    set_module_active(&mut svm, &partner_id, &module_id_hash, false);

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    expect_regtech_error(res, RegtechError::ModuleInactive);
}

#[test]
fn user_with_insufficient_lamports_cannot_start() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();
    // No airdrop for this user, so they can't pay rent on the Attempt PDA.

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    // System program rejects create_account when the user has no lamports.
    // Not a RegtechError, just a plain InstructionError.
    assert!(res.is_err(), "user with 0 SOL cannot fund the Attempt PDA");
}
