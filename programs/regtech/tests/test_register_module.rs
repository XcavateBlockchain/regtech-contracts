mod common;

use {common::*, regtech::error::RegtechError, solana_keypair::Keypair};

#[test]
fn happy_path_snapshots_defaults_from_partner() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let module_code = "mifid-appr-v1".to_string();
    let module_id_hash = code_hash(&module_code);

    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code.clone(),
            "ipfs://QmFake".to_string(),
            None,
            None,
            Some(31_536_000),
        ),
        &[&partner_admin],
    );

    let m = read_module(&svm, &partner_id, &module_id_hash);
    assert_eq!(m.partner_id, partner_id);
    assert_eq!(m.module_id_hash, module_id_hash);
    assert_eq!(m.module_code, module_code);
    assert_eq!(m.metadata_uri, "ipfs://QmFake");
    assert_eq!(m.pass_threshold_bps, 7_000, "snapshotted from partner");
    assert_eq!(m.cooldown_seconds, 86_400, "snapshotted from partner");
    assert_eq!(m.expires_in_seconds, Some(31_536_000));
    assert!(m.active);
}

#[test]
fn explicit_overrides_win_over_partner_defaults() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let module_code = "custom".to_string();
    let module_id_hash = code_hash(&module_code);

    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code,
            "ipfs://x".to_string(),
            Some(9_500),
            Some(300),
            None,
        ),
        &[&partner_admin],
    );

    let m = read_module(&svm, &partner_id, &module_id_hash);
    assert_eq!(m.pass_threshold_bps, 9_500);
    assert_eq!(m.cooldown_seconds, 300);
    assert_eq!(m.expires_in_seconds, None, "no expiry = never expires");
}

#[test]
fn hash_mismatch_rejected() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let real_code = "foo".to_string();
    let wrong_hash = code_hash("bar");

    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            wrong_hash,
            real_code,
            "uri".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::ModuleHashMismatch);
}

#[test]
fn non_partner_admin_caller_rejected() {
    let PartnerFixture {
        mut svm,
        partner_id,
        ..
    } = register_partner_fixture();

    let imposter = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let module_code = "x".to_string();
    let module_id_hash = code_hash(&module_code);

    let res = send(
        &mut svm,
        ix_register_module(
            imposter.pubkey(),
            partner_id,
            module_id_hash,
            module_code,
            "uri".to_string(),
            None,
            None,
            None,
        ),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn module_code_too_long_rejected() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let long_code = "a".repeat(65);
    let module_id_hash = code_hash(&long_code);

    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            long_code,
            "uri".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::StringTooLong);
}

#[test]
fn metadata_uri_too_long_rejected() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let module_code = "x".to_string();
    let module_id_hash = code_hash(&module_code);
    let long_uri = "a".repeat(257);

    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code,
            long_uri,
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::StringTooLong);
}

#[test]
fn threshold_override_over_bounds_rejected() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let module_code = "x".to_string();
    let module_id_hash = code_hash(&module_code);

    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code,
            "uri".to_string(),
            Some(10_001),
            None,
            None,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::InvalidThreshold);
}

#[test]
fn negative_cooldown_override_rejected() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let module_code = "x".to_string();
    let module_id_hash = code_hash(&module_code);

    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code,
            "uri".to_string(),
            None,
            Some(-1),
            None,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::InvalidCooldown);
}

#[test]
fn zero_expiry_rejected() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let module_code = "x".to_string();
    let module_id_hash = code_hash(&module_code);

    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code,
            "uri".to_string(),
            None,
            None,
            Some(0),
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::InvalidExpiry);
}

#[test]
fn negative_expiry_rejected() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let module_code = "x".to_string();
    let module_id_hash = code_hash(&module_code);

    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code,
            "uri".to_string(),
            None,
            None,
            Some(-1),
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::InvalidExpiry);
}

#[test]
fn rejects_when_paused() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();
    set_config_paused(&mut svm, true);

    let code = "x".to_string();
    let hash = code_hash(&code);
    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            hash,
            code,
            "uri".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::Paused);
}

#[test]
fn rejects_when_partner_inactive() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();
    set_partner_active(&mut svm, &partner_id, false);

    let code = "x".to_string();
    let hash = code_hash(&code);
    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            hash,
            code,
            "uri".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::PartnerInactive);
}

#[test]
fn duplicate_module_hash_rejected() {
    let PartnerFixture {
        mut svm,
        partner_admin,
        partner_id,
        ..
    } = register_partner_fixture();

    let module_code = "x".to_string();
    let module_id_hash = code_hash(&module_code);

    send_ok(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code.clone(),
            "uri".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );

    let res = send(
        &mut svm,
        ix_register_module(
            partner_admin.pubkey(),
            partner_id,
            module_id_hash,
            module_code,
            "uri2".to_string(),
            None,
            None,
            None,
        ),
        &[&partner_admin],
    );
    assert!(res.is_err(), "duplicate module hash should fail init constraint");
}
