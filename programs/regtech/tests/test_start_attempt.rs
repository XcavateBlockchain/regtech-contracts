mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

#[test]
fn happy_path_writes_fresh_attempt() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

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
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user_a = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
    let user_b = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

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
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

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
    // Second init on the same PDA bounces off the system program with
    // "account already in use". Not one of our error codes.
    assert!(res.is_err(), "second start_attempt for same (user, module) should fail init");
}

#[test]
fn start_attempt_without_enrollment_rejected() {
    let ModuleFixture {
        mut svm,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    expect_error_code(res, 3012);
}

#[test]
fn start_attempt_after_revoked_enrollment_rejected() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

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
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    expect_error_code(res, 3012);
}

#[test]
fn start_attempt_with_unknown_module_rejected() {
    // No module registered at this hash. Anchor walks accounts in the
    // order they're declared, and `module` sits before `enrollment`, so
    // the module loader is the one that trips with AccountNotInitialized.
    let PartnerFixture {
        mut svm,
        partner_id,
        ..
    } = register_partner_fixture();

    let user = Keypair::new();
    fund(&mut svm, &user.pubkey(), 1_000_000_000);

    let fake_hash = code_hash("never-registered");

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, fake_hash),
        &[&user],
    );
    expect_error_code(res, 3012);
}

#[test]
fn rejects_when_paused() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    // Enroll before pausing. Enrollment is an admin op and runs even
    // while paused, but here we want to isolate the pause check on
    // start_attempt.
    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    set_config_paused(&mut svm, true);

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
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    set_partner_active(&mut svm, &partner_id, false);

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
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    set_module_active(&mut svm, &partner_id, &module_id_hash, false);

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
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    // Enroll the user but skip funding them. Partner_admin paid for
    // the Enrollment rent, but the Attempt PDA rent comes out of the
    // user's own account.
    let user = Keypair::new();
    send_ok(
        &mut svm,
        ix_enroll_user(
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
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    // With zero lamports the system program won't create_account, so
    // this fails with a plain InstructionError rather than one of ours.
    assert!(res.is_err(), "user with 0 SOL cannot fund the Attempt PDA");
}
