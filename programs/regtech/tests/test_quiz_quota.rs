mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

#[test]
fn allocate_bumps_purchased_counter() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    // Fixture already allocated DEFAULT_QUIZ_ALLOCATION.
    let before = read_partner(&svm, &partner_id);
    assert_eq!(before.quizzes_purchased, DEFAULT_QUIZ_ALLOCATION);
    assert_eq!(before.quizzes_consumed, 0);
    assert_eq!(before.quizzes_refunded, 0);

    send_ok(
        &mut svm,
        ix_allocate_quizzes(admin.pubkey(), partner_id, 250),
        &[&admin],
    );

    let after = read_partner(&svm, &partner_id);
    assert_eq!(after.quizzes_purchased, DEFAULT_QUIZ_ALLOCATION + 250);
    assert_eq!(after.quizzes_consumed, 0, "consumed untouched by allocation");
    assert_eq!(after.quizzes_refunded, 0, "refunded untouched by allocation");
}

#[test]
fn start_attempt_consumes_one_quiz() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    let before = read_partner(&svm, &partner_id).quizzes_consumed;
    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    let after = read_partner(&svm, &partner_id).quizzes_consumed;

    assert_eq!(after, before + 1, "consumed should bump by exactly 1");
}

#[test]
fn start_attempt_rejected_when_quota_exhausted() {
    // Refund the entire allocation so remaining = 0, then try to start.
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
        ix_refund_quizzes(admin.pubkey(), partner_id, DEFAULT_QUIZ_ALLOCATION, 0),
        &[&admin],
    );

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::QuizQuotaExhausted);
}

#[test]
fn quota_check_fires_before_vault_check() {
    // If both fail, the partner should see QuizQuotaExhausted, not
    // VaultInsufficient. Quota is an actionable error ("buy more
    // quizzes"), vault is an Xcavate ops issue.
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

    // Drain both the vault and the quota. Enrollment already took some
    // rent from the vault, so read the actual available balance.
    let refundable = vault_available(&svm, &partner_id);
    send_ok(
        &mut svm,
        ix_refund_partner(admin.pubkey(), partner_id, refundable),
        &[&admin],
    );
    send_ok(
        &mut svm,
        ix_refund_quizzes(admin.pubkey(), partner_id, DEFAULT_QUIZ_ALLOCATION, 0),
        &[&admin],
    );

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::QuizQuotaExhausted);
}

#[test]
fn refund_bumps_refunded_counter() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    send_ok(
        &mut svm,
        ix_refund_quizzes(admin.pubkey(), partner_id, 100, 7),
        &[&admin],
    );

    let p = read_partner(&svm, &partner_id);
    assert_eq!(p.quizzes_purchased, DEFAULT_QUIZ_ALLOCATION, "purchased untouched");
    assert_eq!(p.quizzes_consumed, 0, "consumed untouched");
    assert_eq!(p.quizzes_refunded, 100);
}

#[test]
fn refund_rejected_when_exceeds_available() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_refund_quizzes(admin.pubkey(), partner_id, DEFAULT_QUIZ_ALLOCATION + 1, 0),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidAmount);
}

#[test]
fn refund_accounts_for_already_consumed() {
    // Partner purchased 1000, used some, tries to refund. The refund cap
    // is purchased - consumed - refunded, so refund + consumed can't
    // exceed purchased.
    let ModuleFixture {
        mut svm,
        admin,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    // Run three starts, bumping consumed to 3.
    for _ in 0..3 {
        let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
        send_ok(
            &mut svm,
            ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
            &[&attestor],
        );
        advance_blockhash(&mut svm);
    }

    // Now try to refund (DEFAULT_QUIZ_ALLOCATION - 2). That would push
    // consumed + refunded above purchased by 1. Should fail.
    let res = send(
        &mut svm,
        ix_refund_quizzes(admin.pubkey(), partner_id, DEFAULT_QUIZ_ALLOCATION - 2, 0),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidAmount);

    // Refunding exactly purchased - consumed works.
    send_ok(
        &mut svm,
        ix_refund_quizzes(admin.pubkey(), partner_id, DEFAULT_QUIZ_ALLOCATION - 3, 0),
        &[&admin],
    );
    let p = read_partner(&svm, &partner_id);
    assert_eq!(p.quizzes_consumed + p.quizzes_refunded, p.quizzes_purchased);
}

#[test]
fn allocate_rejected_for_non_admin() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_allocate_quizzes(partner_admin.pubkey(), partner_id, 100),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn refund_rejected_for_non_admin() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_refund_quizzes(partner_admin.pubkey(), partner_id, 100, 0),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn allocate_with_zero_count_rejected() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_allocate_quizzes(admin.pubkey(), partner_id, 0),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidAmount);
}

#[test]
fn refund_with_zero_count_rejected() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let res = send(
        &mut svm,
        ix_refund_quizzes(admin.pubkey(), partner_id, 0, 0),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidAmount);
}

#[test]
fn fresh_partner_has_zero_quota_and_cant_start() {
    // Register a partner via the low-level ix (not the fixture, which
    // pre-allocates). Then enroll a user and try to start. Should fail
    // with QuizQuotaExhausted because no one allocated any quizzes yet.
    let PlatformFixture { mut svm, admin } = init_platform();

    let partner_id = [42u8; 16];
    let partner_admin = Keypair::new();
    let attestor = Keypair::new();
    let collection = Keypair::new().pubkey();
    fund(&mut svm, &partner_admin.pubkey(), 2_000_000_000);
    fund(&mut svm, &attestor.pubkey(), 2_000_000_000);
    install_collection(&mut svm, collection, partner_pda(&partner_id));

    send_ok(
        &mut svm,
        ix_register_partner(
            admin.pubkey(),
            partner_id,
            collection,
            "Zero Quota".to_string(),
            attestor.pubkey(),
            partner_admin.pubkey(),
            None,
            None,
        ),
        &[&admin],
    );
    // Fund the vault so vault isn't what blocks us.
    send_ok(
        &mut svm,
        ix_fund_partner(admin.pubkey(), partner_id, DEFAULT_VAULT_FUNDING),
        &[&admin],
    );

    let code = "mod".to_string();
    let module_id_hash = code_hash(&code);
    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            code,
            "uri".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    let res = send(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::QuizQuotaExhausted);
}

#[test]
fn consumed_counter_is_monotonic_across_multiple_starts() {
    let ModuleFixture {
        mut svm,
        partner_admin,
        attestor,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    for expected in 1..=5 {
        let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
        send_ok(
            &mut svm,
            ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash),
            &[&attestor],
        );
        advance_blockhash(&mut svm);
        assert_eq!(
            read_partner(&svm, &partner_id).quizzes_consumed,
            expected,
            "consumed should monotonically increase"
        );
    }
}
