mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

fn start_scenario() -> (litesvm::LiteSVM, Keypair, Keypair, [u8; 16], [u8; 32]) {
    let ModuleFixture {
        mut svm,
        attestor,
        partner_admin,
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

    (svm, user, attestor, partner_id, module_id_hash)
}

#[test]
fn happy_path_pass_latches_passed_and_sets_passed_at() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();

    send_ok(
        &mut svm,
        ix_submit_attempt(
            attestor.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            8_000,
        ),
        &[&attestor],
    );

    let a = read_attempt(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(a.attempt_count, 1);
    assert_eq!(a.last_score_bps, 8_000);
    assert!(a.last_attempt_at > 0);
    assert!(a.passed, "score >= threshold (7000) should latch passed");
    assert!(a.passed_at.is_some(), "passed_at set on the pass transition");
    assert_eq!(a.passed_at, Some(a.last_attempt_at));
}

#[test]
fn happy_path_fail_leaves_passed_false() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();

    send_ok(
        &mut svm,
        ix_submit_attempt(
            attestor.pubkey(),
            user.pubkey(),
            partner_id,
            module_id_hash,
            6_000,
        ),
        &[&attestor],
    );

    let a = read_attempt(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(a.attempt_count, 1);
    assert_eq!(a.last_score_bps, 6_000);
    assert!(!a.passed, "score < threshold should not latch passed");
    assert_eq!(a.passed_at, None);
}

#[test]
fn chain_computes_pass_flag_not_attestor_assertion() {
    let ModuleFixture {
        mut svm,
        attestor,
        partner_admin,
        partner_id,
        ..
    } = register_module_fixture();

    // Register a stricter module with a 9500 threshold.
    let strict_code = "strict".to_string();
    let strict_hash = code_hash(&strict_code);
    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            strict_hash,
            strict_code,
            "uri".to_string(),
            Some(9_500),
            None,
            None,
        ),
        &[&partner_admin],
    );

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, strict_hash);

    send_ok(
        &mut svm,
        ix_start_attempt(attestor.pubkey(), user.pubkey(), partner_id, strict_hash),
        &[&attestor],
    );

    // Attestor submits 8000. That's above the partner default of 7000 but
    // below the module's override of 9500, so the chain should say fail.
    send_ok(
        &mut svm,
        ix_submit_attempt(
            attestor.pubkey(),
            user.pubkey(),
            partner_id,
            strict_hash,
            8_000,
        ),
        &[&attestor],
    );

    let a = read_attempt(&svm, &user.pubkey(), &partner_id, &strict_hash);
    assert!(!a.passed, "8000 is below strict module threshold of 9500");
}

#[test]
fn cooldown_not_elapsed_rejects_second_submission() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();

    send_ok(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 6_000),
        &[&attestor],
    );

    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 6_500),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::CooldownNotElapsed);
}

#[test]
fn cooldown_elapsed_allows_resubmission() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();

    send_ok(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 6_000),
        &[&attestor],
    );

    // Module cooldown is 86400 seconds (24h). Warp the clock past it.
    warp_unix_seconds(&mut svm, 86_401);

    send_ok(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 6_500),
        &[&attestor],
    );

    let a = read_attempt(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert_eq!(a.attempt_count, 2);
    assert_eq!(a.last_score_bps, 6_500);
}

#[test]
fn already_passed_rejects_further_submissions() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();

    // Pass on the first attempt so the latch flips.
    send_ok(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 9_000),
        &[&attestor],
    );

    // Warp past cooldown so that isn't what trips here.
    warp_unix_seconds(&mut svm, 86_401);

    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 10_000),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::AlreadyPassed);
}

#[test]
fn non_attestor_signer_rejected() {
    let (mut svm, user, _real_attestor, partner_id, module_id_hash) = start_scenario();
    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_submit_attempt(imposter.pubkey(), user.pubkey(), partner_id, module_id_hash, 9_000),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn score_over_ten_thousand_rejected() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();

    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 10_001),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::InvalidScore);
}

#[test]
fn score_at_exactly_threshold_passes() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();

    // Module threshold is 7000, inherited from Partner, which inherited
    // it from Config.
    send_ok(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 7_000),
        &[&attestor],
    );

    let a = read_attempt(&svm, &user.pubkey(), &partner_id, &module_id_hash);
    assert!(a.passed, "score == threshold should be treated as a pass (>=)");
}

#[test]
fn rejects_when_paused() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();
    set_config_paused(&mut svm, true);

    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 8_000),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::Paused);
}

#[test]
fn rejects_when_partner_inactive() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();
    set_partner_active(&mut svm, &partner_id, false);

    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 8_000),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::PartnerInactive);
}

#[test]
fn rejects_when_module_inactive() {
    let (mut svm, user, attestor, partner_id, module_id_hash) = start_scenario();
    set_module_active(&mut svm, &partner_id, &module_id_hash, false);

    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 8_000),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::ModuleInactive);
}

#[test]
fn submit_attempt_with_uninitialized_pda_fails() {
    // User was enrolled but never called start_attempt, so the Attempt PDA
    // has no data. Anchor's Account<'info, Attempt> loader returns
    // AccountNotInitialized (code 3012). Not a RegtechError, so we pin the
    // raw code instead.
    let ModuleFixture {
        mut svm,
        attestor,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    // Enroll so the enrollment gate isn't what rejects. Still skip start.
    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 8_000),
        &[&attestor],
    );
    expect_error_code(res, 3012);
}
