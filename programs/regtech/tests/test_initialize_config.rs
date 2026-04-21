mod common;

use {common::*, regtech::error::RegtechError};

#[test]
fn happy_path_writes_expected_config() {
    let (mut svm, admin) = setup();
    send_ok(
        &mut svm,
        ix_initialize_config(admin.pubkey(), 7_000, 86_400),
        &[&admin],
    );

    let config = read_config(&svm);
    assert_eq!(config.admin, admin.pubkey());
    assert!(!config.paused);
    assert_eq!(config.default_pass_threshold_bps, 7_000);
    assert_eq!(config.default_cooldown_seconds, 86_400);
}

#[test]
fn accepts_threshold_exactly_at_ten_thousand() {
    let (mut svm, admin) = setup();
    send_ok(
        &mut svm,
        ix_initialize_config(admin.pubkey(), 10_000, 0),
        &[&admin],
    );
    let config = read_config(&svm);
    assert_eq!(config.default_pass_threshold_bps, 10_000);
    assert_eq!(config.default_cooldown_seconds, 0);
}

#[test]
fn rejects_threshold_over_ten_thousand_bps() {
    let (mut svm, admin) = setup();
    let res = send(
        &mut svm,
        ix_initialize_config(admin.pubkey(), 10_001, 86_400),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidThreshold);
}

#[test]
fn rejects_negative_cooldown() {
    let (mut svm, admin) = setup();
    let res = send(
        &mut svm,
        ix_initialize_config(admin.pubkey(), 7_000, -1),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidCooldown);
}

#[test]
fn rejects_double_initialization() {
    // Second init hits "account already in use" from the system program,
    // not one of our error codes, so we just check the tx fails.
    let (mut svm, admin) = setup();
    send_ok(
        &mut svm,
        ix_initialize_config(admin.pubkey(), 7_000, 86_400),
        &[&admin],
    );
    assert!(send(
        &mut svm,
        ix_initialize_config(admin.pubkey(), 8_000, 43_200),
        &[&admin],
    )
    .is_err());
}
