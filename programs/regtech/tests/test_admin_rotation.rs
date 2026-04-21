mod common;

use {
    anchor_lang::solana_program::pubkey::Pubkey,
    common::*,
    regtech::error::RegtechError,
    solana_keypair::Keypair,
};

#[test]
fn propose_then_accept_transfers_admin() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let new_admin = Keypair::new();
    fund(&mut svm, &new_admin.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), new_admin.pubkey()),
        &[&admin],
    );

    // After propose, pending is set but admin hasn't changed yet.
    let after_propose = read_config(&svm);
    assert_eq!(after_propose.admin, admin.pubkey());
    assert_eq!(after_propose.pending_admin, Some(new_admin.pubkey()));

    send_ok(&mut svm, ix_accept_admin_update(new_admin.pubkey()), &[&new_admin]);

    let after_accept = read_config(&svm);
    assert_eq!(after_accept.admin, new_admin.pubkey());
    assert_eq!(after_accept.pending_admin, None);
}

#[test]
fn old_admin_loses_authority_after_accept() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let new_admin = Keypair::new();
    fund(&mut svm, &new_admin.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), new_admin.pubkey()),
        &[&admin],
    );
    send_ok(&mut svm, ix_accept_admin_update(new_admin.pubkey()), &[&new_admin]);

    let res = send(&mut svm, ix_set_paused(admin.pubkey(), true, 0), &[&admin]);
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn new_admin_can_use_admin_powers_after_accept() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let new_admin = Keypair::new();
    fund(&mut svm, &new_admin.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), new_admin.pubkey()),
        &[&admin],
    );
    send_ok(&mut svm, ix_accept_admin_update(new_admin.pubkey()), &[&new_admin]);

    send_ok(
        &mut svm,
        ix_set_paused(new_admin.pubkey(), true, 0),
        &[&new_admin],
    );
    assert!(read_config(&svm).paused);
}

#[test]
fn pending_proposal_does_not_grant_authority() {
    // The whole point of two-step: proposing alone doesn't transfer power.
    // The candidate has to actively claim it by signing accept.
    let PlatformFixture { mut svm, admin } = init_platform();
    let candidate = Keypair::new();
    fund(&mut svm, &candidate.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), candidate.pubkey()),
        &[&admin],
    );

    // Candidate can't use admin powers until they accept.
    let res = send(
        &mut svm,
        ix_set_paused(candidate.pubkey(), true, 0),
        &[&candidate],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);

    // Old admin still has authority.
    send_ok(&mut svm, ix_set_paused(admin.pubkey(), true, 0), &[&admin]);
    assert!(read_config(&svm).paused);
}

#[test]
fn non_admin_cannot_propose() {
    let PlatformFixture { mut svm, admin: _ } = init_platform();
    let imposter = Keypair::new();
    let would_be_admin = Keypair::new();
    fund(&mut svm, &imposter.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_propose_admin_update(imposter.pubkey(), would_be_admin.pubkey()),
        &[&imposter],
    );
    expect_regtech_error(res, RegtechError::NotAuthorized);
}

#[test]
fn wrong_key_cannot_accept_someone_elses_proposal() {
    // Admin proposes A. B tries to accept. Should fail, even though B is a
    // valid signer: the pending_admin check confirms B isn't the candidate.
    let PlatformFixture { mut svm, admin } = init_platform();
    let candidate = Keypair::new();
    let interloper = Keypair::new();
    fund(&mut svm, &candidate.pubkey(), 1_000_000_000);
    fund(&mut svm, &interloper.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), candidate.pubkey()),
        &[&admin],
    );

    let res = send(
        &mut svm,
        ix_accept_admin_update(interloper.pubkey()),
        &[&interloper],
    );
    expect_regtech_error(res, RegtechError::PendingAdminMismatch);
}

#[test]
fn accept_with_no_pending_proposal_fails() {
    let PlatformFixture { mut svm, admin: _ } = init_platform();
    let claimant = Keypair::new();
    fund(&mut svm, &claimant.pubkey(), 1_000_000_000);

    let res = send(
        &mut svm,
        ix_accept_admin_update(claimant.pubkey()),
        &[&claimant],
    );
    expect_regtech_error(res, RegtechError::NoPendingAdmin);
}

#[test]
fn accept_twice_fails() {
    let PlatformFixture { mut svm, admin } = init_platform();
    let new_admin = Keypair::new();
    fund(&mut svm, &new_admin.pubkey(), 1_000_000_000);

    send_ok(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), new_admin.pubkey()),
        &[&admin],
    );
    send_ok(&mut svm, ix_accept_admin_update(new_admin.pubkey()), &[&new_admin]);

    // Force a fresh blockhash so the second accept tx doesn't come back as
    // AlreadyProcessed (identical signature) before the instruction runs.
    advance_blockhash(&mut svm);

    // Pending is cleared, so a second accept has nothing to accept.
    let res = send(
        &mut svm,
        ix_accept_admin_update(new_admin.pubkey()),
        &[&new_admin],
    );
    expect_regtech_error(res, RegtechError::NoPendingAdmin);
}

#[test]
fn proposing_to_zero_pubkey_rejected() {
    let PlatformFixture { mut svm, admin } = init_platform();

    let res = send(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), Pubkey::default()),
        &[&admin],
    );
    expect_regtech_error(res, RegtechError::InvalidPubkey);
}

#[test]
fn admin_can_overwrite_pending_proposal() {
    // Admin proposes A, then proposes B. Final pending should be B.
    // Useful if admin typos the first proposal: they can just propose again.
    let PlatformFixture { mut svm, admin } = init_platform();
    let candidate_a = Keypair::new();
    let candidate_b = Keypair::new();

    send_ok(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), candidate_a.pubkey()),
        &[&admin],
    );
    send_ok(
        &mut svm,
        ix_propose_admin_update(admin.pubkey(), candidate_b.pubkey()),
        &[&admin],
    );

    assert_eq!(read_config(&svm).pending_admin, Some(candidate_b.pubkey()));

    // A can no longer accept since they're no longer the pending candidate.
    fund(&mut svm, &candidate_a.pubkey(), 1_000_000_000);
    let res = send(
        &mut svm,
        ix_accept_admin_update(candidate_a.pubkey()),
        &[&candidate_a],
    );
    expect_regtech_error(res, RegtechError::PendingAdminMismatch);
}
