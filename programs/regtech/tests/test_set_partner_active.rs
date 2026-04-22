mod common;

use {common::*, regtech::error::RegtechError};

#[test]
fn admin_can_deactivate_and_reactivate() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        ..
    } = register_partner_fixture();

    send_ok(
        &mut svm,
        ix_set_partner_active(admin.pubkey(), partner_id, false, 0),
        &[&admin],
    );
    assert!(!read_partner(&svm, &partner_id).active);

    send_ok(
        &mut svm,
        ix_set_partner_active(admin.pubkey(), partner_id, true, 0),
        &[&admin],
    );
    assert!(read_partner(&svm, &partner_id).active);
}

#[test]
fn non_admin_caller_rejected() {
    let PartnerFixture {
        mut svm,
        partner_id,
        partner_admin,
        ..
    } = register_partner_fixture();

    // Even the partner's own admin can't deactivate the partner.
    // Only the super-admin controls partner activation state.
    let res = send(
        &mut svm,
        ix_set_partner_active(partner_admin.pubkey(), partner_id, false, 0),
        &[&partner_admin],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn deactivation_blocks_register_module() {
    let PartnerFixture {
        mut svm,
        admin,
        partner_id,
        partner_admin,
        ..
    } = register_partner_fixture();

    send_ok(
        &mut svm,
        ix_set_partner_active(admin.pubkey(), partner_id, false, 0),
        &[&admin],
    );

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
fn deactivation_blocks_start_attempt() {
    let ModuleFixture {
        mut svm,
        admin,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    // Enroll while still active, then deactivate. Otherwise deactivation
    // prevents enrollment, and we want to test the start_attempt gate here.
    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);

    send_ok(
        &mut svm,
        ix_set_partner_active(admin.pubkey(), partner_id, false, 0),
        &[&admin],
    );

    let res = send(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
    expect_regtech_error(res, RegtechError::PartnerInactive);
}

#[test]
fn deactivation_blocks_submit_attempt() {
    let ModuleFixture {
        mut svm,
        admin,
        attestor,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    // Enroll and start the attempt while the partner is still active.
    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
    send_ok(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );

    // Now the admin cuts the partner off.
    send_ok(
        &mut svm,
        ix_set_partner_active(admin.pubkey(), partner_id, false, 0),
        &[&admin],
    );

    // In-flight attempts can't be scored.
    let res = send(
        &mut svm,
        ix_submit_attempt(attestor.pubkey(), user.pubkey(), partner_id, module_id_hash, 9_000),
        &[&attestor],
    );
    expect_regtech_error(res, RegtechError::PartnerInactive);
}

#[test]
fn reactivation_restores_normal_operation() {
    let ModuleFixture {
        mut svm,
        admin,
        partner_admin,
        partner_id,
        module_id_hash,
        ..
    } = register_module_fixture();

    send_ok(
        &mut svm,
        ix_set_partner_active(admin.pubkey(), partner_id, false, 0),
        &[&admin],
    );
    send_ok(
        &mut svm,
        ix_set_partner_active(admin.pubkey(), partner_id, true, 0),
        &[&admin],
    );

    let user = enrolled_user(&mut svm, &partner_admin, partner_id, module_id_hash);
    send_ok(
        &mut svm,
        ix_start_attempt(user.pubkey(), partner_id, module_id_hash),
        &[&user],
    );
}
