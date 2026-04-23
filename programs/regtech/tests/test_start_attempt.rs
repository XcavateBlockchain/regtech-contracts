mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

#[test]
fn happy_path_writes_fresh_attempt() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
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
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user_a = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
    let user_b = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user_a.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    advance_blockhash(&mut svm);
    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user_b.pubkey(), partner_id, module_id_hash),
        &[&attestor],
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
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );

    advance_blockhash(&mut svm);

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    // Second init on the same PDA bounces off the system program with
    // "account already in use". Not one of our error codes.
    assert!(res.is_err(), "second start_attempt for same (user, module) should fail init");
}

#[test]
fn start_attempt_without_enrollment_rejected() {
    let ModuleFixture {
        mut svm,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = Keypair::new();

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    expect_error_code(res, 3012);
}

#[test]
fn start_attempt_after_revoked_enrollment_rejected() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
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
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
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
        attestor,
        partner_id,
        ..
    } = register_partner_fixture();

    let user = Keypair::new();
    let fake_hash = code_hash("never-registered");

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, fake_hash),
        &[&attestor],
    );
    expect_error_code(res, 3012);
}

#[test]
fn non_attestor_signer_rejected() {
    // has_one = attestor on Partner means only the key registered as
    // attestor can sign start_attempt. If partner_admin tries to start
    // on behalf of a user, they get NotAuthorized.
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
        ix_start_attempt(partner_admin.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&partner_admin],
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

    // Enroll before pausing. Enrollment is an admin op and runs even
    // while paused, but here we want to isolate the pause check on
    // start_attempt.
    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    set_config_paused(&mut svm, true);

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
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

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    set_partner_active(&mut svm, &partner_id, false);

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
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

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    set_module_active(&mut svm, &partner_id, &module_id_hash, false);

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::ModuleInactive);
}

#[test]
fn rejects_when_vault_empty() {
    // Drain the Partner vault down to the floor with a full refund, then
    // try to start. No quiz budget, no start.
    let ModuleFixture {
        mut svm,
        admin,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    send_ok(
        &mut svm,
        ix_refund_partner(admin.pubkey(), partner_id, DEFAULT_VAULT_FUNDING),
        &[&admin],
    );

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::VaultInsufficient);
}

#[test]
fn attestor_is_net_zero_after_start() {
    // Sanity check on the lamport-swap. Anchor's init debits the
    // attestor for the Attempt rent, then our handler refunds the same
    // amount from the Partner vault. The attestor should only be down
    // the tx fee.
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    let attestor_before = svm.get_account(&attestor.pubkey()).unwrap().lamports;
    let vault_before = svm
        .get_account(&partner_pda(&partner_id))
        .unwrap()
        .lamports;

    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );

    let attestor_after = svm.get_account(&attestor.pubkey()).unwrap().lamports;
    let vault_after = svm
        .get_account(&partner_pda(&partner_id))
        .unwrap()
        .lamports;

    // Attestor should only be down the tx fee (~5000 lamports base).
    // Everything else got refunded from the vault.
    let attestor_delta = attestor_before - attestor_after;
    assert!(
        attestor_delta < 100_000,
        "attestor should only be down the tx fee, was down {attestor_delta}"
    );

    // Vault should be down by the rent cost of the Attempt PDA.
    assert!(
        vault_after < vault_before,
        "partner vault should be debited for Attempt rent"
    );
}
